use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations,
    text_path::{self, Settings},
    vector,
};
use serde_json::json;
fn shape(v: serde_json::Value) -> Shape {
    serde_json::from_value(v).unwrap()
}
fn guide() -> Shape {
    shape(
        json!({"id":10000,"type":"path","contours":[{"closed":false,"nodes":[{"point":[0,0],"incoming":[0,0],"outgoing":[0.3,-1]},{"point":[1,0],"incoming":[0.7,1],"outgoing":[1,0]}]}],"evenOdd":false,"x":20,"y":110,"w":380,"h":70,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1}),
    )
}
fn text() -> Shape {
    shape(
        json!({"id":10001,"type":"text","text":"Prisme • Été 2026","fontFamily":"Segoe UI","fontSize":28,"bold":false,"italic":false,"x":10,"y":20,"w":240,"h":40,"rotation":0,"fill":"#245b9e","stroke":null,"strokeWidth":0,"opacity":1}),
    )
}
fn settings() -> Settings {
    Settings {
        source_id: Some(10000),
        contour_index: 0,
        start_offset: 20.,
        percent: false,
        normal_offset: 0.,
        anchor: "start".into(),
        reverse: false,
    }
}
fn curved() -> Vec<Shape> {
    let g = guide();
    let t = text_path::layout(vec![g.clone(), text()], 10001, settings()).unwrap();
    vec![g, t]
}
fn command(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    command(
        &mut e,
        json!({"type":"new","width":460,"height":260,"background":"#ffffff"}),
    )
    .unwrap();
    command(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.doc().unwrap().layers.last().unwrap().id;
    for s in curved() {
        command(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    }
    (e, id)
}
#[track_caller]
fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
    compare_bounded(a, b, 12., 0.15, 1.);
}
#[track_caller]
fn compare_bounded(
    a: &image::RgbaImage,
    b: &image::RgbaImage,
    max_limit: f64,
    mean_limit: f64,
    fraction_limit: f64,
) {
    assert_eq!(a.dimensions(), b.dimensions());
    let (mut max, mut sum, mut over) = (0f64, 0f64, 0usize);
    for (p, q) in a.pixels().zip(b.pixels()) {
        let mut pixel_max = 0f64;
        for c in 0..4 {
            let d = if c == 3 {
                p[c].abs_diff(q[c]) as f64
            } else {
                (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
            };
            max = max.max(d);
            pixel_max = pixel_max.max(d);
            sum += d;
        }
        if pixel_max > 8. {
            over += 1;
        }
    }
    let mean = sum / a.as_raw().len() as f64;
    let fraction = over as f64 / (a.width() * a.height()) as f64;
    eprintln!("text glyph comparison max={max} mean={mean} pixelsOver8={over} fraction={fraction} limits={max_limit}/{mean_limit}/{fraction_limit}");
    assert!(
        max <= max_limit && mean < mean_limit && fraction <= fraction_limit,
        "max {max} mean {mean} pixels over8 {over}"
    );
}
#[test]
fn independent_svg_curved_text_offsets_anchors_reverse_and_font_styles_match() {
    for (offset, percent, anchor, reverse, normal, bold, italic) in [
        (20., false, "start", false, 0., false, false),
        (50., true, "middle", false, 8., true, false),
        (90., true, "end", false, -6., false, true),
        (10., false, "start", true, 3., false, false),
    ] {
        let mut s = settings();
        s.start_offset = offset;
        s.percent = percent;
        s.anchor = anchor.into();
        s.reverse = reverse;
        s.normal_offset = normal;
        let mut t = text();
        if let ShapeKind::Text {
            bold: b, italic: i, ..
        } = &mut t.kind
        {
            *b = bold;
            *i = italic;
        }
        let t = text_path::layout(vec![guide(), t], 10001, s).unwrap();
        let d = if reverse {
            "M400 110 C286 180 134 40 20 110"
        } else {
            "M20 110 C134 40 286 180 400 110"
        };
        let raw=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"460\" height=\"260\"><defs><path id=\"guide\" d=\"{d}\"/></defs><text fill=\"#245b9e\" font-family=\"Segoe UI\" font-size=\"28\" font-weight=\"{}\" font-style=\"{}\" text-anchor=\"{anchor}\" dy=\"{normal}\"><textPath href=\"#guide\" startOffset=\"{offset}{}\">Prisme • Été 2026</textPath></text></svg>",if bold{"bold"}else{"normal"},if italic{"italic"}else{"normal"},if percent{"%"}else{""});
        compare(
            &vector::render_svg(&raw, 460, 260).unwrap(),
            &vector::rasterize(&[t], 460, 260).unwrap(),
        );
    }
}
#[test]
fn linked_nodes_text_edits_and_missing_guide_refusals_are_atomic() {
    let (mut e, id) = editor();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let mut g = groups::find(shapes, 10000).unwrap().clone();
    if let ShapeKind::Path { contours, .. } = &mut g.kind {
        contours[0].nodes[0].outgoing[1] = -2.;
    }
    command(&mut e, json!({"type":"shape","id":id,"shape":g})).unwrap();
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
    command(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    let serialized = io::project_bytes(e.doc().unwrap()).unwrap();
    e.info();
    let info = serde_json::to_value(e.info()).unwrap();
    assert!(command(
        &mut e,
        json!({"type":"deleteShape","id":id,"shapeId":10000})
    )
    .unwrap_err()
    .contains("détachez"));
    assert_eq!(serialized, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(info, serde_json::to_value(e.info()).unwrap());
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let mut t = groups::find(shapes, 10001).unwrap().clone();
    if let ShapeKind::Text {
        text, font_size, ..
    } = &mut t.kind
    {
        *text = "Édition après lien".into();
        *font_size = 34.;
    }
    command(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
}
#[test]
fn guides_across_group_frames_grouping_and_duplicate_ids_remain_linked() {
    // Re-evaluating a curve in a translated group changes floating-point arc
    // placement at a raster edge: first measurement max 14, mean 0.0001254.
    let (mut e, id) = editor();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    command(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[10000,10001]}),
    )
    .unwrap();
    compare_bounded(
        &before,
        &vector::flatten(e.doc().unwrap()).unwrap(),
        16.,
        0.001,
        0.0001,
    );
    let gid = match &e.doc().unwrap().layers.last().unwrap().content {
        Content::Vector(s) => s[0].id,
        _ => 0,
    };
    command(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":gid}),
    )
    .unwrap();
    compare_bounded(
        &before,
        &vector::flatten(e.doc().unwrap()).unwrap(),
        16.,
        0.001,
        0.0001,
    );
    command(&mut e, json!({"type":"duplicateLayer","id":id})).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let t = shapes.iter().find(|s| s.text_path.is_some()).unwrap();
    let guide = t.text_path.as_ref().unwrap().source_id.unwrap();
    assert_ne!(guide, 10000);
    assert!(groups::find(shapes, guide).is_some());
    groups::validate(shapes).unwrap();
    let mut shapes = curved();
    let mut g = crate::clip::base();
    g.id = 20000;
    g.fill = None;
    g.x = 15.;
    g.y = 20.;
    g.w = 400.;
    g.h = 200.;
    g.kind = ShapeKind::Group {
        children: vec![shapes.remove(0)],
        intrinsic_w: 400.,
        intrinsic_h: 200.,
    };
    g.transform = Some([0.95, 0.1, 0.15, 1.05, 0., 0.]);
    shapes.insert(0, g);
    text_path::refresh(&mut shapes).unwrap();
    groups::validate(&shapes).unwrap();
    assert_ne!(before, vector::rasterize(&shapes, 460, 260).unwrap());
}
#[test]
fn detaching_keeps_pixels_and_allows_independent_geometry_and_guide_deletion() {
    let (mut e, id) = editor();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let mut s = settings();
    s.source_id = None;
    let mut t = text_path::layout(shapes.clone(), 10001, s).unwrap();
    command(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    command(
        &mut e,
        json!({"type":"deleteShape","id":id,"shapeId":10000}),
    )
    .unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    t.x += 15.;
    t.rotation = 7.;
    command(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
    command(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
}
#[test]
fn v15_roundtrip_keeps_text_link_and_rejects_invalid_layouts() {
    let (e, _) = editor();
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    let data = io::project_bytes(e.doc().unwrap()).unwrap();
    let d = io::read_project(&data).unwrap();
    assert_eq!(pixels, vector::flatten(&d).unwrap());
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&data)).unwrap();
    let meta: serde_json::Value =
        serde_json::from_reader(z.by_name("document.json").unwrap()).unwrap();
    assert_eq!(meta["version"], crate::io::PROJECT_VERSION);
    for key in 0..5 {
        let mut t = text();
        let mut s = settings();
        match key {
            0 => s.source_id = Some(999999),
            1 => s.contour_index = 999,
            2 => s.normal_offset = 5000.,
            3 => s.anchor = "invented".into(),
            _ => {
                if let ShapeKind::Text { text, .. } = &mut t.kind {
                    *text = "deux\nlignes".into();
                }
            }
        };
        assert!(text_path::layout(vec![guide(), t], 10001, s).is_err());
    }
    let mut invalid = guide();
    invalid.text_path = curved()[1].text_path.clone();
    assert!(io::validate_shape(&invalid).is_err());
}
#[test]
fn native_outline_conversion_pdf_copy_and_svg_keep_curved_glyphs() {
    let (mut e, id) = editor();
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    let original = io::project_bytes(e.doc().unwrap()).unwrap();
    let svg = vector::export_svg(e.doc().unwrap()).unwrap();
    assert!(svg.contains("<textPath"));
    compare(&pixels, &vector::render_svg(&svg, 460, 260).unwrap());
    // Quadratic font outlines become editable cubic contours. Their raster edge
    // coverage differs (measured max 41, mean 0.00936, 106 pixels >8). Keep a
    // separate, explicit edge criterion without changing native SVG criteria.
    let copy = crate::vector_export::prepare_pdf_images(e.doc().unwrap()).unwrap();
    compare_bounded(&pixels, &vector::flatten(&copy).unwrap(), 48., 0.02, 0.001);
    assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
    let pdf = crate::vector_export::pdf(e.doc().unwrap()).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    command(&mut e, json!({"type":"toPath","id":id,"shapeId":10001})).unwrap();
    compare_bounded(
        &pixels,
        &vector::flatten(e.doc().unwrap()).unwrap(),
        48.,
        0.02,
        0.001,
    );
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    assert!(matches!(
        groups::find(shapes, 10001).unwrap().kind,
        ShapeKind::Group { .. }
    ));
    assert!(!vector::export_svg(e.doc().unwrap())
        .unwrap()
        .contains("<textPath"));
    command(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
}
#[test]
fn native_selection_matches_visible_glyphs_and_rejects_empty_bounding_pixels() {
    let shapes = curved();
    let pixels = vector::rasterize(&shapes, 460, 260).unwrap();
    let mut e = Editor::default();
    let layer = e.new_vector_layer("Texte", shapes);
    let Content::Vector(s) = &layer.content else {
        panic!()
    };
    let id = s.iter().find(|s| s.text_path.is_some()).unwrap().id;
    let (mut hit, mut blank) = (0, 0);
    for y in (55..180).step_by(7) {
        for x in (20..420).step_by(7) {
            let expected = pixels.get_pixel(x, y)[3] > 0;
            let found = crate::svg_mask::pick(&layer, &[id], [x as f32, y as f32])
                .unwrap()
                .is_some();
            assert_eq!(found, expected, "{x} {y}");
            if found {
                hit += 1;
            } else {
                blank += 1;
            }
        }
    }
    assert!(hit > 10 && blank > 100);
}
#[test]
fn closed_and_compound_guides_keep_contour_choice_and_empty_text_exports() {
    let mut ellipse = guide();
    ellipse.kind = ShapeKind::Ellipse;
    ellipse.x = 40.;
    ellipse.y = 35.;
    ellipse.w = 320.;
    ellipse.h = 170.;
    let mut st = settings();
    st.percent = true;
    st.start_offset = 15.;
    let t = text_path::layout(vec![ellipse.clone(), text()], 10001, st).unwrap();
    assert!(t.text_path.as_ref().unwrap().contour.closed);
    assert!(vector::rasterize(&[t], 460, 260)
        .unwrap()
        .pixels()
        .any(|p| p[3] > 0));
    let mut g = guide();
    if let ShapeKind::Path { contours, .. } = &mut g.kind {
        let mut second = contours[0].clone();
        for n in &mut second.nodes {
            for p in [&mut n.point, &mut n.incoming, &mut n.outgoing] {
                p[1] += 0.8;
            }
        }
        contours.push(second);
    }
    let mut st = settings();
    st.contour_index = 1;
    let t = text_path::layout(vec![g.clone(), text()], 10001, st).unwrap();
    assert_eq!(t.text_path.as_ref().unwrap().contour_index, 1);
    let mut shapes = vec![g, t];
    if let ShapeKind::Path { contours, .. } = &mut shapes[0].kind {
        contours.pop();
    }
    assert!(text_path::refresh(&mut shapes).is_err());
    let (mut e, id) = editor();
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let mut t = groups::find(shapes, 10001).unwrap().clone();
    if let ShapeKind::Text { text, .. } = &mut t.kind {
        text.clear();
    }
    command(&mut e, json!({"type":"shape","id":id,"shape":t})).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let prepared = crate::vector_export::prepare_pdf_images(e.doc().unwrap()).unwrap();
    assert_eq!(before, vector::flatten(&prepared).unwrap());
    crate::vector_export::pdf(e.doc().unwrap()).unwrap();
}

#[test]
fn ungrouping_a_scaled_rotated_group_keeps_curved_font_transform() {
    let (mut e, id) = editor();
    command(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[10000,10001]}),
    )
    .unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers.last().unwrap().content else {
        panic!()
    };
    let mut group = shapes[0].clone();
    let group_id = group.id;
    group.w *= 1.3;
    group.h *= 0.8;
    group.rotation = 7.;
    command(&mut e, json!({"type":"shape","id":id,"shape":group})).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    command(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":group_id}),
    )
    .unwrap();
    compare_bounded(
        &before,
        &vector::flatten(e.doc().unwrap()).unwrap(),
        24.,
        0.01,
        0.0005,
    );
}
