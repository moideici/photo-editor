use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations, vector,
};
use serde_json::json;
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn make() -> (Editor, u64, Vec<u64>) {
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":160,"height":120,"background":null}),
    )
    .unwrap();
    run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.active_hint.unwrap();
    for (x, y, fill) in [
        (10, 15, "#ff0000"),
        (30, 25, "#0040ff"),
        (80, 30, "#00ff00"),
    ] {
        run(&mut e,json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":0,"x":x,"y":y,"w":40,"h":35,"rotation":0,"fill":fill,"stroke":null,"strokeWidth":0,"opacity":1}})).unwrap();
    }
    let ids = shapes(&e).iter().map(|s| s.id).collect();
    (e, id, ids)
}
fn shapes(e: &Editor) -> &Vec<Shape> {
    let Content::Vector(s) = &e.doc().unwrap().layers[1].content else {
        panic!()
    };
    s
}
fn edit(e: &mut Editor, id: u64, mut shape: Shape, f: impl FnOnce(&mut Shape)) {
    f(&mut shape);
    run(e, json!({"type":"shape","id":id,"shape":shape})).unwrap()
}
fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut sum = 0u64;
    for (a, b) in a.as_raw().iter().zip(b.as_raw()) {
        let d = a.abs_diff(*b);
        sum += d as u64;
        assert!(d <= 2, "delta {d}")
    }
    assert!((sum as f64 / a.as_raw().len() as f64) < 0.02);
}
#[test]
fn groups_preserve_stacking_styles_children_project_and_undo() {
    let (mut e, id, ids) = make();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let n = e.info().history.len();
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[0],ids[1]]}),
    )
    .unwrap();
    assert_eq!(e.info().history.len(), n + 1);
    compare(&before, &vector::flatten(e.doc().unwrap()).unwrap());
    let group = shapes(&e)[0].clone();
    assert!(matches!(group.kind, ShapeKind::Group { .. }));
    let mut child = groups::find(shapes(&e), ids[0]).unwrap().clone();
    child.fill = Some("#ffff00".into());
    run(&mut e, json!({"type":"shape","id":id,"shape":child})).unwrap();
    assert_eq!(
        vector::flatten(e.doc().unwrap())
            .unwrap()
            .get_pixel(15, 20)
            .0,
        [255, 255, 0, 255]
    );
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let loaded = io::read_project(&bytes).unwrap();
    compare(
        &vector::flatten(e.doc().unwrap()).unwrap(),
        &vector::flatten(&loaded).unwrap(),
    );
    let mut reopened = Editor::default();
    reopened.load(loaded, None, "Ouvrir");
    assert!(reopened.next_id() > ids[2]);
    run(&mut e, json!({"type":"undo"})).unwrap();
    compare(&before, &vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(shapes(&e).len(), 3);
    run(&mut e, json!({"type":"redo"})).unwrap();
    assert_eq!(shapes(&e).len(), 2);
}
#[test]
fn nested_nonuniform_rotated_groups_dissociate_without_rasterization() {
    let (mut e, id, ids) = make();
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[0],ids[1]]}),
    )
    .unwrap();
    let inner = shapes(&e)[0].id;
    let group = shapes(&e)[0].clone();
    edit(&mut e, id, group, |s| {
        s.w *= 1.3;
        s.h *= 0.8;
        s.rotation = 23.;
        s.x += 5.;
    });
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[inner,ids[2]]}),
    )
    .unwrap();
    let outer = shapes(&e)[0].id;
    let group = shapes(&e)[0].clone();
    edit(&mut e, id, group, |s| {
        s.w *= 0.85;
        s.h *= 1.1;
        s.rotation = -17.;
    });
    let expected = vector::flatten(e.doc().unwrap()).unwrap();
    run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":outer}),
    )
    .unwrap();
    compare(&expected, &vector::flatten(e.doc().unwrap()).unwrap());
    run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":inner}),
    )
    .unwrap();
    compare(&expected, &vector::flatten(e.doc().unwrap()).unwrap());
    assert_eq!(shapes(&e).len(), 3);
    assert!(shapes(&e)
        .iter()
        .all(|s| matches!(s.kind, ShapeKind::Rect { .. }) && s.transform.is_some()));
    let loaded = io::read_project(&io::project_bytes(e.doc().unwrap()).unwrap()).unwrap();
    compare(&expected, &vector::flatten(&loaded).unwrap());
}
#[test]
fn group_opacity_is_isolated_and_invalid_grouping_is_atomic() {
    let (mut e, id, ids) = make();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let history = e.info().history;
    assert!(run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[0],ids[2]]})
    )
    .unwrap_err()
    .contains("consécutives"));
    assert_eq!(e.info().history, history);
    compare(&before, &vector::flatten(e.doc().unwrap()).unwrap());
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[0],ids[1]]}),
    )
    .unwrap();
    let group = shapes(&e)[0].clone();
    let gid = group.id;
    edit(&mut e, id, group, |s| s.opacity = 0.5);
    let image = vector::flatten(e.doc().unwrap()).unwrap();
    assert_eq!(image.get_pixel(35, 30).0, [0, 64, 255, 128]);
    assert_eq!(image.get_pixel(15, 20)[3], 128);
    let history = e.info().history;
    assert!(run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":gid})
    )
    .unwrap_err()
    .contains("100 %"));
    assert_eq!(e.info().history, history);
    compare(&image, &vector::flatten(e.doc().unwrap()).unwrap());
    let mut invalid = shapes(&e)[0].clone();
    if let ShapeKind::Group { children, .. } = &mut invalid.kind {
        children[1].id = children[0].id;
    }
    assert!(run(&mut e, json!({"type":"shape","id":id,"shape":invalid})).is_err());
    assert_eq!(e.info().history, history);
}
#[test]
fn grouped_image_transforms_match_exported_svg_and_keep_children() {
    let (mut e, id, ids) = make();
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[0],ids[1]]}),
    )
    .unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    run(&mut e, json!({"type":"rotate","degrees":90})).unwrap();
    assert_eq!(
        vector::flatten(e.doc().unwrap()).unwrap(),
        image::imageops::rotate90(&before)
    );
    run(&mut e, json!({"type":"flip","horizontal":true})).unwrap();
    let flipped = vector::flatten(e.doc().unwrap()).unwrap();
    assert_eq!(
        flipped,
        image::imageops::flip_horizontal(&image::imageops::rotate90(&before))
    );
    assert!(groups::find(shapes(&e), ids[0]).is_some());
    run(&mut e, json!({"type":"resize","width":144,"height":128})).unwrap();
    assert!(groups::find(shapes(&e), ids[0]).is_some());
    let svg = vector::export_svg(e.doc().unwrap()).unwrap();
    compare(
        &vector::flatten(e.doc().unwrap()).unwrap(),
        &vector::render_svg(&svg, 144, 128).unwrap(),
    );
}

