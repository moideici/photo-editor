use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations, svg_edit,
    svg_filter::{self, FilterResource},
    vector,
};
use base64::{engine::general_purpose::STANDARD, Engine};
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
fn delta(a: &image::RgbaImage, b: &image::RgbaImage) -> (f64, f64) {
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
    (max, sum / a.as_raw().len() as f64)
}
fn compare(s: &Shape) -> Vec<Shape> {
    let out = svg_edit::decompose(s).unwrap();
    groups::validate(&out).unwrap();
    let a = vector::rasterize(&[s.clone()], 300, 230).unwrap();
    let b = vector::rasterize(&out, 300, 230).unwrap();
    let (max, mean) = delta(&a, &b);
    if max > 12. || mean >= 0.15 {
        std::fs::create_dir_all("../work/feimage-debug").unwrap();
        a.save("../work/feimage-debug/before.png").unwrap();
        b.save("../work/feimage-debug/after.png").unwrap();
        std::fs::write(
            "../work/feimage-debug/after.svg",
            vector::svg_document(
                300,
                230,
                &out.iter().map(vector::shape_svg).collect::<String>(),
            ),
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
fn image() -> image::RgbaImage {
    image::RgbaImage::from_fn(40, 20, |x, y| {
        image::Rgba([
            (x * 6) as u8,
            (y * 12) as u8,
            if x < 20 { 50 } else { 220 },
            if y < 10 { 255 } else { 160 },
        ])
    })
}
fn source(href: &str, attributes: &str, defs: &str) -> Shape {
    block(&format!(
        r##"<defs>{defs}<filter id="f" filterUnits="userSpaceOnUse" x="5" y="5" width="220" height="165"><feImage href="{href}" {attributes} result="texture"/><feComposite in="texture" in2="SourceGraphic" operator="in"/></filter></defs><g transform="translate(5 4) rotate(7 110 80) skewX(5)" filter="url(#f)" opacity=".8"><rect x="20" y="20" width="190" height="135" fill="#dd5533"/></g>"##
    ))
}
#[test]
fn feimage_raster_formats_aspect_placement_units_and_sampling_preserve_pixels() {
    for (mime, format) in [
        ("image/png", image::ImageFormat::Png),
        ("image/jpeg", image::ImageFormat::Jpeg),
        ("image/gif", image::ImageFormat::Gif),
        ("image/webp", image::ImageFormat::WebP),
    ] {
        let mut bytes = std::io::Cursor::new(Vec::new());
        let data = if mime == "image/jpeg" {
            image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(image()).to_rgb8())
        } else {
            image::DynamicImage::ImageRgba8(image())
        };
        data.write_to(&mut bytes, format).unwrap();
        let href = format!("data:{mime};base64,{}", STANDARD.encode(bytes.into_inner()));
        for aspect in ["xMidYMid meet", "xMaxYMin slice", "none"] {
            for rendering in ["optimizeQuality", "pixelated"] {
                let out = compare(&source(
                    &href,
                    &format!(
                        r#"x="25" y="20" width="170" height="125" preserveAspectRatio="{aspect}" style="image-rendering:{rendering}""#
                    ),
                    "",
                ));
                let resource = flat(&out)
                    .iter()
                    .find_map(|s| s.filters.as_ref())
                    .unwrap()
                    .filters[0]
                    .primitives[0]
                    .resource
                    .as_ref()
                    .unwrap();
                assert!(flat(&resource.children)
                    .iter()
                    .any(|s| matches!(&s.kind,ShapeKind::Image{asset}if asset.mime==mime)));
            }
        }
    }
    let href = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(io::png_bytes(&image()).unwrap())
    );
    compare(&block(&format!(
        r##"<defs><filter id="f" primitiveUnits="objectBoundingBox"><feImage href="{href}" x=".1" y=".1" width=".8" height=".8" preserveAspectRatio="xMinYMax meet"/></filter></defs><g filter="url(#f)" transform="rotate(8 100 70)"><rect x="30" y="25" width="150" height="110"/></g>"##
    )));
}
#[test]
fn feimage_vector_references_inherited_filters_and_nested_svg_resources_keep_styles() {
    let defs = r##"<linearGradient id="p"><stop stop-color="#ff6633"/><stop offset="1" stop-color="#3355cc"/></linearGradient><clipPath id="c"><circle cx="70" cy="60" r="50"/></clipPath><mask id="m"><rect width="180" height="130" fill="white"/><circle cx="75" cy="60" r="20" fill="#555555"/></mask><filter id="shadow"><feDropShadow stdDeviation="2" dx="4" dy="3"/></filter><g id="art" transform="translate(8 5) rotate(8 70 60)" opacity=".7" clip-path="url(#c)" mask="url(#m)"><rect x="10" y="15" width="150" height="100" fill="url(#p)"/><g filter="url(#shadow)"><circle cx="95" cy="65" r="30" fill="#55cc99"/></g></g>"##;
    let out = compare(&source(
        "#art",
        r#"x="20" y="15" width="180" height="140""#,
        defs,
    ));
    let resource = flat(&out)
        .iter()
        .find_map(|s| s.filters.as_ref())
        .unwrap()
        .filters[0]
        .primitives[0]
        .resource
        .as_ref()
        .unwrap();
    assert!(flat(&resource.children).iter().any(|s| s.mask.is_some()));
    let inner = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="90"><defs><filter id="f"><feGaussianBlur in="SourceAlpha" stdDeviation="2" result="height"/><feDiffuseLighting in="height" surfaceScale="3" lighting-color="#ee9966"><feDistantLight azimuth="45" elevation="45"/></feDiffuseLighting></filter></defs><rect x="15" y="15" width="85" height="60" fill="#33aa77" filter="url(#f)"/></svg>"##;
    let overflowing = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="90"><defs><filter id="f" x="-.5" y="-.5" width="2" height="2"><feDropShadow stdDeviation="3" dx="7" dy="7"/></filter></defs><rect width="120" height="90" fill="#44bb88" filter="url(#f)"/></svg>"##;
    compare(&source(
        &format!("data:image/svg+xml;base64,{}", STANDARD.encode(overflowing)),
        r#"x="20" y="15" width="180" height="140""#,
        "",
    ));
    let href = format!("data:image/svg+xml;base64,{}", STANDARD.encode(inner));
    let out = compare(&source(
        &href,
        r#"x="20" y="15" width="180" height="140""#,
        "",
    ));
    let r = flat(&out)
        .iter()
        .find_map(|s| s.filters.as_ref())
        .unwrap()
        .filters[0]
        .primitives[0]
        .resource
        .as_ref()
        .unwrap();
    assert!(flat(&r.children).iter().any(|s| s.filters.is_some()));
    compare(&block(
        r##"<defs><g id="art"><rect x="5" y="5" width="90" height="70" fill="#33aa99"/></g><filter id="base" filterUnits="userSpaceOnUse" x="10" y="10" width="200" height="150"><feImage href="#art"/></filter><filter id="f" href="#base"/></defs><rect x="20" y="20" width="150" height="120" filter="url(#f)"/>"##,
    ));
}
fn run(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
    operations::execute(e, serde_json::from_value(v).unwrap())
}
#[test]
fn feimage_resources_edit_undo_duplicate_and_v14_are_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("resource.svg");
    let s = source(
        "#art",
        r#"x="20" y="10" width="180" height="140""#,
        r##"<g id="art"><rect x="5" y="5" width="140" height="100" fill="#3377bb"/></g>"##,
    );
    let ShapeKind::Svg { source, .. } = s.kind else {
        panic!()
    };
    std::fs::write(&p, source).unwrap();
    let mut e = Editor::default();
    run(&mut e, json!({"type":"openSvgEditable","path":p})).unwrap();
    let layer = e.doc().unwrap().layers[0].id;
    let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
        panic!()
    };
    let mut group = (*flat(shapes).iter().find(|s| s.filters.is_some()).unwrap())
        .clone()
        .clone();
    let original = io::project_bytes(e.doc().unwrap()).unwrap();
    let pixels = vector::flatten(e.doc().unwrap()).unwrap();
    group.filters.as_mut().unwrap().filters[0].primitives[0]
        .resource
        .as_mut()
        .unwrap()
        .children[0]
        .opacity = 0.4;
    run(&mut e, json!({"type":"shape","id":layer,"shape":group})).unwrap();
    assert_ne!(pixels, vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
    for kind in 0..4 {
        let mut bad = group.clone();
        let p = &mut bad.filters.as_mut().unwrap().filters[0].primitives[0];
        match kind {
            0 => p.resource = None,
            1 => p.resource.as_mut().unwrap().children.clear(),
            2 => p.resource.as_mut().unwrap().children[0].opacity = 2.,
            _ => {
                p.kind = "feOffset".into();
            }
        }
        let history = e.info().history_index;
        assert!(run(&mut e, json!({"type":"shape","id":layer,"shape":bad})).is_err());
        assert_eq!(history, e.info().history_index);
        assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
    }
    let saved = io::read_project(&original).unwrap();
    assert_eq!(pixels, vector::flatten(&saved).unwrap());
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&original)).unwrap();
    let meta: serde_json::Value =
        serde_json::from_reader(z.by_name("document.json").unwrap()).unwrap();
    assert_eq!(meta["version"], crate::io::PROJECT_VERSION);
    run(&mut e, json!({"type":"duplicateLayer","id":layer})).unwrap();
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_eq!(pixels, vector::flatten(e.doc().unwrap()).unwrap());
}
#[test]
fn missing_external_corrupt_and_normalized_away_resources_are_explicit() {
    for href in [
        "#missing",
        "https://example.invalid/image.png",
        "file:///C:/absent.png",
        "data:image/png;base64,bm90LWFuLWltYWdl",
        "",
    ] {
        let s = source(href, "", "");
        let message = svg_edit::decompose(&s).unwrap_err();
        assert!(message.contains("conservé"), "{message}");
    }
    let s = source("#empty", "", r#"<g id="empty"/>"#);
    assert!(svg_edit::decompose(&s).unwrap_err().contains("conservé"));
    let s = block(
        r##"<defs><filter id="unused"><feImage href="#missing"/></filter></defs><rect x="20" y="20" width="100" height="80" fill="#ff5522"/>"##,
    );
    compare(&s);
    let nested = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><defs><filter id="f"><feImage href="#missing"/></filter></defs><rect width="20" height="20" filter="url(#f)"/></svg>"##;
    let href = format!("data:image/svg+xml;base64,{}", STANDARD.encode(nested));
    assert!(svg_edit::decompose(&source(&href, "", ""))
        .unwrap_err()
        .contains("conservé"));
}
#[test]
fn resource_asset_fit_preview_and_nested_limits_are_checked() {
    let asset = crate::embedded::ImageAsset::from_bytes(
        "image/png",
        &io::png_bytes(&image()).unwrap(),
        "pixelated",
    )
    .unwrap();
    for fit in ["meet", "slice", "none"] {
        for align in ["xMinYMin", "xMidYMid", "xMaxYMax"] {
            let r = FilterResource::from_asset(asset.clone(), 160., 120., fit, align).unwrap();
            r.validate().unwrap();
            let p = r.preview().unwrap();
            assert!(p.width() <= 224 && p.height() <= 224 && p.pixels().any(|p| p[3] > 0));
        }
    }
    assert!(FilterResource::from_asset(asset.clone(), 160., 0., "meet", "xMidYMid").is_err());
    assert!(FilterResource::from_asset(asset, 160., 120., "oops", "xMidYMid").is_err());
    let mut p = svg_filter::catalog().pop().unwrap();
    let mut resource = p.resource.clone().unwrap();
    resource.children = vec![resource.children[0].clone(); 1001];
    assert!(resource.validate().is_err());
    for _ in 0..10 {
        let mut s = crate::clip::base();
        s.kind = ShapeKind::Group {
            children: vec![crate::clip::base()],
            intrinsic_w: 1.,
            intrinsic_h: 1.,
        };
        s.fill = None;
        s.filters = Some(svg_filter::FilterChain {
            transform: groups::IDENTITY,
            filters: vec![svg_filter::Filter {
                region: [0., 0., 100., 100.],
                primitives: vec![p],
            }],
        });
        p = svg_filter::catalog().pop().unwrap();
        p.resource = Some(FilterResource { children: vec![s] });
    }
    assert!(p.resource.unwrap().validate().is_err());
}
#[test]
fn feimage_pdf_copy_native_selection_and_svg_export_keep_resource_pixels() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("resource.svg");
    let href = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(io::png_bytes(&image()).unwrap())
    );
    let ShapeKind::Svg { source, .. } = source(
        &href,
        r#"x="35" y="25" width="150" height="115" preserveAspectRatio="none""#,
        "",
    )
    .kind
    else {
        panic!()
    };
    std::fs::write(&p, source).unwrap();
    for editable in [true, false] {
        let mut e = Editor::default();
        run(
            &mut e,
            json!({"type":if editable{"openSvgEditable"}else{"open"},"path":p}),
        )
        .unwrap();
        let original = io::project_bytes(e.doc().unwrap()).unwrap();
        let a = vector::flatten(e.doc().unwrap()).unwrap();
        let prepared = crate::vector_export::prepare_pdf_images(e.doc().unwrap()).unwrap();
        let (max, mean) = delta(&a, &vector::flatten(&prepared).unwrap());
        assert!(max <= 12. && mean < 0.15, "{max}, {mean}");
        assert!(crate::vector_export::pdf(e.doc().unwrap())
            .unwrap()
            .starts_with(b"%PDF"));
        assert_eq!(original, io::project_bytes(e.doc().unwrap()).unwrap());
        let svg = vector::export_svg(e.doc().unwrap()).unwrap();
        let (max, mean) = delta(&a, &vector::render_svg(&svg, 240, 180).unwrap());
        assert!(max <= 12. && mean < 0.15, "{max}, {mean}");
        if editable {
            let l = &e.doc().unwrap().layers[0];
            let Content::Vector(shapes) = &l.content else {
                panic!()
            };
            let id = flat(shapes)
                .iter()
                .find(|s| s.filters.is_some())
                .unwrap()
                .id;
            for y in (5..175).step_by(19) {
                for x in (5..235).step_by(19) {
                    assert_eq!(
                        crate::svg_mask::pick(l, &[id], [x as f32, y as f32])
                            .unwrap()
                            .is_some(),
                        a.get_pixel(x, y)[3] > 0,
                        "{x},{y}"
                    );
                }
            }
        }
    }
}
