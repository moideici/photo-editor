use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    fonts, groups, io, operations, text_path, text_styles, vector, vector_export,
};
use serde_json::{json, Value};
fn text() -> Shape {
    serde_json::from_value(json!({"id":7001,"type":"text","text":"HHHH","fontFamily":"Prisme Variable Test","fontSize":64,"bold":false,"italic":false,"x":40,"y":20,"w":153.6,"h":83.2,"rotation":0,"fill":"#245b9e","stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
}
fn opts(w: f32, o: f32) -> fonts::FontOptions {
    serde_json::from_value(json!({"axes":{"wght":w,"opsz":o},"opticalSizing":"none"})).unwrap()
}
fn cmd(e: &mut Editor, v: Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn setup() -> (Editor, u64) {
    let mut e = Editor::default();
    cmd(
        &mut e,
        json!({"type":"new","width":460,"height":160,"background":null}),
    )
    .unwrap();
    cmd(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.doc().unwrap().layers.last().unwrap().id;
    cmd(&mut e, json!({"type":"shape","id":id,"shape":text()})).unwrap();
    (e, id)
}
fn get(e: &Editor, id: u64) -> &Shape {
    let Content::Vector(ss) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    groups::find(ss, id).unwrap()
}
#[track_caller]
fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut max = 0f64;
    let mut sum = 0.;
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
    eprintln!("independent font outlines max={max} mean={mean}");
    assert!(max <= 12. && mean < 0.02, "max={max} mean={mean}");
}
#[test]
fn actual_fvar_catalog_and_missing_font_resolution() {
    let f = fonts::info("Prisme Variable Test", false, false);
    assert!(f.available);
    assert_eq!(f.axes.len(), 2);
    let a = f.axes.iter().find(|a| a.tag == "opsz").unwrap();
    assert_eq!((a.min, a.default, a.max), (5., 10.5, 36.));
    assert!(fonts::catalog().iter().any(|n| n == "Prisme Variable Test"));
    let absent = fonts::info("Prisme Font Missing 9181", false, false);
    assert!(!absent.available);
    assert!(absent.resolved.is_some());
    assert!(absent.axes.is_empty());
}
#[test]
fn independent_fonttools_glyphs_match_axes_and_optical_sizing() {
    let mut images = vec![];
    for (w, o, reference) in [
        (300., 5., include_str!("fixtures/variable-light.svg")),
        (700., 36., include_str!("fixtures/variable-heavy.svg")),
        (400., 10.5, include_str!("fixtures/variable-default.svg")),
    ] {
        let mut t = text();
        t.font_options = Some(opts(w, o));
        let native = vector::rasterize(&[t], 460, 160).unwrap();
        compare(&native, &vector::render_svg(reference, 460, 160).unwrap());
        images.push(native);
    }
    assert_ne!(images[0], images[1]);
    let mut t = text();
    t.font_options = Some(fonts::FontOptions::default());
    let native = vector::rasterize(&[t.clone()], 460, 160).unwrap();
    compare(
        &native,
        &vector::render_svg(include_str!("fixtures/variable-auto.svg"), 460, 160).unwrap(),
    );
    assert_ne!(native, images[2]);
    t.font_options = Some(opts(300., 5.));
    if let ShapeKind::Text { bold, .. } = &mut t.kind {
        *bold = true
    };
    compare(&vector::rasterize(&[t], 460, 160).unwrap(), &images[0]);
}
#[test]
fn corrupt_axes_rejected_and_foreign_ranges_preserved_with_notices() {
    let (mut e, id) = setup();
    let baseline = io::project_bytes(e.doc().unwrap()).unwrap();
    e.info(); // Consume the one-shot active layer hint before comparison.
    let history = serde_json::to_value(e.info()).unwrap();
    for axes in [
        json!({"bad":400}),
        json!({"wg't":400}),
        json!({"wght":1000001}),
    ] {
        let mut t = serde_json::to_value(text()).unwrap();
        t["fontOptions"] = json!({"axes":axes,"opticalSizing":"auto"});
        assert!(cmd(&mut e, json!({"type":"shape","id":id,"shape":t})).is_err());
        assert_eq!(baseline, io::project_bytes(e.doc().unwrap()).unwrap());
        assert_eq!(history, serde_json::to_value(e.info()).unwrap());
    }
    let mut t = text();
    t.font_options = Some(opts(999., 5.));
    assert!(fonts::validate(&t).is_ok());
    assert!(fonts::validate_controls(&t).is_err());
    cmd(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    assert_eq!(e.info().font_notices.len(), 1);
    let loaded = io::read_project(&io::project_bytes(e.doc().unwrap()).unwrap()).unwrap();
    let Content::Vector(ss) = &loaded.layers.last().unwrap().content else {
        panic!()
    };
    assert_eq!(ss[0].font_options.as_ref().unwrap().axes["wght"], 999.);
}
#[test]
fn styles_batch_apply_atomic_undo_and_snapshot_semantics() {
    let (mut e, id) = setup();
    let mut t = text();
    t.font_options = Some(opts(700., 36.));
    cmd(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    cmd(
        &mut e,
        json!({"type":"textStyleSave","id":id,"shapeId":7001,"name":"Titre"}),
    )
    .unwrap();
    let mut second = text();
    second.id = 7002;
    second.x = 160.;
    if let ShapeKind::Text {
        text, font_size, ..
    } = &mut second.kind
    {
        *text = "H H".into();
        *font_size = 32.
    };
    second.w /= 2.;
    second.h /= 2.;
    cmd(&mut e, json!({"type":"shape","id":id,"shape":second})).unwrap();
    let baseline = io::project_bytes(e.doc().unwrap()).unwrap();
    let index = e.info().history_index;
    assert!(cmd(
        &mut e,
        json!({"type":"textStyleApply","id":id,"shapeIds":[7002,999999],"name":"Titre"})
    )
    .is_err());
    assert_eq!(baseline, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(e.info().history_index, index);
    cmd(
        &mut e,
        json!({"type":"textStyleApply","id":id,"shapeIds":[7001,7002],"name":"Titre"}),
    )
    .unwrap();
    assert_eq!(e.info().history_index, index + 1);
    let applied = get(&e, 7002);
    assert_eq!(applied.x, 160.);
    assert!(
        matches!(&applied.kind,ShapeKind::Text{text,font_size,..} if text=="H H"&&*font_size==64.)
    );
    assert_eq!(applied.font_options.as_ref().unwrap().axes["wght"], 700.);
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    cmd(&mut e, json!({"type":"textStyleDelete","name":"Titre"})).unwrap();
    assert_eq!(pixels, vector::flatten(e.doc().unwrap()).unwrap());
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(baseline, io::project_bytes(e.doc().unwrap()).unwrap());
}
#[test]
fn style_copies_typography_preserves_link_own_transform_and_content() {
    let mut t = text();
    t.font_options = Some(opts(700., 36.));
    t.opacity = 0.7;
    let style = text_styles::TextStyle::capture("Titre", &t).unwrap();
    assert!(matches!(&style.template.kind,ShapeKind::Text{text,..} if text.is_empty()));
    let guide:Shape=serde_json::from_value(json!({"id":7000,"type":"line","x":20,"y":90,"w":400,"h":1,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1})).unwrap();
    let settings: text_path::Settings=serde_json::from_value(json!({"sourceId":7000,"contourIndex":0,"startOffset":0,"percent":false,"normalOffset":0,"anchor":"start","reverse":false})).unwrap();
    let mut curved = text_path::layout(vec![guide, text()], 7001, settings).unwrap();
    curved.transform = Some([1.1, 0.2, 0., 0.9, 5., 6.]);
    let before = serde_json::to_value(&curved).unwrap();
    style.apply(&mut curved).unwrap();
    let after = serde_json::to_value(&curved).unwrap();
    for key in ["id", "text", "textPath", "transform", "x", "y", "w", "h"] {
        assert_eq!(before[key], after[key], "{key}");
    }
    assert_eq!(curved.opacity, 0.7);
    assert_eq!(curved.font_options.unwrap().axes["wght"], 700.);
}
#[test]
fn project_and_library_roundtrip_collision_limits_and_invalid_template() {
    let (mut e, id) = setup();
    cmd(
        &mut e,
        json!({"type":"textStyleSave","id":id,"shapeId":7001,"name":"Titre"}),
    )
    .unwrap();
    let data = io::project_bytes(e.doc().unwrap()).unwrap();
    let doc = io::read_project(&data).unwrap();
    assert_eq!(
        serde_json::to_value(&doc.text_styles).unwrap(),
        serde_json::to_value(&e.doc().unwrap().text_styles).unwrap()
    );
    let mut merged = doc.text_styles.clone();
    text_styles::merge(&mut merged, doc.text_styles.clone()).unwrap();
    assert_eq!(merged[1].name, "Titre (importé 2)");
    let mut invalid = doc.text_styles[0].clone();
    invalid.template.x = 1.;
    assert!(invalid.validate().is_err());
    assert!(text_styles::validate(&vec![doc.text_styles[0].clone(); 129]).is_err());
    let file = std::env::temp_dir().join(format!("prisme-font-styles-{}.json", std::process::id()));
    text_styles::write(file.to_str().unwrap(), &merged).unwrap();
    let read = text_styles::read(file.to_str().unwrap()).unwrap();
    assert_eq!(read.len(), 2);
    std::fs::write(&file, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(text_styles::read(file.to_str().unwrap()).is_err());
    std::fs::remove_file(file).unwrap();
}
#[test]
fn variable_svg_optimization_and_pdf_keep_source_text_and_axes() {
    let (mut e, id) = setup();
    let mut t = text();
    t.font_options = Some(opts(700., 36.));
    cmd(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    let doc = e.doc().unwrap();
    let before = io::project_bytes(doc).unwrap();
    let svg = vector_export::optimized_svg(doc, 6).unwrap();
    assert!(svg.contains("font-variation-settings"));
    compare(
        &vector::flatten(doc).unwrap(),
        &vector::render_svg(&svg, 460, 160).unwrap(),
    );
    let pdf = vector_export::pdf(doc).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert_eq!(before, io::project_bytes(doc).unwrap());
    assert!(get(&e, 7001).font_options.is_some());
    assert!(fonts::needs_outlines(get(&e, 7001)));
}