#[test]
fn legacy_projects_migrate_ids_and_invalid_trees_are_rejected() {
    use std::io::{Cursor, Read, Write};
    let (e, _, _) = make();
    let original = vector::flatten(e.doc().unwrap()).unwrap();
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let mut source = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..source.len() {
        let mut file = source.by_index(i).unwrap();
        let name = file.name().unwrap().into_owned();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if name == "document.json" {
            let mut p: serde_json::Value = serde_json::from_slice(&data).unwrap();
            p["version"] = json!(6);
            for s in p["layers"][1]["shapes"].as_array_mut().unwrap() {
                s["id"] = json!(0);
            }
            data = serde_json::to_vec(&p).unwrap();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&data).unwrap();
    }
    let migrated = io::read_project(&output.finish().unwrap().into_inner()).unwrap();
    compare(&original, &vector::flatten(&migrated).unwrap());
    let Content::Vector(s) = &migrated.layers[1].content else {
        panic!()
    };
    groups::validate(s).unwrap();
    let mut invalid = s.clone();
    invalid[0].transform = Some([0.; 6]);
    assert!(groups::validate(&invalid).is_err());
    invalid = s.clone();
    invalid[0].id = u64::MAX;
    assert!(groups::validate(&invalid).is_err());
    assert!(groups::repair_legacy_ids(&mut invalid).is_err());
    let mut nested = s[0].clone();
    for id in 10..28 {
        nested = Shape {
            font_options: None,
            text_path: None,
            svg_unclipped: false,
            svg_viewport: false,
            filters: None,
            mask: None,
            clip: None,
            group_style: None,
            fill_pattern: None,
            stroke_pattern: None,
            fill_gradient: None,
            stroke_gradient: None,
            id,
            transform: None,
            kind: ShapeKind::Group {
                children: vec![nested],
                intrinsic_w: 160.,
                intrinsic_h: 120.,
            },
            x: 0.,
            y: 0.,
            w: 160.,
            h: 120.,
            rotation: 0.,
            fill: None,
            stroke: None,
            stroke_width: 0.,
            opacity: 1.,
        };
    }
    assert!(groups::validate(&[nested])
        .unwrap_err()
        .contains("16 niveaux"));
}

#[test]
fn transformed_text_in_a_group_keeps_text_editable_when_ungrouped() {
    let (mut e, id, ids) = make();
    run(&mut e,json!({"type":"shape","id":id,"shape":{"id":0,"type":"text","text":"Texte éditable","fontFamily":"Segoe UI","fontSize":18,"bold":true,"italic":false,"x":5,"y":75,"w":140,"h":28,"rotation":-5,"fill":"#123456","stroke":null,"strokeWidth":0,"opacity":0.8}})).unwrap();
    let text_id = shapes(&e)[3].id;
    run(
        &mut e,
        json!({"type":"groupShapes","id":id,"shapeIds":[ids[2],text_id]}),
    )
    .unwrap();
    let group = shapes(&e)[2].clone();
    let gid = group.id;
    edit(&mut e, id, group, |s| {
        s.w *= 0.8;
        s.h *= 1.1;
        s.rotation = 17.;
    });
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":gid}),
    )
    .unwrap();
    compare(&before, &vector::flatten(e.doc().unwrap()).unwrap());
    let shape = groups::find(shapes(&e), text_id).unwrap();
    assert!(matches!(&shape.kind,ShapeKind::Text{text,..} if text=="Texte éditable"));
    let mut changed = shape.clone();
    if let ShapeKind::Text { text, .. } = &mut changed.kind {
        *text = "Toujours éditable".into();
    }
    run(&mut e, json!({"type":"shape","id":id,"shape":changed})).unwrap();
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
}
