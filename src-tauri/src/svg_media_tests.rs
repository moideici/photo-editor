use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    embedded::ImageAsset,
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
    compare_tolerance(s, 16., 10)
}
fn compare_tolerance(s: &Shape, edge_max: f64, edge_pixels: usize) -> Vec<Shape> {
    let out = svg_edit::decompose(s).unwrap();
    groups::validate(&out).unwrap();
    let a = vector::rasterize(&[s.clone()], 300, 220).unwrap();
    let b = vector::rasterize(&out, 300, 220).unwrap();
    let mut sum = 0f64;
    let mut max = 0f64;
    let mut bad = 0;
    for (p, q) in a.pixels().zip(b.pixels()) {
        let mut pixel = 0f64;
        for c in 0..4 {
            let d = if c == 3 {
                (p[c] as f64 - q[c] as f64).abs()
            } else {
                (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
            };
            max = max.max(d);
            sum += d;
            pixel = pixel.max(d);
        }
        if pixel > 8. {
            bad += 1;
        }
    }
    let mean = sum / a.as_raw().len() as f64;
    for y in 1..a.height() - 1 {
        for x in 1..a.width() - 1 {
            let p = a.get_pixel(x, y);
            let flat = (y - 1..=y + 1).all(|yy| {
                (x - 1..=x + 1).all(|xx| {
                    a.get_pixel(xx, yy)
                        .0
                        .iter()
                        .zip(p.0)
                        .all(|(v, w)| v.abs_diff(w) <= 4)
                })
            });
            if flat {
                let q = b.get_pixel(x, y);
                for c in 0..4 {
                    let d = if c == 3 {
                        p[c].abs_diff(q[c]) as f64
                    } else {
                        (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
                    };
                    assert!(d <= 3., "interior mismatch at {x},{y}: {d}");
                }
            }
        }
    }
    assert!(
        max <= edge_max && mean < 0.15 && bad < edge_pixels,
        "max {max}, mean {mean}, pixels >8: {bad}, source {:?}",
        s.kind
    );
    out
}
fn image_asset(format: image::ImageFormat) -> ImageAsset {
    let im = image::RgbaImage::from_fn(16, 12, |x, y| {
        image::Rgba([
            ((x * 31) % 256) as u8,
            ((y * 37) % 256) as u8,
            ((x * y * 11) % 256) as u8,
            if x < 8 { 255 } else { 130 },
        ])
    });
    let mut bytes = std::io::Cursor::new(vec![]);
    let dynamic = if format == image::ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(im).to_rgb8())
    } else {
        image::DynamicImage::ImageRgba8(im)
    };
    dynamic.write_to(&mut bytes, format).unwrap();
    ImageAsset::from_bytes(
        match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::Gif => "image/gif",
            _ => "image/webp",
        },
        &bytes.into_inner(),
        "optimizeQuality",
    )
    .unwrap()
}
#[test]
fn embedded_raster_formats_transforms_interpolation_and_slice_keep_pixels_and_payload() {
    for format in [
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::Gif,
        image::ImageFormat::WebP,
    ] {
        let asset = image_asset(format);
        for aspect in ["none", "xMidYMid meet", "xMaxYMin slice"] {
            for interpolation in ["optimizeQuality", "pixelated"] {
                let s = block(&format!(
                    r##"<rect width="240" height="180" fill="#aaccee"/><g opacity=".7" transform="translate(13 11) rotate(9 100 70) skewX(7)"><image x="20" y="20" width="120" height="100" href="{}" preserveAspectRatio="{aspect}" style="image-rendering:{interpolation}"/></g>"##,
                    asset.url()
                ));
                let out = compare(&s);
                let all = flat(&out);
                let images = all
                    .iter()
                    .filter_map(|s| {
                        if let ShapeKind::Image { asset } = &s.kind {
                            Some(asset)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].bytes().unwrap(), asset.bytes().unwrap());
                assert_eq!(images[0].rendering, interpolation);
            }
        }
    }
}
#[test]
fn embedded_svg_can_be_decomposed_after_nonuniform_resize_and_placement() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="60"><defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs><rect x="5" y="5" width="70" height="50" fill="url(#g)"/><circle cx="40" cy="30" r="20" fill="#22cc88"/></svg>"##;
    let asset =
        ImageAsset::from_bytes("image/svg+xml", source.as_bytes(), "optimizeQuality").unwrap();
    let s = block(&format!(
        r#"<image x="20" y="30" width="180" height="90" href="{}" preserveAspectRatio="none" transform="skewX(8)"/>"#,
        asset.url()
    ));
    let out = compare(&s);
    let mut embedded = flat(&out)
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Image { .. }))
        .unwrap()
        .clone();
    embedded.w *= 0.8;
    embedded.h *= 1.3;
    embedded.rotation = 7.;
    embedded.opacity = 0.6;
    compare(&embedded);
}
#[test]
fn editable_clips_preserve_union_holes_units_transforms_and_nested_intersections() {
    for body in [
        r##"<defs><clipPath id="c"><circle cx="65" cy="70" r="43"/><rect x="115" y="40" width="80" height="75"/></clipPath></defs><g clip-path="url(#c)"><rect width="240" height="180" fill="#ee7733"/><circle cx="130" cy="100" r="70" fill="#3355dd"/></g>"##,
        r##"<defs><clipPath id="c" transform="translate(12 8) rotate(7 100 70)"><path d="M10 10H190V140H10Z M45 40H150V110H45Z" clip-rule="evenodd"/></clipPath></defs><g transform="translate(7 5) skewX(5)" clip-path="url(#c)" opacity=".7"><rect x="10" y="10" width="200" height="145" fill="#3355dd"/></g>"##,
        r##"<defs><clipPath id="c" clipPathUnits="objectBoundingBox"><ellipse cx=".5" cy=".5" rx=".4" ry=".45"/></clipPath></defs><rect x="25" y="20" width="170" height="130" fill="#ee7733" clip-path="url(#c)" transform="rotate(5 100 80)"/>"##,
        r##"<defs><clipPath id="a" transform="translate(5 0)"><circle cx="90" cy="80" r="60"/></clipPath><clipPath id="b" clip-path="url(#a)" transform="translate(7 8)"><rect x="30" y="15" width="100" height="120"/></clipPath></defs><g clip-path="url(#b)" transform="translate(4 3)"><rect width="230" height="170" fill="#22aa66"/></g>"##,
        r##"<defs><clipPath id="a"><circle cx="70" cy="65" r="50"/></clipPath><clipPath id="b"><rect x="10" y="10" width="100" height="120" transform="translate(15 5)" clip-path="url(#a)"/><circle cx="180" cy="80" r="30"/></clipPath></defs><g clip-path="url(#b)"><rect width="240" height="180" fill="#ee7733"/></g>"##,
        r##"<defs><clipPath id="c"><text x="10" y="110" font-family="Arial" font-size="90">Ab</text></clipPath></defs><rect width="220" height="150" fill="#3355dd" clip-path="url(#c)"/>"##,
        r##"<rect x="-15" y="-10" width="260" height="190" fill="#22aa66"/><path d="M-20 30L100 10L260 150Z" fill="#ee7733"/>"##,
    ] {
        let s = block(body);
        // Glyph outlines are normalized to editable cubics. Verify separately
        // the one-pixel antialias band, rather than relaxing all import checks.
        let out = if body.contains("<text") {
            compare_tolerance(&s, 64., 100)
        } else {
            compare(&s)
        };
        assert!(flat(&out).iter().any(|s| s.clip.is_some()));
    }
}
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
#[test]
fn image_and_clip_edits_are_atomic_undoable_and_persist_in_v11() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("clips.svg");
    let asset = image_asset(image::ImageFormat::Png);
    let s = block(&format!(
        r##"<defs><clipPath id="c"><circle cx="100" cy="80" r="60"/></clipPath></defs><g clip-path="url(#c)"><image x="20" y="20" width="180" height="130" href="{}" preserveAspectRatio="none"/></g>"##,
        asset.url()
    ));
    let ShapeKind::Svg { source, .. } = &s.kind else {
        panic!()
    };
    std::fs::write(&path, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":path})).unwrap();
    let id = e.doc().unwrap().layers[0].id;
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
        panic!()
    };
    let mut group = flat(shapes)
        .into_iter()
        .find(|s| s.clip.is_some())
        .unwrap()
        .clone();
    let group_id = group.id;
    let mut image = flat(shapes)
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Image { .. }))
        .unwrap()
        .clone();
    let history = e.info().history.len();
    group.clip.as_mut().unwrap().transform[4] += 25.;
    run(&mut e, json!({"type":"shape","id":id,"shape":group})).unwrap();
    assert!(before != vector::flatten(e.doc().unwrap()).unwrap());
    assert_eq!(e.info().history.len(), history + 1);
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    let ShapeKind::Image { asset } = &mut image.kind else {
        panic!()
    };
    *asset = image_asset(image::ImageFormat::WebP);
    asset.rendering = "pixelated".into();
    run(&mut e, json!({"type":"shape","id":id,"shape":image})).unwrap();
    let edited = vector::flatten(e.doc().unwrap()).unwrap();
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let reopened = io::read_project(&bytes).unwrap();
    assert_eq!(edited, vector::flatten(&reopened).unwrap());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let metadata: serde_json::Value =
        serde_json::from_reader(zip.by_name("document.json").unwrap()).unwrap();
    assert_eq!(metadata["version"], crate::io::PROJECT_VERSION);
    let saved = io::project_bytes(e.doc().unwrap()).unwrap();
    let history = e.info().history.clone();
    assert!(run(
        &mut e,
        json!({"type":"ungroupShapes","id":id,"shapeId":group_id})
    )
    .unwrap_err()
    .contains("découpe"));
    if let ShapeKind::Image { asset } = &mut image.kind {
        asset.data = "invalid".into();
    }
    assert!(run(&mut e, json!({"type":"shape","id":id,"shape":image})).is_err());
    group.clip.as_mut().unwrap().transform = [0.; 6];
    assert!(run(&mut e, json!({"type":"shape","id":id,"shape":group})).is_err());
    assert_eq!(saved, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(history, e.info().history);
    assert!(crate::vector_export::pdf(e.doc().unwrap())
        .unwrap()
        .starts_with(b"%PDF-"));
    assert_eq!(saved, io::project_bytes(e.doc().unwrap()).unwrap());
    assert_eq!(history, e.info().history);
}
#[test]
fn missing_external_or_invalid_svg_images_are_reported_and_editable_import_refuses_atomically() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("external.svg");
    let mut e = Editor::default();
    run(
        &mut e,
        json!({"type":"new","width":240,"height":180,"background":"#ffaa55"}),
    )
    .unwrap();
    for href in [
        "missing.png",
        "https://invalid.example/private.png",
        "data:image/png;base64,broken",
        "data:image/bmp;base64,Qk0=",
    ] {
        let ShapeKind::Svg{source,..}=block(&format!(r#"<rect width="40" height="40" fill="red"/><image href="{href}" width="100" height="100"/>"#)).kind else{panic!()};
        let notices = crate::embedded::diagnostics(&source).unwrap();
        assert!(!notices.is_empty(), "{href}");
        std::fs::write(&path, source).unwrap();
        let before = io::project_bytes(e.doc().unwrap()).unwrap();
        let history = e.info().history.clone();
        assert!(run(&mut e, json!({"type":"openSvgEditable","path":path})).is_err());
        assert_eq!(before, io::project_bytes(e.doc().unwrap()).unwrap());
        assert_eq!(history, e.info().history);
        run(&mut e, json!({"type":"open","path":path})).unwrap();
        assert!(!e.doc().unwrap().metadata.notices.is_empty());
    }
}
#[test]
fn image_assets_reject_corruption_spoofed_mime_dimensions_and_unbounded_clips() {
    let asset = image_asset(image::ImageFormat::Png);
    for field in ["mime", "dimensions", "rendering", "data"] {
        let mut bad = asset.clone();
        match field {
            "mime" => bad.mime = "image/jpeg".into(),
            "dimensions" => bad.width += 1,
            "rendering" => bad.rendering = "invented".into(),
            _ => bad.data = "invalid".into(),
        };
        assert!(bad.validate().is_err());
    }
    let mut clip = crate::clip::Clip::rectangle(0., 0., 100., 100., groups::IDENTITY);
    clip.validate().unwrap();
    for _ in 0..10 {
        clip = crate::clip::Clip {
            transform: groups::IDENTITY,
            paths: clip.paths.clone(),
            intersect: Some(Box::new(clip)),
        };
    }
    assert!(clip.validate().is_err());
    assert!(ImageAsset::from_bytes("image/svg+xml",br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="secret.png"/></svg>"#,"smooth").is_err());
}
