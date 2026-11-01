//! Régressions du jalon 1.2 : résultats observables et conservation des documents.
use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    io, operations, paths, vector,
};
use serde_json::json;
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn new() -> Editor {
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":64,"height":48,"background":"#ffffff"}),
    )
    .unwrap();
    e
}
fn rectangle(x: f32, y: f32, w: f32, h: f32) -> Shape {
    serde_json::from_value(json!({"type":"rect","id":0,"x":x,"y":y,"w":w,"h":h,"rotation":0,"radius":0,"fill":"#ff0000","stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
}
#[test]
fn boolean_membership_holes_empty_and_rotated_paths() {
    let a = paths::convert(&rectangle(5.0, 5.0, 30.0, 30.0)).unwrap();
    let b = paths::convert(&rectangle(20.0, 5.0, 30.0, 30.0)).unwrap();
    for (kind, expected) in [
        ("union", [true, true, true]),
        ("difference", [true, false, false]),
        ("intersection", [false, true, false]),
        ("xor", [true, false, true]),
    ] {
        let s = paths::boolean(&a, &b, kind).unwrap();
        let image = vector::rasterize(&[s], 64, 48).unwrap();
        for (x, opaque) in [10, 25, 45].into_iter().zip(expected) {
            assert_eq!(image.get_pixel(x, 15)[3] > 0, opaque, "{kind} {x}");
        }
    }
    let hole = paths::boolean(
        &a,
        &paths::convert(&rectangle(12.0, 12.0, 10.0, 10.0)).unwrap(),
        "difference",
    )
    .unwrap();
    let image = vector::rasterize(&[hole], 64, 48).unwrap();
    assert_eq!(image.get_pixel(15, 15)[3], 0);
    assert_eq!(image.get_pixel(8, 8)[3], 255);
    let empty = paths::boolean(&a, &a, "difference").unwrap();
    assert!(vector::rasterize(&[empty], 64, 48)
        .unwrap()
        .pixels()
        .all(|p| p[3] == 0));
    let mut rotated = a.clone();
    rotated.rotation = 30.0;
    assert!(paths::boolean(&rotated, &b, "intersection").is_ok());
    let mut open = b.clone();
    if let ShapeKind::Path { contours, .. } = &mut open.kind {
        contours[0].closed = false;
    }
    assert!(paths::boolean(&a, &open, "union").is_err());
}
#[test]
fn curves_project_exports_boolean_and_undo_keep_editable_nodes() {
    let mut e = new();
    run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.doc().unwrap().layers[1].id;
    let mut shape = paths::convert(&rectangle(5.0, 5.0, 30.0, 30.0)).unwrap();
    if let ShapeKind::Path { contours, .. } = &mut shape.kind {
        contours[0].nodes[0].outgoing = [0.7, 0.0];
        contours[0].nodes[1].incoming = [1.0, 0.7];
    }
    run(&mut e, json!({"type":"shape","id":id,"shape":shape})).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let packed = io::project_bytes(e.doc().unwrap()).unwrap();
    let reopened = io::read_project(&packed).unwrap();
    assert_eq!(vector::flatten(&reopened).unwrap(), before);
    let svg = String::from_utf8(io::export_bytes(e.doc().unwrap(), "svg", 90).unwrap()).unwrap();
    assert!(svg.contains('C'));
    assert!(io::export_bytes(e.doc().unwrap(), "pdf", 90)
        .unwrap()
        .starts_with(b"%PDF-"));
    let shape2 = paths::convert(&rectangle(20.0, 5.0, 30.0, 30.0)).unwrap();
    run(&mut e, json!({"type":"shape","id":id,"shape":shape2})).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
        panic!()
    };
    let ids = (shapes[0].id, shapes[1].id);
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    run(
        &mut e,
        json!({"type":"boolean","id":id,"first":ids.0,"second":ids.1,"kind":"difference"}),
    )
    .unwrap();
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
    let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
        panic!()
    };
    assert_eq!(shapes.len(), 2);
    assert!(matches!(shapes[0].kind, ShapeKind::Path { .. }));
    let mut invalid = shape;
    if let ShapeKind::Path { contours, .. } = &mut invalid.kind {
        contours[0].nodes[0].point[0] = f32::NAN;
    }
    assert!(io::validate_shape(&invalid).is_err());
}
#[test]
fn gif_export_preserves_sequence_disposal_delays_and_loop_semantics() {
    use crate::animation::{Animation, AnimationFrame};
    use image::{Rgba, RgbaImage};
    use std::sync::Arc;
    let mut e = new();
    let frames = vec![
        AnimationFrame {
            image: Arc::new(RgbaImage::from_fn(64, 48, |x, _| {
                if x < 20 {
                    Rgba([255, 0, 0, 255])
                } else {
                    Rgba([0, 0, 0, 0])
                }
            })),
            delay_ms: 50,
        },
        AnimationFrame {
            image: Arc::new(RgbaImage::from_fn(64, 48, |x, _| {
                if x > 40 {
                    Rgba([0, 255, 0, 255])
                } else {
                    Rgba([0, 0, 0, 0])
                }
            })),
            delay_ms: 80,
        },
    ];
    for loops in [0, 1, 3, 65536] {
        e.doc_mut().unwrap().animation = Some(Animation {
            id: 1,
            frames: Arc::new(frames.clone()),
            current: 1,
            loops,
        });
        let bytes = io::export_bytes(e.doc().unwrap(), "gif", 90).unwrap();
        let (_, decoded) = crate::animation::decode_gif(&bytes).unwrap();
        let decoded = decoded.unwrap();
        assert_eq!(decoded.loops, loops);
        assert_eq!(decoded.info().delays, vec![50, 80]);
        for (a, b) in decoded.frames.iter().zip(&frames) {
            assert_eq!(a.image, b.image);
        }
    }
    e.doc_mut().unwrap().animation = None;
    assert!(io::export_bytes(e.doc().unwrap(), "gif", 90)
        .unwrap()
        .starts_with(b"GIF89a"));
}
#[test]
fn merge_prefix_preserves_every_blend_and_upper_layers() {
    for mode in [
        "normal",
        "multiply",
        "screen",
        "overlay",
        "darken",
        "lighten",
        "difference",
        "add",
    ] {
        let mut e = new();
        for _ in 0..3 {
            run(&mut e, json!({"type":"addLayer","kind":"raster"})).unwrap();
            let id = e.doc().unwrap().layers.last().unwrap().id;
            run(
                &mut e,
                json!({"type":"fill","id":id,"x":0,"y":0,"color":"#7090b0","tolerance":0}),
            )
            .unwrap();
            run(
                &mut e,
                json!({"type":"layer","id":id,"blend":mode,"opacity":0.63}),
            )
            .unwrap();
        }
        let doc = e.doc().unwrap();
        let id = doc.layers[2].id;
        let upper = doc.layers[3].id;
        let before = vector::flatten(doc).unwrap();
        run(&mut e, json!({"type":"mergePrefix","id":id})).unwrap();
        assert_eq!(e.doc().unwrap().layers.len(), 2);
        assert_eq!(e.doc().unwrap().layers[1].id, upper);
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before, "{mode}");
        run(&mut e, json!({"type":"undo"})).unwrap();
        assert_eq!(e.doc().unwrap().layers.len(), 4);
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
        let id = e.doc().unwrap().layers[1].id;
        run(&mut e, json!({"type":"mergeDown","id":id})).unwrap();
        assert_eq!(
            vector::flatten(e.doc().unwrap()).unwrap(),
            before,
            "pair {mode}"
        );
    }
}
