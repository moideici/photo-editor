use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    group_style, groups, io, operations, svg_edit, vector,
};
use serde_json::json;
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn block(body: &str) -> Shape {
    serde_json::from_value(json!({"id":5,"type":"svg","source":format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180">{body}</svg>"#),"intrinsicW":240,"intrinsicH":180,"x":0,"y":0,"w":240,"h":180,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
}
fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut sum = 0.;
    let mut max = 0f64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        for c in 0..4 {
            let d = if c == 3 {
                (p[3] as f64 - q[3] as f64).abs()
            } else {
                (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
            };
            sum += d;
            max = max.max(d);
        }
    }
    assert!(
        max <= 4. && sum / (a.as_raw().len() as f64) < 0.15,
        "max {max}, mean {}",
        sum / a.as_raw().len() as f64
    );
}
fn flat(shapes: &[Shape]) -> Vec<&Shape> {
    shapes
        .iter()
        .flat_map(|s| {
            std::iter::once(s).chain(if let ShapeKind::Group { children, .. } = &s.kind {
                flat(children)
            } else {
                Vec::new()
            })
        })
        .collect()
}
fn fixture() -> Shape {
    block(
        r##"<rect x="5" y="5" width="225" height="165" fill="#aaccee"/><g opacity=".6" transform="translate(10 8) rotate(6 100 75)"><rect x="20" y="20" width="100" height="90" fill="#ee4422"/><g style="mix-blend-mode:multiply;isolation:isolate" opacity=".75"><ellipse cx="105" cy="85" rx="65" ry="45" fill="#22aa66"/><path d="M60 50H190V130H60Z" fill="#2244cc" fill-opacity=".7"/></g></g>"##,
    )
}
#[test]
fn all_svg_blends_opacity_and_explicit_isolation_preserve_independent_render() {
    for mode in group_style::BLENDS {
        for isolate in [false, true] {
            let body = format!(
                r##"<rect x="5" y="5" width="225" height="165" fill="#aaccee"/><g transform="translate(5 4)"><rect x="10" y="10" width="110" height="110" fill="#eeaa22"/><g opacity=".65" style="mix-blend-mode:{mode};isolation:{}"><rect x="30" y="30" width="110" height="100" fill="#cc3355"/><g style="mix-blend-mode:screen"><rect x="70" y="60" width="120" height="90" fill="#2255aa"/></g></g></g>"##,
                if isolate { "isolate" } else { "auto" }
            );
            let s = block(&body);
            let out = svg_edit::decompose(&s).unwrap();
            groups::validate(&out).unwrap();
            assert!(flat(&out).iter().any(|s| s
                .group_style
                .as_ref()
                .is_some_and(|s| s.blend == mode && s.isolate == isolate)));
            compare(
                &vector::rasterize(&[s], 240, 180).unwrap(),
                &vector::rasterize(&out, 240, 180).unwrap(),
            );
        }
    }
    let mut isolated = fixture();
    let ShapeKind::Svg { source, .. } = &mut isolated.kind else {
        panic!()
    };
    let normal = source
        .replace("isolation:isolate", "isolation:auto")
        .replace("mix-blend-mode:multiply;", "mix-blend-mode:normal;");
    assert_ne!(
        vector::rasterize(&[isolated], 240, 180).unwrap(),
        vector::rasterize(
            &[block(
                normal
                    .split_once('>')
                    .unwrap()
                    .1
                    .rsplit_once("</svg>")
                    .unwrap()
                    .0
            )],
            240,
            180
        )
        .unwrap()
    );
}
#[test]
fn svg_block_opacity_and_affine_placement_wrap_overlapping_surfaces_once() {
    let mut s = block(
        r##"<rect x="20" y="20" width="110" height="100" fill="#ff0000"/><rect x="70" y="45" width="120" height="100" fill="#0040ff"/>"##,
    );
    s.opacity = 0.5;
    let out = svg_edit::decompose(&s).unwrap();
    assert_eq!(out.len(), 1);
    assert!(matches!(out[0].kind, ShapeKind::Group { .. }));
    let before = vector::rasterize(&[s.clone()], 320, 250).unwrap();
    assert!((127..=128).contains(&before.get_pixel(90, 80)[3]));
    compare(&before, &vector::rasterize(&out, 320, 250).unwrap());
    s.x = 25.;
    s.y = 20.;
    s.w = 250.;
    s.h = 200.;
    s.rotation = 11.;
    s.transform = Some([0.9, 0.05, 0.1, 0.85, 7., 3.]);
    let out = svg_edit::decompose(&s).unwrap();
    compare(
        &vector::rasterize(&[s], 320, 250).unwrap(),
        &vector::rasterize(&out, 320, 250).unwrap(),
    );
}
#[test]
fn styled_import_nested_edit_history_project_and_recursive_id_assignment() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("groups.svg");
    let ShapeKind::Svg { source, .. } = fixture().kind else {
        panic!()
    };
    std::fs::write(&path, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":path})).unwrap();
    let layer = e.doc().unwrap().layers[0].id;
    let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
        panic!()
    };
    groups::validate(shapes).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let mut child = flat(shapes)
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Path { .. }))
        .unwrap()
        .clone();
    child.fill = Some("#ff00ff".into());
    let n = e.info().history.len();
    run(&mut e, json!({"type":"shape","id":layer,"shape":child})).unwrap();
    assert_eq!(e.info().history.len(), n + 1);
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"redo"})).unwrap();
    let edited = vector::flatten(e.doc().unwrap()).unwrap();
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let reopened = io::read_project(&bytes).unwrap();
    assert_eq!(edited, vector::flatten(&reopened).unwrap());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let json: serde_json::Value =
        serde_json::from_reader(zip.by_name("document.json").unwrap()).unwrap();
    assert_eq!(json["version"], crate::io::PROJECT_VERSION);
    run(&mut e, json!({"type":"duplicateLayer","id":layer})).unwrap();
    let saved = tmp.path().join("groups.prisme");
    run(&mut e, json!({"type":"save","path":saved})).unwrap();
    run(&mut e, json!({"type":"import","path":saved})).unwrap();
    let mut all = std::collections::HashSet::new();
    for l in &e.doc().unwrap().layers {
        if let Content::Vector(shapes) = &l.content {
            groups::validate(shapes).unwrap();
            for s in flat(shapes) {
                assert!(all.insert(s.id));
            }
        }
    }
}
#[test]
fn invalid_group_style_and_styled_ungroup_refuse_without_mutation() {
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":240,"height":180,"background":null}),
    )
    .unwrap();
    run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.active_hint.unwrap();
    let mut s = svg_edit::decompose(&fixture())
        .unwrap()
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Group { .. }))
        .unwrap();
    s.id = 0;
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
        panic!()
    };
    let mut s = shapes[0].clone();
    s.opacity = 1.;
    s.group_style = Some(group_style::GroupStyle {
        blend: "multiply".into(),
        isolate: true,
    });
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let hist = e.info().history;
    assert!(run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":s.id})
    )
    .unwrap_err()
    .contains("fusion"));
    assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(hist, e.info().history);
    let mut bad = s.clone();
    bad.group_style.as_mut().unwrap().blend = "invented".into();
    assert!(run(&mut e, json!({"type":"shape","id":id,"shape":bad})).is_err());
    assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
    s.group_style = None;
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    let neutral = vector::flatten(e.doc().unwrap()).unwrap();
    run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":s.id}),
    )
    .unwrap();
    compare(&neutral, &vector::flatten(e.doc().unwrap()).unwrap());
}
#[test]
fn complex_effects_still_refuse_atomically_after_an_editable_sibling() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("complex.svg");
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":240,"height":180,"background":"#ffaa00"}),
    )
    .unwrap();
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let hist = e.info().history;
    for extra in [
        r##"<defs><filter id="f"><feImage href="#unknown"/></filter></defs><g filter="url(#f)"><rect x="10" y="10" width="130" height="130"/></g>"##,
    ] {
        let ShapeKind::Svg { source, .. } = block(&format!(
            r##"<rect x="10" y="10" width="20" height="20" fill="red"/>{extra}"##
        ))
        .kind
        else {
            panic!()
        };
        std::fs::write(&path, source).unwrap();
        assert!(run(&mut e, json!({"type":"openSvgEditable","path":path}))
            .unwrap_err()
            .contains("conservé"));
        assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
        assert_eq!(hist, e.info().history);
    }
}

#[test]
fn group_blends_stay_inside_their_layer_in_svg_exports() {
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":240,"height":180,"background":"#ffaa22"}),
    )
    .unwrap();
    run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.active_hint.unwrap();
    let source = block(
        r##"<g style="mix-blend-mode:multiply"><rect x="20" y="20" width="150" height="120" fill="#2266cc"/></g>"##,
    );
    let mut s = svg_edit::decompose(&source).unwrap().remove(0);
    s.id = 0;
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    assert_eq!(before.get_pixel(60, 60).0, [34, 102, 204, 255]);
    let svg = vector::export_svg(e.doc().unwrap()).unwrap();
    compare(&before, &vector::render_svg(&svg, 240, 180).unwrap());
}
