use crate::{
    document::{Content, Editor, Shape, ShapeKind},
    groups, io, operations,
    paint::*,
    vector,
};
use serde_json::json;
fn shape() -> Shape {
    serde_json::from_value(json!({"id":7,"type":"rect","radius":0,"x":20,"y":25,"w":140,"h":90,"rotation":0,"fill":"#ff0000","stroke":null,"strokeWidth":0,"opacity":1})).unwrap()
}
fn pattern() -> Pattern {
    let mut tile = shape();
    tile.x = 0.;
    tile.y = 0.;
    tile.w = 9.;
    tile.h = 18.;
    tile.fill = Some("#2040aa".into());
    Pattern {
        units: Units::UserSpaceOnUse,
        transform: groups::IDENTITY,
        x: 0.,
        y: 0.,
        width: 18.,
        height: 18.,
        content_width: 18.,
        content_height: 18.,
        children: vec![tile],
    }
}
fn compare(a: &image::RgbaImage, b: &image::RgbaImage) {
    let mut sum = 0.;
    let mut max = 0f64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        for c in 0..4 {
            let d = if c == 3 {
                (p[3] as f64 - q[3] as f64).abs()
            } else {
                (p[c] as f64 * p[3] as f64 / 255. - q[c] as f64 * q[3] as f64 / 255.).abs()
            };
            sum += d;
            max = max.max(d);
        }
    }
    // Le carreau transparent est rendu puis répété : le second arrondi alpha
    // peut différer d'un niveau, y compris après mise en contours des traits.
    eprintln!(
        "motif : max {max}, moyenne {}",
        sum / a.as_raw().len() as f64
    );
    assert!(
        max <= 3. && sum / (a.as_raw().len() as f64) < 0.1,
        "max {max}, mean {}",
        sum / a.as_raw().len() as f64
    );
}
#[test]
fn imported_pattern_units_viewbox_offsets_and_content_transforms_preserve_pixels() {
    for defs in [
        r##"<pattern id="p" patternUnits="userSpaceOnUse" x="3" y="5" width="20" height="16" patternTransform="rotate(12)"><rect width="20" height="16" fill="#aaccff"/><g transform="translate(2 1) rotate(5 8 8)"><circle cx="8" cy="8" r="5" fill="#4020aa" fill-opacity=".65"/></g></pattern>"##,
        r##"<pattern id="p" width=".2" height=".25" patternContentUnits="objectBoundingBox" patternTransform="skewX(13)"><rect width=".1" height=".25" fill="#22aa66"/><path d="M0 .1L.2 .2" stroke="#4433aa" stroke-width=".02"/></pattern>"##,
        r##"<linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue" stop-opacity=".6"/></linearGradient><pattern id="p" patternUnits="userSpaceOnUse" width="24" height="18" viewBox="5 7 12 9" preserveAspectRatio="none"><rect x="5" y="7" width="6" height="9" fill="url(#g)"/><ellipse cx="14" cy="11" rx="2" ry="3" fill="#ffaa00"/></pattern>"##,
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180"><defs>{defs}</defs><g transform="translate(8 4) scale(.9 .8)"><rect x="20" y="20" width="190" height="150" fill="url(#p)" stroke="url(#p)" stroke-width="5"/></g></svg>"##
        );
        let mut s = shape();
        s.x = 0.;
        s.y = 0.;
        s.w = 240.;
        s.h = 180.;
        s.kind = ShapeKind::Svg {
            source,
            intrinsic_w: 240.,
            intrinsic_h: 180.,
        };
        let out = crate::svg_edit::decompose(&s).unwrap();
        assert!(out.iter().all(|s| s.fill_pattern.is_some()));
        compare(
            &vector::rasterize(&[s], 240, 180).unwrap(),
            &vector::rasterize(&out, 240, 180).unwrap(),
        );
    }
}
#[test]
fn patterns_on_text_and_geometry_roundtrip_units_and_boolean() {
    for text in [false, true] {
        let mut s = shape();
        if text {
            s.kind = ShapeKind::Text {
                text: "Motif WWW".into(),
                font_family: "Segoe UI".into(),
                font_size: 26.,
                bold: true,
                italic: false,
            };
        }
        let mut p = pattern();
        p.width = 0.13;
        p.height = 0.2;
        p.units = Units::ObjectBoundingBox;
        s.fill_pattern = Some(p);
        let before = vector::rasterize(&[s.clone()], 240, 180).unwrap();
        let [x, y, w, h] = bounds(&s).unwrap();
        let p = s.fill_pattern.as_mut().unwrap();
        p.transform = [w, 0., 0., h, x, y];
        p.units = Units::UserSpaceOnUse;
        compare(&before, &vector::rasterize(&[s], 240, 180).unwrap());
    }
    let mut a = shape();
    a.fill_pattern = Some(pattern());
    a.rotation = 12.;
    a.transform = Some([1., 0.1, 0.2, 1., 4., 2.]);
    let a = crate::paths::convert(&a).unwrap();
    let mut b = shape();
    b.x = 10.;
    b.w = 55.;
    let result =
        crate::paths::boolean(&a, &crate::paths::convert(&b).unwrap(), "difference").unwrap();
    assert!(result.fill_pattern.is_some());
    let before = vector::rasterize(&[a], 240, 180).unwrap();
    let after = vector::rasterize(&[result], 240, 180).unwrap();
    let mut checked = 0;
    for (p, q) in before.pixels().zip(after.pixels()) {
        if p[3] == 255 && q[3] == 255 {
            checked += 1;
            for c in 0..3 {
                assert!(p[c].abs_diff(q[c]) <= 2);
            }
        }
    }
    assert!(checked > 500);
}
#[test]
fn pattern_project_history_and_invalid_or_recursive_tiles_are_atomic() {
    let mut e = Editor::default();
    let run = |e: &mut Editor, v| operations::execute(e, serde_json::from_value(v).unwrap());
    run(
        &mut e,
        json!({"type":"new","width":240,"height":180,"background":null}),
    )
    .unwrap();
    run(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
    let id = e.active_hint.unwrap();
    let mut s = shape();
    s.id = 0;
    s.fill_pattern = Some(pattern());
    run(&mut e, json!({"type":"shape","id":id,"shape":s})).unwrap();
    let before = vector::flatten(e.doc().unwrap()).unwrap();
    let bytes = io::project_bytes(e.doc().unwrap()).unwrap();
    let doc = io::read_project(&bytes).unwrap();
    assert_eq!(before, vector::flatten(&doc).unwrap());
    let Content::Vector(shapes) = &doc.layers[1].content else {
        panic!()
    };
    let s = shapes[0].clone();
    for kind in 0..5 {
        let mut invalid = s.clone();
        let p = invalid.fill_pattern.as_mut().unwrap();
        match kind {
            0 => p.width = 0.,
            1 => p.transform = [0.; 6],
            2 => {let mut nested=pattern();for _ in 0..9{let mut outer=pattern();outer.children[0].fill_pattern=Some(nested);nested=outer;}p.children[0].fill_pattern=Some(nested);},
            3 => p.children = vec![shape(); 201],
            _ => p.children[0].opacity = 2.,
        }
        let history = e.info().history;
        assert!(run(&mut e, json!({"type":"shape","id":id,"shape":invalid})).is_err());
        assert_eq!(history, e.info().history);
        assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
    }
    run(&mut e, json!({"type":"undo"})).unwrap();
    assert_ne!(before, vector::flatten(e.doc().unwrap()).unwrap());
    run(&mut e, json!({"type":"redo"})).unwrap();
    assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
}

#[test]
fn complex_and_nested_pattern_resources_are_editable_and_bounded(){
 for body in [
 r##"<pattern id="q" width="5" height="6" patternUnits="userSpaceOnUse"><rect width="3" height="6" fill="#3455aa"/></pattern><pattern id="p" width="20" height="18" patternUnits="userSpaceOnUse"><rect width="20" height="18" fill="#ffbb44"/><g opacity=".65" transform="rotate(8 10 9)"><rect width="14" height="18" fill="url(#q)"/><circle cx="9" cy="9" r="7" fill="#11cc66"/></g></pattern>"##,
 r##"<clipPath id="c"><circle cx="10" cy="9" r="8"/></clipPath><pattern id="p" width="20" height="18" patternUnits="userSpaceOnUse"><g clip-path="url(#c)"><rect width="20" height="18" fill="#ee2233"/><image x="2" y="3" width="8" height="9" href="data:image/png;base64,__PNG__"/></g></pattern>"##,
 ]{
  use base64::Engine;let png=base64::engine::general_purpose::STANDARD.encode(io::png_bytes(&image::RgbaImage::from_pixel(2,2,image::Rgba([31,132,247,211]))).unwrap());let body=body.replace("__PNG__",&png);let source=format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180"><defs>{body}</defs><rect x="20" y="10" width="190" height="160" fill="url(#p)"/></svg>"##);
  let mut s=shape();s.x=0.;s.y=0.;s.w=240.;s.h=180.;s.kind=ShapeKind::Svg{source,intrinsic_w:240.,intrinsic_h:180.};
  let editable=crate::svg_edit::decompose(&s).unwrap();compare(&vector::rasterize(&[s],240,180).unwrap(),&vector::rasterize(&editable,240,180).unwrap());
  let mut e=Editor::default();operations::execute(&mut e,serde_json::from_value(json!({"type":"new","width":240,"height":180,"background":null})).unwrap()).unwrap();let layer=e.new_vector_layer("Pattern",editable);e.doc_mut().unwrap().layers.push(layer);
  let round=io::read_project(&io::project_bytes(e.doc().unwrap()).unwrap()).unwrap();assert_eq!(vector::flatten(&round).unwrap(),vector::flatten(e.doc().unwrap()).unwrap());
 }
}
