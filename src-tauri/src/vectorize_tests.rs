use crate::{
    document::{Content, Editor, ShapeKind},
    io, operations, vector,
    vectorize::{self, Input, Settings},
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use std::sync::Arc;
fn input(img: RgbaImage) -> Input {
    let (w, h) = img.dimensions();
    Input {
        image: Arc::new(img),
        selection: None,
        canvas: [w, h],
        offset: [0, 0],
    }
}
fn exact() -> Settings {
    Settings {
        simplify: 0.,
        ..Default::default()
    }
}
fn cmd(e: &mut Editor, v: Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
fn editor(img: RgbaImage) -> (Editor, u64) {
    let mut e = Editor::default();
    cmd(
        &mut e,
        json!({"type":"new","width":img.width(),"height":img.height(),"background":null}),
    )
    .unwrap();
    let id = e.doc().unwrap().layers[0].id;
    e.doc_mut().unwrap().layers[0].content = Content::Raster(Arc::new(img));
    e.touch(0);
    e.commit("Bitmap de référence");
    (e, id)
}
#[track_caller]
fn compare(a: &RgbaImage, b: &RgbaImage, max_limit: f64, mean_limit: f64) {
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
    if max > 0. {
        eprintln!(
            "vectorization comparison max={max} mean={mean}, limits={max_limit}/{mean_limit}"
        );
    }
    assert!(
        max <= max_limit && mean <= mean_limit,
        "max={max} mean={mean}"
    );
}
#[test]
fn every_three_by_three_binary_topology_preserves_pixels_holes_and_diagonal_contacts() {
    // All 511 nonempty patterns are independent pixel references, including
    // the eight-neighbour ambiguities that can connect holes incorrectly.
    for bits in 1u32..512 {
        let img = RgbaImage::from_fn(3, 3, |x, y| {
            if bits & (1 << (y * 3 + x)) != 0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([0; 4])
            }
        });
        let data = vectorize::trace(&input(img.clone()), &exact()).unwrap();
        let rendered = vector::rasterize(&data.shapes, 3, 3).unwrap();
        compare(&img, &rendered, 0., 0.);
        assert!(data
            .shapes
            .iter()
            .all(|s| matches!(s.kind, ShapeKind::Path { even_odd: true, .. })));
    }
}
#[test]
fn independent_rgba_pixel_reference_color_palette_opacity_and_determinism() {
    let colors = [
        [240, 40, 30, 255],
        [20, 100, 220, 255],
        [60, 180, 30, 136],
        [0; 4],
    ];
    let img = RgbaImage::from_fn(32, 24, |x, y| Rgba(colors[((x / 8 + y / 6) % 4) as usize]));
    let settings = Settings {
        mode: "color".into(),
        colors: 8,
        ..exact()
    };
    let a = vectorize::trace(&input(img.clone()), &settings).unwrap();
    let b = vectorize::trace(&input(img.clone()), &settings).unwrap();
    assert_eq!(
        serde_json::to_value(&a.shapes).unwrap(),
        serde_json::to_value(&b.shapes).unwrap()
    );
    assert_eq!(a.stats.paths, 3);
    compare(
        &img,
        &vector::rasterize(&a.shapes, 32, 24).unwrap(),
        1.,
        0.2,
    );
}
#[test]
fn monochrome_threshold_invert_ink_alpha_and_transparency_cutoff() {
    let img = RgbaImage::from_fn(8, 2, |x, y| {
        Rgba([
            x as u8 * 32,
            x as u8 * 32,
            x as u8 * 32,
            if y == 0 { 255 } else { 119 },
        ])
    });
    let i = input(img);
    let s = Settings {
        color: "#245b9e".into(),
        ..exact()
    };
    let data = vectorize::trace(&i, &s).unwrap();
    let expected = RgbaImage::from_fn(8, 2, |x, y| {
        if x < 4 {
            Rgba([36, 91, 158, if y == 0 { 255 } else { 119 }])
        } else {
            Rgba([0; 4])
        }
    });
    compare(
        &expected,
        &vector::rasterize(&data.shapes, 8, 2).unwrap(),
        1.,
        0.3,
    );
    let inverted = vectorize::trace(
        &i,
        &Settings {
            invert: true,
            alpha_threshold: 120,
            ..s
        },
    )
    .unwrap();
    assert_eq!(inverted.stats.paths, 1);
    let r = vector::rasterize(&inverted.shapes, 8, 2).unwrap();
    for y in 0..2 {
        for x in 0..8 {
            assert_eq!(r.get_pixel(x, y)[3], if x >= 4 && y == 0 { 255 } else { 0 });
        }
    }
}
#[test]
fn simplification_reduces_nodes_and_rounding_matches_independent_quadratic_corners() {
    let image = RgbaImage::from_fn(40, 40, |x, y| {
        if (x as i32 - 20).pow(2) + (y as i32 - 20).pow(2) <= 225 {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    let original = vectorize::trace(&input(image.clone()), &exact()).unwrap();
    let simple = vectorize::trace(
        &input(image.clone()),
        &Settings {
            simplify: 1.,
            ..exact()
        },
    )
    .unwrap();
    assert!(simple.stats.nodes < original.stats.nodes / 2);
    let raster = vector::rasterize(&simple.shapes, 40, 40).unwrap();
    let mismatch = image
        .pixels()
        .zip(raster.pixels())
        .filter(|(a, b)| (a[3] > 127) != (b[3] > 127))
        .count();
    assert!(mismatch < 80, "{mismatch}");
    let image = RgbaImage::from_fn(24, 24, |x, y| {
        if (4..20).contains(&x) && (4..20).contains(&y) {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    let rounded = vectorize::trace(
        &input(image),
        &Settings {
            smoothing: 0.5,
            ..exact()
        },
    )
    .unwrap();
    let reference = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M4 6 Q4 4 6 4 H18 Q20 4 20 6 V18 Q20 20 18 20 H6 Q4 20 4 18 Z" fill="#000000"/></svg>"##;
    let reference = vector::render_svg(reference, 24, 24).unwrap();
    let rendered = vector::rasterize(&rounded.shapes, 24, 24).unwrap();
    // First measurement: max 48, mean .13845, from quadratic/cubic edge
    // coverage. Keep a separate bound for rounded 24px geometry, with an
    // affected-pixel budget and exact flat interiors, not a zero-delta claim.
    compare(&reference, &rendered, 64., 0.2);
    assert!(
        reference
            .pixels()
            .zip(rendered.pixels())
            .filter(|(a, b)| a[3].abs_diff(b[3]) > 8)
            .count()
            <= 16
    );
    for y in 1..23 {
        for x in 1..23 {
            let a = reference.get_pixel(x, y)[3];
            if (y - 1..=y + 1)
                .all(|dy| (x - 1..=x + 1).all(|dx| reference.get_pixel(dx, dy)[3] == a))
            {
                assert_eq!(a, rendered.get_pixel(x, y)[3]);
            }
        }
    }
}
#[test]
fn small_islands_and_holes_removed_before_graph_budget_and_empty_outputs_refused() {
    let image = RgbaImage::from_fn(32, 24, |x, y| {
        if (((2..16).contains(&x) && (2..16).contains(&y)) && !(x == 8 && y == 8))
            || (x == 25 && y == 20)
        {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    let data = vectorize::trace(
        &input(image),
        &Settings {
            min_area: 2.,
            ..exact()
        },
    )
    .unwrap();
    assert_eq!(data.stats.removed_contours, 2);
    assert_eq!(data.stats.contours, 1);
    let r = vector::rasterize(&data.shapes, 32, 24).unwrap();
    assert_eq!(r.get_pixel(8, 8)[3], 255);
    assert_eq!(r.get_pixel(25, 20)[3], 0);
    let noisy = RgbaImage::from_fn(512, 512, |x, y| {
        if (x < 16 && y < 16) || (x + y) % 2 == 0 {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    assert!(vectorize::trace(&input(noisy.clone()), &exact())
        .unwrap_err_string()
        .contains("complexe"));
    let data = vectorize::trace(
        &input(noisy),
        &Settings {
            min_area: 2.,
            max_side: 512,
            ..exact()
        },
    )
    .unwrap();
    assert_eq!(data.stats.paths, 1);
    assert!(data.stats.removed_contours > 100_000);
    assert!(vectorize::trace(&input(RgbaImage::new(4, 4)), &exact()).is_err());
}
// Keep errors readable without deriving Debug for potentially huge image data.
trait ErrorString {
    fn unwrap_err_string(self) -> String;
}
impl ErrorString for Result<vectorize::ResultData, String> {
    fn unwrap_err_string(self) -> String {
        match self {
            Err(e) => e,
            Ok(_) => panic!("expected refusal"),
        }
    }
}
#[test]
fn selection_uses_canvas_coordinates_soft_alpha_and_offset_before_downsampling() {
    let mut i = input(RgbaImage::from_pixel(12, 8, Rgba([0, 0, 0, 255])));
    i.canvas = [20, 16];
    i.offset = [-2, 3];
    let mask = (0..320)
        .map(|n| {
            if (4..10).contains(&(n % 20)) && (5..9).contains(&(n / 20)) {
                119
            } else {
                0
            }
        })
        .collect();
    i.selection = Some(crate::selection::from_mask(20, 16, mask));
    let data = vectorize::trace(
        &i,
        &Settings {
            use_selection: true,
            ..exact()
        },
    )
    .unwrap();
    let result = vector::rasterize(&data.shapes, 12, 8).unwrap();
    for y in 0..8 {
        for x in 0..12 {
            assert_eq!(
                result.get_pixel(x, y)[3],
                if (6..12).contains(&x) && (2..6).contains(&y) {
                    119
                } else {
                    0
                }
            );
        }
    }
    let img = RgbaImage::from_pixel(2048, 64, Rgba([0, 0, 0, 255]));
    let result = vectorize::trace(
        &input(img),
        &Settings {
            max_side: 128,
            ..exact()
        },
    )
    .unwrap();
    assert_eq!((result.stats.width, result.stats.height), (128, 4));
    assert_eq!(result.shapes[0].w, 2048.);
    assert_eq!(result.shapes[0].h, 64.);
}
#[test]
fn transaction_preserves_source_properties_history_identity_and_project_svg_pdf() {
    let image = RgbaImage::from_fn(32, 24, |x, y| {
        if (2..20).contains(&x) && (3..18).contains(&y) && !(7..10).contains(&x) {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    let (mut e, id) = editor(image);
    cmd(
        &mut e,
        json!({"type":"layer","id":id,"x":3,"y":-2,"opacity":0.7}),
    )
    .unwrap();
    let source = e.doc().unwrap().layers[0].raster().unwrap().clone();
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    let history = e.info().history_index;
    let revision = e.revision();
    cmd(&mut e,json!({"type":"vectorize","id":id,"revision":revision,"settings":exact(),"hideSource":true})).unwrap();
    assert_eq!(e.info().history_index, history + 1);
    let doc = e.doc().unwrap();
    assert!(!doc.layers[0].visible);
    assert!(Arc::ptr_eq(&source, doc.layers[0].raster().unwrap()));
    assert_eq!(
        (doc.layers[1].x, doc.layers[1].y, doc.layers[1].opacity),
        (3, -2, 0.7)
    );
    compare(&pixels, &vector::flatten(doc).unwrap(), 0., 0.);
    let Content::Vector(ss) = &doc.layers[1].content else {
        panic!()
    };
    assert!(ss.iter().all(|s| s.id != 0));
    let bytes = io::project_bytes(doc).unwrap();
    let reopened = io::read_project(&bytes).unwrap();
    compare(
        &vector::flatten(doc).unwrap(),
        &vector::flatten(&reopened).unwrap(),
        0.,
        0.,
    );
    let svg = vector::export_svg(doc).unwrap();
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<image"));
    compare(
        &vector::flatten(doc).unwrap(),
        &vector::render_svg(&svg, 32, 24).unwrap(),
        1.,
        0.1,
    );
    assert!(crate::vector_export::pdf(doc).unwrap().starts_with(b"%PDF"));
    cmd(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
    cmd(&mut e, json!({"type":"redo"})).unwrap();
    assert_eq!(e.doc().unwrap().layers.len(), 2);
}
#[test]
fn stale_preview_invalid_parameters_and_complex_outputs_are_atomic() {
    let image = RgbaImage::from_fn(96, 96, |x, y| {
        if (x + y) % 2 == 0 {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([0; 4])
        }
    });
    let (mut e, id) = editor(image);
    let old = e.revision();
    cmd(&mut e, json!({"type":"layer","id":id,"x":1})).unwrap();
    let before = io::project_bytes(e.doc().unwrap()).unwrap();
    let history = e.info().history_index;
    for (revision, settings) in [
        (old, exact()),
        (
            e.revision(),
            Settings {
                max_side: 4096,
                ..exact()
            },
        ),
        (e.revision(), exact()),
    ] {
        assert!(cmd(&mut e,json!({"type":"vectorize","id":id,"revision":revision,"settings":settings,"hideSource":true})).is_err());
        assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
        assert_eq!(e.info().history_index, history);
    }
    assert!(Settings {
        simplify: f32::NAN,
        ..exact()
    }
    .validate()
    .is_err());
}
