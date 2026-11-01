use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations, svg_edit, vector,
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
    let a = vector::rasterize(&[s.clone()], 300, 220).unwrap();
    let b = vector::rasterize(&out, 300, 220).unwrap();
    let mut max = 0f64;
    let mut sum = 0f64;
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
    assert!(
        max <= 8. && mean < 0.1,
        "max {max}, mean {mean}; {:?}",
        s.kind
    );
    out
}
#[test]
fn masks_preserve_alpha_luminance_regions_units_transforms_and_group_compositing() {
    for mode in ["alpha", "luminance"] {
        for units in ["userSpaceOnUse", "objectBoundingBox"] {
            let (region, content) = if units == "userSpaceOnUse" {
                (
                    r#"x="15" y="15" width="200" height="145""#,
                    r#"x="25" y="25" width="175" height="120""#,
                )
            } else {
                (
                    r#"x="0" y="0" width="1" height="1""#,
                    r#"x=".1" y=".1" width=".8" height=".8""#,
                )
            };
            let s = block(&format!(
                r##"<defs><linearGradient id="p"><stop stop-color="white"/><stop offset="1" stop-color="#202040" stop-opacity=".2"/></linearGradient><mask id="m" maskUnits="{units}" maskContentUnits="{units}" {region} mask-type="{mode}"><rect {content} fill="url(#p)"/><circle cx="{}" cy="{}" r="{}" fill="#88ff33" opacity=".5"/></mask></defs><g transform="translate(8 5) rotate(6 110 80) skewX(5)" opacity=".7" mask="url(#m)"><rect x="20" y="20" width="180" height="120" fill="#ee4422"/><circle cx="140" cy="90" r="55" fill="#2255dd"/></g>"##,
                if units == "userSpaceOnUse" {
                    "90"
                } else {
                    ".5"
                },
                if units == "userSpaceOnUse" {
                    "70"
                } else {
                    ".5"
                },
                if units == "userSpaceOnUse" {
                    "30"
                } else {
                    ".25"
                }
            ));
            let out = compare(&s);
            assert!(flat(&out).iter().any(|s| s.mask.is_some()));
        }
    }
}
#[test]
fn mask_intersections_nested_masks_clips_painted_groups_and_empty_masks_survive() {
    for body in [
        r##"<defs><mask id="a" maskUnits="userSpaceOnUse" x="0" y="0" width="240" height="180"><circle cx="110" cy="85" r="70" fill="white"/></mask><mask id="b" mask="url(#a)" maskUnits="userSpaceOnUse" x="0" y="0" width="240" height="180"><rect x="25" y="20" width="120" height="130" fill="#aaaaaa"/></mask></defs><g mask="url(#b)"><rect width="230" height="170" fill="#22aa77"/></g>"##,
        r##"<defs><clipPath id="c"><circle cx="100" cy="70" r="60"/></clipPath><mask id="a"><rect width="240" height="180" fill="white"/><circle cx="110" cy="80" r="30" fill="black"/></mask><mask id="b"><g mask="url(#a)" clip-path="url(#c)" opacity=".7"><rect width="240" height="180" fill="white"/><g style="mix-blend-mode:multiply;isolation:isolate"><rect x="30" y="30" width="130" height="100" fill="#aaaaaa"/></g></g></mask></defs><g mask="url(#b)"><rect x="10" y="10" width="210" height="150" fill="#ee6633"/></g>"##,
        r##"<defs><pattern id="p" width="20" height="20" patternUnits="userSpaceOnUse"><rect width="10" height="20" fill="white"/></pattern><mask id="m"><rect width="240" height="180" fill="url(#p)"/></mask></defs><g mask="url(#m)"><rect x="20" y="20" width="180" height="120" fill="#3355dd"/></g>"##,
    ] {
        compare(&block(body));
    }
}
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
#[test]
fn editable_mask_changes_undo_project_v12_and_invalid_mutations_are_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("mask.svg");
    let s = block(
        r##"<defs><mask id="m"><rect x="20" y="20" width="180" height="120" fill="white"/><circle cx="100" cy="80" r="30" fill="black"/></mask></defs><g mask="url(#m)"><rect width="240" height="180" fill="#2266cc"/></g>"##,
    );
    let ShapeKind::Svg { source, .. } = &s.kind else {
        panic!()
    };
    std::fs::write(&p, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":p})).unwrap();
    let id = e.doc().unwrap().layers[0].id;
    let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
        panic!()
    };
    let mut g = flat(shapes)
        .into_iter()
        .find(|s| s.mask.is_some())
        .unwrap()
        .clone();
    let original = vector::flatten(e.doc().unwrap()).unwrap();
    let history = e.info().history_index;
    g.mask.as_mut().unwrap().children[1].fill = Some("#888888".into());
    run(&mut e, json!({"type":"shape","id":id,"shape":g})).unwrap();
    assert_eq!(e.info().history_index, history + 1);
    assert!(original != vector::flatten(e.doc().unwrap()).unwrap());
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let reopened = io::read_project(&bytes).unwrap();
    assert!(vector::flatten(&reopened).unwrap() == vector::flatten(e.doc().unwrap()).unwrap());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let metadata: serde_json::Value =
        serde_json::from_reader(zip.by_name("document.json").unwrap()).unwrap();
    assert_eq!(metadata["version"], crate::io::PROJECT_VERSION);
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert!(original == vector::flatten(e.doc().unwrap()).unwrap());
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let history = e.info().history_index;
    g.mask.as_mut().unwrap().region[2] = 0.;
    assert!(run(&mut e, json!({"type":"shape","id":id,"shape":g})).is_err());
    assert!(run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":g.id})
    )
    .unwrap_err()
    .contains("masque"));
    assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(history, e.info().history_index);
}
#[test]
fn images_inside_masks_keep_bytes_and_alpha_and_can_be_exported() {
    let image = image::RgbaImage::from_fn(8, 6, |x, y| {
        image::Rgba([
            if x < 4 { 255 } else { 100 },
            (y * 40) as u8,
            180,
            if x < 4 { 255 } else { 100 },
        ])
    });
    let asset = crate::embedded::ImageAsset::from_bytes(
        "image/png",
        &io::png_bytes(&image).unwrap(),
        "optimizeQuality",
    )
    .unwrap();
    let s = block(&format!(
        r##"<defs><mask id="m" mask-type="alpha"><image x="20" y="20" width="180" height="120" href="{}" preserveAspectRatio="none"/></mask></defs><rect width="240" height="180" fill="#2266cc" mask="url(#m)"/>"##,
        asset.url()
    ));
    let out = compare(&s);
    let mask = flat(&out)
        .into_iter()
        .find_map(|s| s.mask.as_ref())
        .unwrap();
    assert!(flat(&mask.children)
        .iter()
        .any(|s| matches!(&s.kind,ShapeKind::Image{asset:a} if a.data==asset.data)));
}
#[test]
fn native_selection_samples_visible_mask_pixels_and_ancestors_without_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("hit.svg");
    let ShapeKind::Svg{source,..}=block(r##"<defs><mask id="m"><rect width="240" height="180" fill="white"/><circle cx="110" cy="80" r="35" fill="black"/></mask></defs><rect width="240" height="180" fill="#aabbcc"/><g mask="url(#m)"><rect width="240" height="180" fill="#2255dd"/></g>"##).kind else{panic!()};
    std::fs::write(&p, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":p})).unwrap();
    let n = e.info().history_index;
    let layer = &e.doc().unwrap().layers[0];
    let Content::Vector(shapes) = &layer.content else {
        panic!()
    };
    let bg = shapes[0].id;
    let g = &shapes[1];
    let ShapeKind::Group { children, .. } = &g.kind else {
        panic!()
    };
    let leaf = children[0].id;
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let isolated = vector::rasterize(&[g.clone()], 240, 180).unwrap();
    for x in (10..230).step_by(20) {
        for y in (10..170).step_by(20) {
            let selected =
                crate::svg_mask::pick(layer, &[leaf, g.id, bg], [x as f32, y as f32]).unwrap();
            assert_eq!(
                selected,
                Some(if isolated.get_pixel(x, y)[3] > 0 {
                    leaf
                } else {
                    bg
                }),
                "{x},{y}"
            );
        }
    }
    assert!(crate::svg_mask::pick(layer, &[leaf, leaf], [10., 10.]).is_err());
    assert!(crate::svg_mask::pick(layer, &[9999999], [10., 10.]).is_err());
    let preview = g.mask.as_ref().unwrap().preview().unwrap();
    assert_eq!(preview.width(), 192);
    assert!(preview.pixels().any(|p| p[3] == 0));
    assert!(preview.pixels().any(|p| p[3] == 255));
    assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(n, e.info().history_index);
}
#[test]
fn optimized_away_masked_content_refuses_without_losing_visible_siblings() {
    let s = block(
        r##"<defs><mask id="empty"/></defs><rect width="50" height="50" fill="red"/><rect x="60" width="120" height="100" fill="blue" mask="url(#empty)"/>"##,
    );
    assert!(svg_edit::decompose(&s).unwrap_err().contains("entièrement"));
    let unused =
        block(r##"<defs><mask id="empty"/></defs><rect width="50" height="50" fill="red"/>"##);
    compare(&unused);
}
