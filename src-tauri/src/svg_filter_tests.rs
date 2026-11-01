use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations, svg_edit, svg_filter, vector,
};
use serde_json::json;
fn block(body: &str) -> Shape {
    serde_json::from_value(json!({"id":5,"type":"svg","source":format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180">{body}</svg>"#),"intrinsicW":240,"intrinsicH":180,"x":0,"y":0,"w":240,"h":180,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
}
fn flat(shapes: &[Shape]) -> Vec<&Shape> {
    shapes
        .iter()
        .flat_map(|s| {
            std::iter::once(s).chain(if let ShapeKind::Group { children, .. } = &s.kind {
                flat(children)
            } else {
                vec![]
            })
        })
        .collect()
}
fn compare(s: &Shape) -> Vec<Shape> {
    let out = svg_edit::decompose(s).unwrap();
    groups::validate(&out).unwrap();
    let a = vector::rasterize(&[s.clone()], 300, 230).unwrap();
    let b = vector::rasterize(&out, 300, 230).unwrap();
    let (mut max, mut sum) = (0f64, 0f64);
    for (p, q) in a.pixels().zip(b.pixels()) {
        for c in 0..4 {
            let d = if c == 3 {
                p[c].abs_diff(q[c]) as f64
            } else {
                (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
            };
            max = max.max(d);
            sum += d;
        }
    }
    let mean = sum / a.as_raw().len() as f64;
    if max > 12. || mean >= 0.15 {
        std::fs::create_dir_all("../work/filter-debug").unwrap();
        a.save("../work/filter-debug/before.png").unwrap();
        b.save("../work/filter-debug/after.png").unwrap();
        std::fs::write(
            "../work/filter-debug/after.svg",
            vector::svg_document(
                300,
                230,
                &out.iter().map(vector::shape_svg).collect::<String>(),
            ),
        )
        .unwrap();
        std::fs::write(
            "../work/filter-debug/shapes.json",
            serde_json::to_vec_pretty(&out).unwrap(),
        )
        .unwrap();
    }
    assert!(
        max <= 12. && mean < 0.15,
        "max {max}, mean {mean}, {:?}",
        s.kind
    );
    out
}
fn source(effect: &str) -> Shape {
    block(&format!(
        r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="5" y="5" width="220" height="160">{effect}</filter></defs><g transform="translate(6 4) rotate(7 110 80) skewX(5)" filter="url(#f)" opacity=".8"><rect x="30" y="30" width="120" height="100" fill="#ee6644"/><circle cx="150" cy="95" r="35" fill="#3366dd"/></g>"##
    ))
}
#[test]
fn sixteen_filter_families_preserve_independent_svg_pixels_and_affine_frames() {
    for effect in [
        r#"<feGaussianBlur stdDeviation="2 4"/>"#,
        r##"<feDropShadow dx="9" dy="-4" stdDeviation="3 1" flood-color="#224488" flood-opacity=".7"/>"##,
        r#"<feOffset dx="15" dy="-5"/>"#,
        r#"<feColorMatrix type="hueRotate" values="75"/>"#,
        r#"<feComponentTransfer><feFuncR type="gamma" amplitude=".8" exponent="2" offset=".1"/><feFuncG type="table" tableValues="0 .4 1"/><feFuncB type="linear" slope=".5" intercept=".2"/><feFuncA type="identity"/></feComponentTransfer>"#,
        r#"<feBlend in="SourceGraphic" in2="SourceAlpha" mode="screen"/>"#,
        r#"<feComposite in="SourceGraphic" in2="SourceAlpha" operator="arithmetic" k1=".5" k2=".3" k3=".2" k4=".05"/>"#,
        r#"<feOffset dx="15" result="o"/><feMerge><feMergeNode in="o"/><feMergeNode in="SourceGraphic"/></feMerge>"#,
        r##"<feFlood flood-color="#22aa88" flood-opacity=".6"/>"##,
        r#"<feMorphology operator="dilate" radius="3 1"/>"#,
        r#"<feConvolveMatrix order="3" kernelMatrix="0 -1 0 -1 5 -1 0 -1 0" preserveAlpha="true" edgeMode="duplicate"/>"#,
        r#"<feTurbulence baseFrequency=".04 .08" seed="4" numOctaves="2" result="noise"/><feDisplacementMap in="SourceGraphic" in2="noise" scale="12" xChannelSelector="R" yChannelSelector="G"/>"#,
        r#"<feTurbulence type="fractalNoise" baseFrequency=".04 .08" seed="4" numOctaves="2" stitchTiles="stitch"/>"#,
        r#"<feOffset in="SourceGraphic" x="25" y="25" width="50" height="50" result="tile"/><feTile in="tile"/>"#,
        r##"<feGaussianBlur in="SourceAlpha" stdDeviation="2" result="height"/><feDiffuseLighting in="height" surfaceScale="4" diffuseConstant=".8" lighting-color="#ff8844"><fePointLight x="80" y="10" z="100"/></feDiffuseLighting>"##,
        r##"<feGaussianBlur in="SourceAlpha" stdDeviation="2" result="height"/><feSpecularLighting in="height" surfaceScale="4" specularConstant=".8" specularExponent="12" lighting-color="#ff8844"><feSpotLight x="80" y="10" z="100" pointsAtX="100" pointsAtY="80" pointsAtZ="0" limitingConeAngle="80"/></feSpecularLighting>"##,
    ] {
        let out = compare(&source(effect));
        assert!(flat(&out).iter().any(|s| s.filters.is_some()));
    }
    assert_eq!(svg_filter::catalog().len(), 17);
}
#[test]
fn filter_chains_object_units_nested_clips_and_filters_inside_masks_survive() {
    for body in [
        r##"<g style="filter:blur(2px)" transform="rotate(7 100 80)" opacity=".7"><rect x="20" y="20" width="150" height="120" fill="#dd5533"/></g>"##,
        r##"<defs><filter id="f" x="-.2" y="-.3" width="1.4" height="1.6" primitiveUnits="objectBoundingBox"><feGaussianBlur stdDeviation=".025 .05"/><feOffset dx=".05" dy=".03"/></filter></defs><g transform="translate(6 8) scale(1.1 .85) rotate(8 100 80)" filter="url(#f)"><rect x="30" y="25" width="140" height="110" fill="#ee5533"/></g>"##,
        r##"<defs><filter id="f"><feGaussianBlur stdDeviation="2"/></filter><filter id="g"><feColorMatrix type="saturate" values=".2"/></filter><clipPath id="c"><circle cx="100" cy="80" r="65"/></clipPath><mask id="m"><rect width="240" height="180" fill="white"/><circle filter="url(#f)" cx="100" cy="80" r="30" fill="black"/></mask></defs><g mask="url(#m)" clip-path="url(#c)" filter="url(#f) url(#g)" opacity=".7"><rect x="20" y="20" width="180" height="130" fill="#ee8833"/><g filter="url(#g)"><circle cx="130" cy="80" r="50" fill="#3355dd"/></g></g>"##,
    ] {
        compare(&block(body));
    }
}
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
#[test]
fn editable_filters_undo_v13_and_invalid_graph_changes_are_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("filter.svg");
    let ShapeKind::Svg { source, .. } =
        source(r#"<feGaussianBlur stdDeviation="3" result="b"/><feOffset in="b" dx="7" dy="4"/>"#)
            .kind
    else {
        panic!()
    };
    std::fs::write(&p, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":p})).unwrap();
    let id = e.doc().unwrap().layers[0].id;
    let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
        panic!()
    };
    let mut s = (*flat(shapes)
        .into_iter()
        .find(|s| s.filters.is_some())
        .unwrap())
    .clone();
    let original = io::project_bytes(e.doc().unwrap()).unwrap();
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    s.filters.as_mut().unwrap().filters[0].primitives[0]
        .attributes
        .insert("stdDeviation".into(), "7 2".into());
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    assert_ne!(pixels, vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(pixels, vector::flatten(e.doc().unwrap()).unwrap());
    assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
    let hist = e.info().history;
    for (key, value) in [
        ("in", "future"),
        ("stdDeviation", "NaN"),
        ("stdDeviation", "-1"),
        ("stdDeviation", "900"),
        ("result", "SourceGraphic"),
        ("onclick", "evil"),
    ] {
        let mut bad = s.clone();
        bad.filters.as_mut().unwrap().filters[0].primitives[0]
            .attributes
            .insert(key.into(), value.into());
        assert!(run(&mut e, json!({"type":"shape","id":id,"shape":bad})).is_err());
        assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
        assert_eq!(hist, e.info().history);
    }
    let project = tmp.path().join("filter.prisme");
    std::fs::write(&project, &original).unwrap();
    let restored = io::read_project(&original).unwrap();
    assert_eq!(pixels, vector::flatten(&restored).unwrap());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&original)).unwrap();
    let meta: serde_json::Value =
        serde_json::from_reader(zip.by_name("document.json").unwrap()).unwrap();
    assert_eq!(meta["version"], crate::io::PROJECT_VERSION);
    assert!(run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":s.id})
    )
    .is_err());
}
#[test]
fn unsupported_or_eliminated_filters_preserve_all_source_content() {
    for effect in [
        r##"<feImage href="#missing"/>"##,
        r#"<feUnknown/>"#,
        r#"<feGaussianBlur stdDeviation="2"/>"#,
    ] {
        let region = if effect.contains("Gaussian") {
            r#"width="0""#
        } else {
            ""
        };
        let s = block(&format!(
            r##"<defs><filter id="f" {region}>{effect}</filter></defs><rect width="20" height="20" fill="blue"/><g filter="url(#f)"><rect x="30" y="30" width="120" height="100" fill="red"/></g>"##
        ));
        assert!(svg_edit::decompose(&s).unwrap_err().contains("conservé"));
    }
    let s = block(
        r##"<defs><filter id="unused"><feImage href="#missing"/></filter></defs><rect x="20" y="20" width="100" height="100" fill="red"/>"##,
    );
    compare(&s);
}
#[test]
fn native_picking_includes_filtered_pixels_outside_source_geometry() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("offset.svg");
    let ShapeKind::Svg { source, .. } = source(r#"<feOffset dx="35" dy="20"/>"#).kind else {
        panic!()
    };
    std::fs::write(&p, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":p})).unwrap();
    let layer = &e.doc().unwrap().layers[0];
    let Content::Vector(shapes) = &layer.content else {
        panic!()
    };
    let s = flat(shapes)
        .into_iter()
        .find(|s| s.filters.is_some())
        .unwrap();
    let rendered = vector::rasterize(shapes, 240, 180).unwrap();
    for y in (5..175).step_by(13) {
        for x in (5..235).step_by(13) {
            let hit =
                crate::svg_mask::pick(layer, &[s.id], [x as f32 + 0.1, y as f32 + 0.1]).unwrap();
            assert_eq!(hit.is_some(), rendered.get_pixel(x, y)[3] > 0, "{x},{y}");
        }
    }
}
#[test]
fn filter_preview_is_bounded_and_graph_limits_are_enforced() {
    let out = svg_edit::decompose(&source(r#"<feGaussianBlur stdDeviation="3"/>"#)).unwrap();
    let s = flat(&out)
        .into_iter()
        .find(|s| s.filters.is_some())
        .unwrap();
    let image = svg_filter::preview(s).unwrap();
    assert!(image.width() <= 224 && image.height() <= 224);
    assert!(image.pixels().any(|p| p[3] > 0));
    let good = s.filters.as_ref().unwrap();
    for (key, value) in [("numOctaves", "100"), ("baseFrequency", "-1")] {
        let mut f = good.clone();
        f.filters[0].primitives[0] = svg_filter::catalog()
            .into_iter()
            .find(|p| p.kind == "feTurbulence")
            .unwrap();
        f.filters[0].primitives[0]
            .attributes
            .insert(key.into(), value.into());
        assert!(f.validate().is_err());
    }
    let mut bad = good.clone();
    bad.filters[0].primitives = vec![bad.filters[0].primitives[0].clone(); 65];
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.filters[0].region[2] = 0.;
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.transform = [0.; 6];
    assert!(bad.validate().is_err());
}

#[test]
fn pdf_filter_copy_preserves_affine_blending_opacity_and_unfiltered_vectors() {
    for editable in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("pdf-filter.svg");
        let s = block(
            r##"<defs><filter id="f"><feDropShadow dx="9" dy="5" stdDeviation="3 1" flood-color="#224477" flood-opacity=".6"/></filter></defs><rect width="240" height="180" fill="#ffeecc"/><g filter="url(#f)" transform="rotate(8 110 80)" opacity=".7" style="mix-blend-mode:multiply"><rect x="30" y="30" width="130" height="110" fill="#cc6633"/><circle cx="140" cy="100" r="40" fill="#3366cc"/></g><circle cx="210" cy="150" r="12" fill="#22aa55"/>"##,
        );
        let ShapeKind::Svg { source, .. } = s.kind else {
            panic!()
        };
        std::fs::write(&p, source).unwrap();
        let mut e = Editor::default();
        run(
            &mut e,
            json!({"type":if editable{"openSvgEditable"}else{"open"},"path":p}),
        )
        .unwrap();
        let doc = e.doc_mut().unwrap();
        doc.width = 320;
        doc.height = 240;
        doc.layers[0].x = 7;
        doc.layers[0].y = -4;
        let Content::Vector(shapes) = &mut doc.layers[0].content else {
            panic!()
        };
        let mut parent = svg_edit::wrap_group(shapes.clone(), 1., None).unwrap();
        groups::apply(&mut parent, [1.05, 0.08, 0.12, 0.95, 10., 8.]);
        *shapes = vec![parent];
        let mut n = 0;
        groups::assign_ids(shapes, &mut || {
            n += 1;
            n
        });
        let original = io::project_bytes(doc).unwrap();
        let a = vector::flatten(doc).unwrap();
        let prepared = crate::vector_export::prepare_pdf_images(doc).unwrap();
        let b = vector::flatten(&prepared).unwrap();
        let (mut max, mut sum) = (0f64, 0f64);
        for (p, q) in a.pixels().zip(b.pixels()) {
            for c in 0..4 {
                let d = if c == 3 {
                    p[c].abs_diff(q[c]) as f64
                } else {
                    (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
                };
                max = max.max(d);
                sum += d;
            }
        }
        let mean = sum / a.as_raw().len() as f64;
        let bad = a
            .pixels()
            .zip(b.pixels())
            .filter(|(p, q)| p.0.iter().zip(q.0).any(|(a, b)| a.abs_diff(b) > 8))
            .count();
        let mut constant_max = 0;
        for y in 1..a.height() - 1 {
            for x in 1..a.width() - 1 {
                let p = a.get_pixel(x, y);
                if (-1..=1).all(|dy| {
                    (-1..=1)
                        .all(|dx| a.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32) == p)
                }) {
                    let q = b.get_pixel(x, y);
                    for c in 0..4 {
                        constant_max = constant_max.max(p[c].abs_diff(q[c]));
                    }
                }
            }
        }
        eprintln!("PDF_COPY editable {editable} max {max} mean {mean} pixelsOver8 {bad} constantMax {constant_max}");
        assert!(bad < 10, "{bad} pixels over 8");
        assert!(constant_max <= 2, "constant interior {constant_max}");
        assert!(
            max <= 16. && mean < 0.15,
            "editable {editable}, max {max}, mean {mean}"
        );
        assert_eq!(original, io::project_bytes(doc).unwrap());
        assert!(crate::vector_export::pdf(doc).unwrap().starts_with(b"%PDF"));
        let Content::Vector(shapes) = &prepared.layers[0].content else {
            panic!()
        };
        assert!(flat(shapes)
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Path { .. })));
        assert!(flat(shapes)
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Image { .. })));
        assert!(!flat(shapes).iter().any(|s| s.filters.is_some()));
    }
}
