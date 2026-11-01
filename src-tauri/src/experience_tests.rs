use crate::{
    document::Editor,
    io, layer_tools, operations,
    workspace::{View, Workspace},
};
use serde_json::{json, Value};
fn cmd(e: &mut Editor, v: Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn editor() -> (Editor, u64) {
    let mut e = Editor::default();
    cmd(
        &mut e,
        json!({"type":"new","width":64,"height":48,"background":null}),
    )
    .unwrap();
    let id = e.doc().unwrap().layers[0].id;
    cmd(&mut e,json!({"type":"stroke","id":id,"points":[[12,16],[42,16]],"size":12,"color":"#245b9e","opacity":0.5,"erase":false})).unwrap();
    (e, id)
}
fn bytes(e: &Editor) -> Vec<u8> {
    io::project_bytes(e.doc().unwrap()).unwrap()
}
#[test]
fn miniature_matches_independent_isolated_reference_with_alpha_and_effects() {
    use base64::Engine;
    let (mut e, id) = editor();
    cmd(
        &mut e,
        json!({"type":"layer","id":id,"x":5,"y":-2,"opacity":0.7,"visible":false}),
    )
    .unwrap();
    let d = e.doc().unwrap();
    let thumb = layer_tools::thumbnail(&d.layers[0], 64, 48, 64).unwrap();
    let image = io::decode(
        &base64::engine::general_purpose::STANDARD
            .decode(thumb.split_once(',').unwrap().1)
            .unwrap(),
    )
    .unwrap();
    let mut reference = image::RgbaImage::new(64, 64);
    crate::raster::composite(
        &mut reference,
        d.layers[0].raster().unwrap(),
        5,
        6,
        0.7,
        Default::default(),
    );
    let mut max = 0f64;
    let mut sum = 0.;
    for (a, b) in reference.pixels().zip(image.pixels()) {
        for c in 0..4 {
            let d = if c == 3 {
                a[c].abs_diff(b[c]) as f64
            } else {
                (a[c] as f64 * a[3] as f64 / 255. - b[c] as f64 * b[3] as f64 / 255.).abs()
            };
            max = max.max(d);
            sum += d;
        }
    }
    let mean = sum / reference.as_raw().len() as f64;
    eprintln!("isolated thumbnail max={max} mean={mean}");
    assert!(max <= 2. && mean <= 0.1);
    let before = bytes(&e);
    assert_eq!(before, bytes(&e));
}
#[test]
fn locks_refuse_mutations_atomically_and_alpha_preserves_original_coverage() {
    let (mut e, id) = editor();
    cmd(&mut e,json!({"type":"layerBatch","ids":[id],"kind":"locks","locks":{"position":true,"pixels":true,"alpha":false}})).unwrap();
    let before = bytes(&e);
    let history = e.info().history_index;
    for v in [
        json!({"type":"layer","id":id,"x":4}),
        json!({"type":"stroke","id":id,"points":[[10,10]],"size":10,"color":"#f00fff","opacity":1,"erase":false}),
        json!({"type":"flatten"}),
        json!({"type":"deleteLayer","id":id}),
    ] {
        assert!(cmd(&mut e, v).is_err());
        assert_eq!(bytes(&e), before);
        assert_eq!(e.info().history_index, history);
    }
    cmd(&mut e,json!({"type":"layerBatch","ids":[id],"kind":"locks","locks":{"position":false,"pixels":false,"alpha":true}})).unwrap();
    let alpha = e.doc().unwrap().layers[0]
        .raster()
        .unwrap()
        .pixels()
        .map(|p| p[3])
        .collect::<Vec<_>>();
    cmd(&mut e,json!({"type":"stroke","id":id,"points":[[0,16],[60,16]],"size":24,"color":"#ff0000","opacity":1,"erase":false})).unwrap();
    assert_eq!(
        alpha,
        e.doc().unwrap().layers[0]
            .raster()
            .unwrap()
            .pixels()
            .map(|p| p[3])
            .collect::<Vec<_>>()
    );
    let persisted = io::read_project(&bytes(&e)).unwrap();
    assert!(persisted.layers[0].locks.alpha);
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert!(e.doc().unwrap().layers[0].locks.alpha);
}
#[test]
fn multiple_layer_duplicate_delete_visibility_one_undo_and_invalid_batches() {
    let (mut e, id) = editor();
    cmd(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let v = e.doc().unwrap().layers[1].id;
    e.info();
    cmd(&mut e,json!({"type":"layerBatch","ids":[id],"kind":"locks","locks":{"position":true,"pixels":false,"alpha":false}})).unwrap();
    assert_eq!(
        e.active_hint, None,
        "A lock must not activate a different layer"
    );
    cmd(&mut e,json!({"type":"layerBatch","ids":[id],"kind":"locks","locks":{"position":false,"pixels":false,"alpha":false}})).unwrap();

    let before = bytes(&e);
    let index = e.info().history_index;
    cmd(
        &mut e,
        json!({"type":"layerBatch","ids":[id,v],"kind":"duplicate"}),
    )
    .unwrap();
    assert_eq!(e.doc().unwrap().layers.len(), 4);
    assert_eq!(e.info().history_index, index + 1);
    let ids = e
        .doc()
        .unwrap()
        .layers
        .iter()
        .map(|l| l.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 4);
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(bytes(&e), before);
    for ids in [vec![id, id], vec![id, 999999]] {
        assert!(cmd(&mut e, json!({"type":"layerBatch","ids":ids,"kind":"hide"})).is_err());
        assert_eq!(bytes(&e), before);
    }
    assert!(cmd(
        &mut e,
        json!({"type":"layerBatch","ids":[id,v],"kind":"delete"})
    )
    .is_err());
    cmd(
        &mut e,
        json!({"type":"layerVisibility","states":{id.to_string():false,v.to_string():true}}),
    )
    .unwrap();
    assert!(!e.doc().unwrap().layers[0].visible);
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(bytes(&e), before);
}
#[test]
fn tabs_keep_independent_history_selection_locks_and_view() {
    let (mut e, id) = editor();
    let mut w = Workspace::default();
    let first = w.status(&e)["active"].as_u64().unwrap();
    cmd(&mut e,json!({"type":"selection","selection":{"shape":{"type":"rect","x":2,"y":3,"w":8,"h":9},"inverted":false}})).unwrap();
    let before = bytes(&e);
    let rev = e.revision();
    w.view(View {
        zoom: 2.,
        active: Some(id),
        selected: None,
        pan: [40., -20.],
    })
    .unwrap();
    w.open(
        &mut e,
        serde_json::from_value(json!({"type":"new","width":64,"height":48,"background":"#ffffff"}))
            .unwrap(),
    )
    .unwrap();
    let second = w.active;
    assert_ne!(first, second);
    assert_ne!(rev, e.revision());
    cmd(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let other = bytes(&e);
    w.switch(&mut e, first).unwrap();
    assert_eq!(bytes(&e), before);
    assert_eq!(w.status(&e)["view"]["zoom"], 2.);
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert!(e.doc().unwrap().selection.is_none());
    w.switch(&mut e, second).unwrap();
    assert_eq!(bytes(&e), other);
    assert_eq!(e.doc().unwrap().layers.len(), 2);
    assert!(w.close(&mut e, false).is_err());
    w.close(&mut e, true).unwrap();
    assert_eq!(w.active, first);
}
#[test]
fn recovery_survives_restart_and_checkpoint_does_not_erase_pending_session() {
    let temp = tempfile::tempdir().unwrap();
    let (mut e, _) = editor();
    let mut w = Workspace::default();
    w.init(temp.path().into());
    w.checkpoint(&e).unwrap();
    let original = w.recoveries()[0].file.clone();
    let expected = bytes(&e);
    let mut restarted = Workspace::default();
    restarted.init(temp.path().into());
    let empty = Editor::default();
    restarted.checkpoint(&empty).unwrap();
    assert!(restarted.recoveries().iter().any(|r| r.file == original));
    restarted.restore(&mut e, &original).unwrap();
    assert_eq!(bytes(&e), expected);
    assert!(e.dirty);
    assert!(e.path.is_none());
    assert!(restarted.restore(&mut e, "../private").is_err());
    restarted.forget_recovery(&original).unwrap();
    assert!(!temp.path().join(&original).exists());
}
#[test]
fn normal_save_removes_own_checkpoint_but_keeps_recent_file_full_path() {
    let temp = tempfile::tempdir().unwrap();
    let (mut e, _) = editor();
    let mut w = Workspace::default();
    w.init(temp.path().join("session"));
    w.checkpoint(&e).unwrap();
    assert_eq!(w.recoveries().len(), 1);
    let file = temp.path().join("important.prisme");
    cmd(&mut e, json!({"type":"save","path":file})).unwrap();
    w.remember(file.to_str().unwrap().into(), &e).unwrap();
    w.checkpoint(&e).unwrap();
    assert!(w.recoveries().is_empty());
    assert_eq!(w.recent[0].path, file.to_str().unwrap());
    assert!(w.recent[0].thumbnail.starts_with("data:image/png"));
    std::fs::remove_file(&file).unwrap();
    assert_eq!(w.status(&e)["recent"][0]["missing"], true);
}
#[test]
fn tab_limit_failed_open_and_bad_recovery_preserve_active_document() {
    let (mut e, _) = editor();
    let mut w = Workspace::default();
    w.status(&e);
    for _ in 0..7 {
        w.open(
            &mut e,
            serde_json::from_value(json!({"type":"new","width":8,"height":8,"background":null}))
                .unwrap(),
        )
        .unwrap();
    }
    let before = bytes(&e);
    let active = w.active;
    assert!(w
        .open(
            &mut e,
            serde_json::from_value(json!({"type":"new","width":8,"height":8,"background":null}))
                .unwrap()
        )
        .is_err());
    assert!(w
        .open(
            &mut e,
            serde_json::from_value(json!({"type":"open","path":"C:/absent-prisme-981.prisme"}))
                .unwrap()
        )
        .is_err());
    assert_eq!(w.active, active);
    assert_eq!(bytes(&e), before);
    assert!(w
        .view(View {
            zoom: f32::NAN,
            ..Default::default()
        })
        .is_err());
}
