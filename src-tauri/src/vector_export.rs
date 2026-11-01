//! Exports de diffusion ; le projet conserve les formes et calques source.
use crate::{document::Document, vector};
use std::sync::{Arc, OnceLock};

pub fn optimized_svg(doc: &Document, precision: u8) -> Result<String, String> {
    let source = vector::export_svg(doc)?;
    // usvg 0.49 renders variations but its text writer omits variation/optical
    // CSS. Keep the faithful source until that serializer supports both.
    // The project and ordinary SVG remain editable; never change font axes
    // just to reduce the export's size.
    if source.contains("font-variation-settings:") {
        return Ok(source);
    }
    let tree = vector::internal_tree(&source)?;
    let options = resvg::usvg::WriteOptions {
        preserve_text: true,
        coordinates_precision: precision.clamp(2, 8),
        transforms_precision: precision.clamp(2, 8),
        indent: resvg::usvg::Indent::None,
        ..Default::default()
    };
    Ok(tree.to_string(&options))
}

fn check_pdf_paints(group: &resvg::usvg::Group, depth: usize) -> Result<(), String> {
    use resvg::usvg::{Node, Paint, SpreadMethod};
    fn complex_pattern(group: &resvg::usvg::Group) -> bool {
        if group.mask().is_some() || !group.filters().is_empty() { return true; }
        group.children().iter().any(|node| match node {
            Node::Image(_) => true,
            Node::Group(g) => complex_pattern(g),
            Node::Text(t) => complex_pattern(t.flattened()),
            Node::Path(p) => [p.fill().map(|f|f.paint()),p.stroke().map(|s|s.paint())]
                .into_iter().flatten().any(|paint|matches!(paint,Paint::Pattern(_))),
        })
    }
    if depth > 64 {
        return Err("Export PDF : arbre SVG trop profond".into());
    }
    if group.blend_mode() == resvg::usvg::BlendMode::PlusLighter {
        return Err("Export PDF : fusion additive SVG non prise en charge par ce convertisseur. Exportez en SVG pour conserver cette fusion.".into());
    }
    for node in group.children() {
        if let Node::Path(p) = node {
            for paint in [p.fill().map(|f| f.paint()), p.stroke().map(|s| s.paint())]
                .into_iter()
                .flatten()
            {
                let unsupported = match paint {
                    Paint::LinearGradient(g) => g.spread_method() != SpreadMethod::Pad,
                    Paint::RadialGradient(g) => {
                        g.spread_method() != SpreadMethod::Pad || g.fr().get() != 0.
                    }
                    Paint::Pattern(p) => complex_pattern(p.root()),
                    _ => false,
                };
                if unsupported {
                    return Err("Export PDF : dégradé répété/miroir ou rayon de foyer non pris en charge. Exportez en SVG pour conserver cette peinture.".into());
                }
            }
        }
        if let Node::Group(g) = node {
            check_pdf_paints(g, depth + 1)?;
        }
        let mut result = Ok(());
        node.subroots(|root| {
            if result.is_ok() {
                result = check_pdf_paints(root, depth + 1);
            }
        });
        result?;
    }
    Ok(())
}
// svg2pdf 0.13 discards image-rendering and writes raster images without an
// interpolation hint. Bake their selected sampling at the page's 96 px/inch
// before conversion. The editor and saved asset are never changed.
pub(crate) fn prepare_pdf_images(doc: &Document) -> Result<Document, String> {
    use crate::{
        document::{Content, Shape, ShapeKind},
        groups,
    };
    fn has_resources(group: &resvg::usvg::Group, depth: usize) -> bool {
        if depth > 64 || group.mask().is_some() || !group.filters().is_empty() {
            return true;
        }
        group.children().iter().any(|node| {
            if let resvg::usvg::Node::Group(g) = node {
                if has_resources(g, depth + 1) {
                    return true;
                }
            }
            let mut found = matches!(node, resvg::usvg::Node::Image(_));
            node.subroots(|g| {
                found |= has_resources(g, depth + 1);
            });
            found
        })
    }
    fn walk_mask(
        mask: &mut crate::svg_mask::VectorMask,
        parent: groups::Matrix,
        pixels: &mut u64,
        crop: [f32; 4],
    ) -> Result<(), String> {
        if mask.mode == crate::svg_mask::MaskMode::Luminance {
            *mask = mask.pdf_alpha(parent, crop, pixels)?;
            return Ok(());
        }
        walk(
            &mut mask.children,
            groups::multiply(parent, mask.transform),
            pixels,
            crop,
            None,
        )?;
        if let Some(next) = &mut mask.intersect {
            walk_mask(next, parent, pixels, crop)?;
        }
        Ok(())
    }
    fn walk(
        shapes: &mut [Shape],
        parent: groups::Matrix,
        pixels: &mut u64,
        crop: [f32; 4],
        viewport: Option<(&Shape, groups::Matrix)>,
    ) -> Result<(), String> {
        for s in shapes {
            if s.text_path.is_some() || crate::fonts::needs_outlines(s) {
                *s = crate::text_path::outlines(s)?;
            }
            let viewport_clip = s.svg_viewport && vector::viewport_overflows(s);
            if s.filters.is_some() {
                let image = s
                    .filters
                    .as_ref()
                    .unwrap()
                    .filters
                    .iter()
                    .flat_map(|f| &f.primitives)
                    .any(|p| p.kind == "feImage");
                *s = crate::svg_filter::pdf_image(
                    s,
                    parent,
                    crop,
                    pixels,
                    if image { viewport } else { None },
                )?;
                continue;
            }
            let source = match &s.kind {
                ShapeKind::Svg { source, .. } => Some(source.clone()),
                ShapeKind::Image { asset } if asset.mime == "image/svg+xml" => {
                    asset.bytes().ok().and_then(|b| String::from_utf8(b).ok())
                }
                _ => None,
            };
            let has_resource = source
                .as_deref()
                .and_then(|source| vector::parse(source).ok())
                .is_some_and(|tree| has_resources(tree.root(), 0));
            if has_resource {
                // Older usvg used by svg2pdf discards images nested inside
                // an SVG image. Move compatible resources into the export
                // tree; refuse an unsupported conversion instead of losing it.
                {
                    let mut editable = crate::svg_edit::decompose(s).map_err(|e| {
                        format!("Export PDF : {e} Exportez en SVG pour conserver ce bloc.")
                    })?;
                    walk(&mut editable, parent, pixels, crop, viewport)?;
                    let id = s.id;
                    *s = crate::svg_edit::wrap_group(
                        editable,
                        1.,
                        Some(crate::group_style::GroupStyle {
                            blend: "normal".into(),
                            isolate: true,
                        }),
                    )?;
                    s.id = id;
                    continue;
                }
            }
            let frame = groups::multiply(parent, groups::frame(s));
            let matrix = groups::multiply(
                parent,
                groups::multiply(s.transform.unwrap_or(groups::IDENTITY), groups::rotation(s)),
            );
            let local_viewport = s.svg_viewport.then(|| s.clone());
            match &mut s.kind {
                ShapeKind::Group { children, .. } => walk(
                    children,
                    frame,
                    pixels,
                    crop,
                    local_viewport.as_ref().map(|s| (s, parent)),
                )?,
                ShapeKind::Image { asset }
                    if asset.mime != "image/svg+xml"
                        && !["optimizeSpeed", "crisp-edges", "pixelated"]
                            .contains(&asset.rendering.as_str()) =>
                {
                    let (w, h) = (
                        (s.w * matrix[0].hypot(matrix[1])).round().max(1.) as u32,
                        (s.h * matrix[2].hypot(matrix[3])).round().max(1.) as u32,
                    );
                    crate::io::dimensions(w, h).map_err(|_| {
                        "Export PDF : image pré-échantillonnée trop grande ; exportez en SVG"
                    })?;
                    *pixels += w as u64 * h as u64;
                    if *pixels > 16_777_216 {
                        return Err("Export PDF : plus de 16 millions de pixels à pré-échantillonner ; exportez en SVG".into());
                    }
                    let svg=vector::svg_document(w,h,&format!("<image width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\" style=\"image-rendering:{}\" href=\"{}\"/>",asset.rendering,asset.url()));
                    let bytes = crate::io::png_bytes(&vector::render_svg(&svg, w, h)?)?;
                    *asset = crate::embedded::ImageAsset::from_bytes(
                        "image/png",
                        &bytes,
                        "optimizeSpeed",
                    )?;
                }
                _ => (),
            }
            if let Some(mask) = &mut s.mask {
                walk_mask(mask, frame, pixels, crop)?;
            }
            if s.svg_viewport {
                if viewport_clip {
                    if let ShapeKind::Group {
                        intrinsic_w,
                        intrinsic_h,
                        ..
                    } = &s.kind
                    {
                        let mut clip = crate::clip::Clip::rectangle(
                            0.,
                            0.,
                            *intrinsic_w,
                            *intrinsic_h,
                            groups::IDENTITY,
                        );
                        clip.intersect = s.clip.take().map(Box::new);
                        s.clip = Some(clip);
                    }
                }
                s.group_style
                    .get_or_insert(crate::group_style::GroupStyle {
                        blend: "normal".into(),
                        isolate: true,
                    })
                    .isolate = true;
                // The filters have already been sampled in native coordinates;
                // ordinary groups now preserve embedded PNGs in svg2pdf.
                s.svg_viewport = false;
                s.svg_unclipped = false;
            }
        }
        Ok(())
    }
    let mut prepared = doc.clone();
    let mut pixels = 0;
    for layer in &mut prepared.layers {
        if !layer.visible || layer.opacity <= 0. {
            continue;
        }
        let pad = layer.effects.padding() as f32;
        let crop = [-pad, -pad, doc.width as f32 + pad, doc.height as f32 + pad];
        if let Content::Vector(shapes) = &mut layer.content {
            walk(
                shapes,
                groups::translation(layer.x as f32, layer.y as f32),
                &mut pixels,
                crop,
                None,
            )?;
        }
    }
    Ok(prepared)
}
pub fn pdf(doc: &Document) -> Result<Vec<u8>, String> {
    let original = vector::export_svg(doc)?;
    let source = if check_pdf_paints(vector::internal_tree(&original)?.root(),0).is_err() {
        // PDF cannot express all SVG spread/focal/additive paints. Bake only the
        // export copy; the project and SVG retain the editable source geometry.
        let png=crate::io::png_bytes(&vector::flatten(doc)?)?;
        use base64::Engine;
        vector::svg_document(doc.width,doc.height,&format!("<image width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\"/>",doc.width,doc.height,base64::engine::general_purpose::STANDARD.encode(png)))
    } else {vector::export_svg(&prepare_pdf_images(doc)?)?};
    // svg2pdf 0.13 ne transmet pas le facteur PageOptions.dpi à la matrice
    // absolue des peintures PDF. Faire porter 96 px/pouce par le viewBox SVG
    // permet au même transform cumulé de placer géométrie, dégradés et motifs.
    let source = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">{source}</svg>",doc.width as f64*0.75,doc.height as f64*0.75,doc.width,doc.height);
    let mut options = svg2pdf::usvg::Options::default();
    static FONTS: OnceLock<Arc<svg2pdf::usvg::fontdb::Database>> = OnceLock::new();
    options.fontdb = FONTS
        .get_or_init(|| {
            let mut fonts = svg2pdf::usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone();
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let tree = svg2pdf::usvg::Tree::from_str(&source, &options)
        .map_err(|e| format!("Préparation PDF : {e}"))?;
    // Les chaînes éditables ont été préparées avec le moteur natif ; le
    // convertisseur garde ce facteur pour les effets de calque restants.
    let scale = (16_777_216.0 / (doc.width as f32 * doc.height as f32))
        .sqrt()
        .min(1.5)
        .max(0.1);
    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions {
            raster_scale: scale,
            ..Default::default()
        },
        svg2pdf::PageOptions { dpi: 72.0 },
    )
    .map_err(|e| format!("Export PDF : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::Editor, operations};
    use base64::Engine;
    use serde_json::json;
    #[test]
    fn pdf_mask_coverage_preserves_colors_affine_placement_shadow_and_source() {
        use crate::document::{Content, ShapeKind};
        for body in [
            r##"<defs><linearGradient id="p"><stop stop-color="red"/><stop offset="1" stop-color="#00cc55"/></linearGradient><mask id="m"><rect width="230" height="160" fill="url(#p)"/><circle cx="95" cy="75" r="30" fill="#2244ff"/></mask></defs><g transform="rotate(7 100 80)" mask="url(#m)"><rect x="10" y="10" width="190" height="130" fill="#ee6633"/></g>"##,
            r##"<defs><mask id="m"><rect width="230" height="160" fill="#ee8833"/><g style="mix-blend-mode:multiply"><rect x="50" y="30" width="130" height="100" fill="#22bb88"/></g></mask></defs><g transform="skewX(8)" mask="url(#m)"><rect x="10" y="10" width="190" height="130" fill="#3355cc"/></g>"##,
            r##"<defs><mask id="m"><rect width="230" height="160" fill="white"/><circle cx="100" cy="80" r="30" fill="#557733"/></mask></defs><g style="mix-blend-mode:multiply" mask="url(#m)"><rect x="10" y="10" width="190" height="130" fill="#3355cc"/></g>"##,
        ] {
            for editable in [true, false] {
                for nested in [false, true] {
                    let tmp = tempfile::tempdir().unwrap();
                    let path = tmp.path().join("pdf-mask.svg");
                    let inner = format!(
                        r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180">{body}</svg>"#
                    );
                    let source = if nested {
                        format!(
                            r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180"><image x="15" y="10" width="200" height="150" href="data:image/svg+xml;base64,{}"/></svg>"#,
                            base64::engine::general_purpose::STANDARD.encode(inner)
                        )
                    } else {
                        inner
                    };
                    std::fs::write(&path, source).unwrap();
                    let mut e = Editor::default();
                    operations::execute(
                        &mut e,
                        serde_json::from_value(
                            json!({"type":if editable{"openSvgEditable"}else{"open"},"path":path}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    if let Content::Vector(shapes) = &mut e.doc_mut().unwrap().layers[0].content {
                        shapes.insert(0,serde_json::from_value(json!({"id":10000,"type":"rect","radius":0,"x":0,"y":0,"w":240,"h":180,"rotation":0,"fill":"#bbcc99","stroke":null,"strokeWidth":0,"opacity":1})).unwrap());
                    }
                    let id = e.doc().unwrap().layers[0].id;
                    operations::execute(
                        &mut e,
                        serde_json::from_value(json!({"type":"layer","id":id,"x":7,"y":-4}))
                            .unwrap(),
                    )
                    .unwrap();
                    e.doc_mut().unwrap().layers[0].effects.shadow = Some(crate::effects::Shadow {
                        x: -18.,
                        y: 8.,
                        blur: 3.,
                        spread: 1.,
                        color: "#000000".into(),
                        opacity: 0.6,
                    });
                    let before = crate::io::project_bytes(e.doc().unwrap()).unwrap();
                    let original = vector::flatten(e.doc().unwrap()).unwrap();
                    let prepared = prepare_pdf_images(e.doc().unwrap()).unwrap();
                    let after = vector::flatten(&prepared).unwrap();
                    let mut max = 0f64;
                    let mut sum = 0f64;
                    let mut edge_pixels = 0;
                    for (p, q) in original.pixels().zip(after.pixels()) {
                        let mut pixel = 0f64;
                        for c in 0..4 {
                            let d = if c == 3 {
                                p[c].abs_diff(q[c]) as f64
                            } else {
                                (p[c] as f64 * p[3] as f64 / 255.
                                    - q[c] as f64 * q[3] as f64 / 255.)
                                    .abs()
                            };
                            max = max.max(d);
                            sum += d;
                            pixel = pixel.max(d);
                        }
                        if pixel > 8. {
                            edge_pixels += 1;
                        }
                    }
                    assert!(
                        max <= if !editable||nested{16.}else{4.} && edge_pixels<10 && sum / (original.as_raw().len() as f64) < 0.04,
                        "editable {editable}, nested {nested}, max {max}, edges {edge_pixels}, mean {}",
                        sum / original.as_raw().len() as f64
                    );
                    let Content::Vector(shapes) = &prepared.layers[0].content else {
                        panic!()
                    };
                    fn masked(
                        shapes: &[crate::document::Shape],
                    ) -> Option<&crate::document::Shape> {
                        shapes.iter().find_map(|s| {
                            if s.mask.is_some() {
                                Some(s)
                            } else if let ShapeKind::Group { children, .. } = &s.kind {
                                masked(children)
                            } else {
                                None
                            }
                        })
                    }
                    let g = masked(shapes).unwrap();
                    assert_eq!(
                        g.mask.as_ref().unwrap().mode,
                        crate::svg_mask::MaskMode::Alpha
                    );
                    assert!(matches!(g.kind, ShapeKind::Group { .. }));
                    assert_eq!(before, crate::io::project_bytes(e.doc().unwrap()).unwrap());
                    assert!(pdf(e.doc().unwrap()).unwrap().starts_with(b"%PDF-"));
                }
            }
        }
    }
    #[test]
    fn pdf_bakes_unrepresentable_paints_without_mutating_editable_source() {
        let mut e = Editor::default();
        operations::execute(
            &mut e,
            serde_json::from_value(
                json!({"type":"new","width":200,"height":120,"background":null}),
            )
            .unwrap(),
        )
        .unwrap();
        operations::execute(
            &mut e,
            serde_json::from_value(json!({"type":"addLayer","kind":"vector"})).unwrap(),
        )
        .unwrap();
        let id = e.active_hint.unwrap();
        for gradient in [
            r##"<linearGradient id="p" spreadMethod="repeat"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>"##,
            r##"<radialGradient id="p" fr=".1"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient>"##,
        ] {
            let source = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120"><defs>{gradient}</defs><rect width="200" height="120" fill="url(#p)"/></svg>"##
            );
            operations::execute(&mut e,serde_json::from_value(json!({"type":"shape","id":id,"shape":{"id":7,"type":"svg","source":source,"intrinsicW":200,"intrinsicH":120,"x":0,"y":0,"w":200,"h":120,"rotation":0,"fill":null,"stroke":null,"strokeWidth":0,"opacity":1}})).unwrap()).unwrap();
            let before = vector::flatten(e.doc().unwrap()).unwrap();
            let history = e.info().history;
            let bytes=pdf(e.doc().unwrap()).unwrap();
            assert!(bytes.starts_with(b"%PDF-"));
            assert!(String::from_utf8_lossy(&bytes).contains("/Image"));
            assert_eq!(before, vector::flatten(e.doc().unwrap()).unwrap());
            assert_eq!(history, e.info().history);
            assert!(vector::export_svg(e.doc().unwrap())
                .unwrap()
                .contains("<svg"));
        }
    }
    #[test]
    fn optimized_svg_preserves_render_and_pdf_contains_vector_text() {
        let mut e = Editor::default();
        let run =
            |e: &mut Editor, v| operations::execute(e, serde_json::from_value(v).unwrap()).unwrap();
        run(
            &mut e,
            json!({"type":"new","width":320,"height":200,"background":"#ffffff"}),
        );
        run(&mut e, json!({"type":"addLayer","kind":"vector"}));
        let id = e.active_hint.unwrap();
        run(
            &mut e,
            json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":6,"x":20.12345,"y":20,"w":100,"h":70,"fill":"#2080e0","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}}),
        );
        run(
            &mut e,
            json!({"type":"shape","id":id,"shape":{"id":0,"type":"text","text":"Prisme PDF","fontFamily":"Segoe UI","fontSize":22,"bold":false,"italic":false,"x":20,"y":115,"w":150,"h":30,"fill":"#202020","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}}),
        );
        let source = optimized_svg(e.doc().unwrap(), 5).unwrap();
        assert!(source.contains("<text"));
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        let after = vector::render_svg(&source, 320, 200).unwrap();
        assert!(before
            .as_raw()
            .iter()
            .zip(after.as_raw())
            .all(|(a, b)| a.abs_diff(*b) <= 1));
        let bytes = pdf(e.doc().unwrap()).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(String::from_utf8_lossy(&bytes).contains("/Type /Font"));
        let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("phase3-vector.pdf"), bytes).unwrap();
        std::fs::write(
            work.join("phase3-vector.png"),
            crate::io::png_bytes(&before).unwrap(),
        )
        .unwrap();
        std::fs::write(work.join("phase3-optimized.svg"), source).unwrap();
    }
}
