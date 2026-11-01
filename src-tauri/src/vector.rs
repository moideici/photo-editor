//! SVG généré par Prisme, sans insertion de code HTML dans l'interface.
use crate::document::{Content, Document, Layer, Shape, ShapeKind};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::RgbaImage;
use resvg::{tiny_skia, usvg};

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn options() -> usvg::Options<'static> {
    let mut opts = usvg::Options::default();
    opts.fontdb = crate::fonts::database();
    // Aucune résolution de fichiers externes lors de l'import d'un SVG.
    opts.image_href_resolver.resolve_string = Box::new(|_, _| None);
    opts
}

pub fn parse(source: &str) -> Result<usvg::Tree, String> {
    if source.len() > 16 * 1024 * 1024 {
        return Err("SVG trop volumineux (16 Mio maximum)".into());
    }
    usvg::Tree::from_str(source, &options()).map_err(|e| format!("SVG invalide : {e}"))
}

pub fn internal_tree(source: &str) -> Result<usvg::Tree, String> {
    usvg::Tree::from_str(source, &options()).map_err(|e| format!("SVG interne invalide : {e}"))
}

pub fn shape_svg(s: &Shape) -> String {
    let mut definitions = String::new();
    let text_inverse = if let ShapeKind::Text {
        text, font_size, ..
    } = &s.kind
    {
        let lines = text.lines().count().max(1) as f32;
        let chars = text
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        let sx = s.w
            / s.text_path
                .as_ref()
                .map(|p| p.intrinsic_w)
                .unwrap_or((chars * font_size * 0.6).max(1.0));
        let sy = s.h
            / s.text_path
                .as_ref()
                .map(|p| p.intrinsic_h)
                .unwrap_or((font_size * 1.3 * lines).max(1.0));
        Some([1. / sx, 0., 0., 1. / sy, -s.x / sx, -s.y / sy])
    } else {
        None
    };
    let mut paint = |gradient: Option<&crate::paint::Gradient>,
                     pattern: Option<&crate::paint::Pattern>,
                     solid: Option<&str>| {
        if let Some(p) = pattern {
            let mut transform = p.transform;
            if matches!(p.units, crate::paint::Units::ObjectBoundingBox) {
                if let Ok([x, y, w, h]) = crate::paint::bounds(s) {
                    transform = crate::groups::multiply([w, 0., 0., h, x, y], transform);
                }
            }
            if let Some(m) = text_inverse {
                transform = crate::groups::multiply(m, transform);
            }
            let id = crate::paint::next_id();
            definitions += &p.definition(&id, transform);
            format!("url(#{id})")
        } else if let Some(g) = gradient {
            let mut g = g.clone();
            if matches!(g.units, crate::paint::Units::UserSpaceOnUse) {
                if let Some(m) = text_inverse {
                    g.transform = crate::groups::multiply(m, g.transform)
                }
            }
            let id = crate::paint::next_id();
            definitions += &g.definition(&id);
            format!("url(#{id})")
        } else {
            escape(solid.unwrap_or("none"))
        }
    };
    let fill = paint(
        s.fill_gradient.as_ref(),
        s.fill_pattern.as_ref(),
        s.fill.as_deref(),
    );
    let stroke = paint(
        s.stroke_gradient.as_ref(),
        s.stroke_pattern.as_ref(),
        s.stroke.as_deref(),
    );
    let clip_attr = if let Some(clip) = &s.clip {
        let id = crate::paint::next_id();
        let mut clip = clip.clone();
        if let Some(f) = &s.filters {
            clip.apply(crate::svg_filter::inverse(f.transform));
        }
        definitions += &clip.definition(&id);
        format!("clip-path=\"url(#{id})\"")
    } else {
        String::new()
    };
    let style = format!(
        "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" opacity=\"{}\"",
        fill, stroke, s.stroke_width, s.opacity
    );
    let mask_attr = if let Some(mask) = &s.mask {
        let id = crate::paint::next_id();
        let mut mask = mask.clone();
        if let Some(f) = &s.filters {
            mask.apply(crate::svg_filter::inverse(f.transform));
        }
        definitions += &mask.definition(&id);
        format!("mask=\"url(#{id})\"")
    } else {
        String::new()
    };
    let body=match &s.kind {
        ShapeKind::Group {children,intrinsic_w,intrinsic_h}=>{
            let content=children.iter().map(shape_svg).collect::<String>();
            let content=if s.svg_viewport {
                let source=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{intrinsic_w}\" height=\"{intrinsic_h}\">{content}</svg>");
                let body=format!("<image width=\"{intrinsic_w}\" height=\"{intrinsic_h}\" preserveAspectRatio=\"none\" href=\"data:image/svg+xml;base64,{}\"/>",STANDARD.encode(&source));
                if !s.svg_unclipped && overflowing_viewport(&source).is_some(){viewport_clip(&mut definitions,0.,0.,*intrinsic_w,*intrinsic_h,body)}else{body}
            }else{content};
            let attrs=format!("opacity=\"{}\" style=\"{}\" {clip_attr} {mask_attr}",s.opacity,s.group_style.as_ref().map(|s|s.svg()).unwrap_or_default());
            let (content,attrs)=if let Some(f)=&s.filters {(f.body(&mut definitions,content,&attrs),String::new())}else{(content,attrs)};
            format!("<g {attrs} transform=\"translate({} {}) scale({} {})\">{content}</g>",s.x,s.y,s.w/intrinsic_w,s.h/intrinsic_h)
        },
        ShapeKind::Path {contours,even_odd}=>format!("<path d=\"{}\" fill-rule=\"{}\" stroke-linecap=\"round\" {style}/>",crate::paths::data(s,contours),if *even_odd {"evenodd"} else {"nonzero"}),
        ShapeKind::Rect {radius}=>format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" {style}/>",s.x,s.y,s.w.max(0.0),s.h.max(0.0),radius),
        ShapeKind::Ellipse=>format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" {style}/>",s.x+s.w/2.0,s.y+s.h/2.0,s.w.abs()/2.0,s.h.abs()/2.0),
        ShapeKind::Line=>format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-linecap=\"round\" {style}/>",s.x,s.y,s.x+s.w,s.y+s.h),
        ShapeKind::Text {text,font_family,font_size,bold,italic}=> {
            let font_attrs=crate::fonts::attributes(s);
            if let Some(p)=&s.text_path {
                let id=crate::paint::next_id();
                let percent=if p.percent{"%"}else{""};
                let sx=s.w/p.intrinsic_w;let sy=s.h/p.intrinsic_h;
                format!("<g transform=\"translate({} {}) scale({sx} {sy})\"><defs><path id=\"{id}\" d=\"{}\"/></defs><text font-family=\"{}\" font-size=\"{font_size}\" font-weight=\"{}\" font-style=\"{}\" text-anchor=\"{}\" dy=\"{}\" {font_attrs} {style}><textPath href=\"#{id}\" startOffset=\"{}{percent}\">{}</textPath></text></g>",s.x,s.y,p.data(),escape(font_family),if *bold{"bold"}else{"normal"},if *italic{"italic"}else{"normal"},p.anchor,p.normal_offset,p.start_offset,escape(text))
            } else {
            let line_count=text.lines().count().max(1) as f32;
            let chars=text.lines().map(|l|l.chars().count()).max().unwrap_or(1).max(1) as f32;
            let sx=s.w/(chars*font_size*0.6).max(1.0);
            let sy=s.h/(font_size*1.3*line_count).max(1.0);
            let lines=text.lines().enumerate().map(|(i,line)|format!("<tspan x=\"0\" dy=\"{}\">{}</tspan>",if i==0 {0.0} else {font_size*1.25},escape(line))).collect::<String>();
            format!("<g transform=\"translate({} {}) scale({sx} {sy})\"><text x=\"0\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" {font_attrs} {style}>{lines}</text></g>",s.x,s.y,font_size,escape(font_family),font_size,if *bold {"bold"} else {"normal"},if *italic {"italic"} else {"normal"})
            }
        },
        ShapeKind::Image {asset}=>format!("<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" style=\"image-rendering:{}\" opacity=\"{}\" href=\"{}\"/>",s.x,s.y,s.w,s.h,escape(&asset.rendering),s.opacity,asset.url()),
        ShapeKind::Svg {source,..}=>format!("<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" opacity=\"{}\" href=\"data:image/svg+xml;base64,{}\"/>",s.x,s.y,s.w,s.h,s.opacity,STANDARD.encode(source)),
    };
    // An embedded SVG has its own viewport. resvg renders nested SVG trees
    // onto the parent surface; explicitly bound that viewport so outside
    // geometry cannot leak into neighbouring objects or letterbox margins.
    let body = match &s.kind {
        ShapeKind::Svg { source, .. } if overflowing_viewport(source).is_some() => {
            let [intrinsic_w, intrinsic_h] = overflowing_viewport(source).unwrap();
            let scale = (s.w / intrinsic_w).min(s.h / intrinsic_h);
            let (w, h) = (intrinsic_w * scale, intrinsic_h * scale);
            let (x, y) = (s.x + (s.w - w) / 2., s.y + (s.h - h) / 2.);
            viewport_clip(&mut definitions, x, y, w, h, body)
        }
        ShapeKind::Image { asset }
            if asset.mime == "image/svg+xml"
                && asset
                    .bytes()
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .and_then(|s| overflowing_viewport(&s))
                    .is_some() =>
        {
            viewport_clip(&mut definitions, s.x, s.y, s.w, s.h, body)
        }
        _ => body,
    };
    format!(
        "<g transform=\"{} rotate({} {} {})\"><defs>{definitions}</defs>{body}</g>",
        s.transform
            .map(|m| format!(
                "matrix({} {} {} {} {} {})",
                m[0], m[1], m[2], m[3], m[4], m[5]
            ))
            .unwrap_or_default(),
        s.rotation,
        s.x + s.w / 2.0,
        s.y + s.h / 2.0
    )
}

fn viewport_clip(defs: &mut String, x: f32, y: f32, w: f32, h: f32, body: String) -> String {
    let id = crate::paint::next_id();
    *defs += &format!(
        "<clipPath id=\"{id}\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/></clipPath>"
    );
    format!("<g clip-path=\"url(#{id})\">{body}</g>")
}
fn overflowing_viewport(source: &str) -> Option<[f32; 2]> {
    let tree = parse(source).ok()?;
    let r = tree.root().abs_layer_bounding_box();
    let (w, h) = (tree.size().width(), tree.size().height());
    (r.left() < -0.001 || r.top() < -0.001 || r.right() > w + 0.001 || r.bottom() > h + 0.001)
        .then_some([w, h])
}
pub(crate) fn viewport_overflows(s: &Shape) -> bool {
    if s.svg_unclipped {
        return false;
    }
    if let ShapeKind::Group {
        children,
        intrinsic_w,
        intrinsic_h,
    } = &s.kind
    {
        let body = children.iter().map(shape_svg).collect::<String>();
        let source=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{intrinsic_w}\" height=\"{intrinsic_h}\">{body}</svg>");
        overflowing_viewport(&source).is_some()
    } else {
        false
    }
}

pub fn svg_document(w: u32, h: u32, body: &str) -> String {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{body}</svg>")
}

pub fn rasterize(shapes: &[Shape], w: u32, h: u32) -> Result<RgbaImage, String> {
    rasterize_at(shapes, w, h, 0, 0)
}

fn rasterize_at(shapes: &[Shape], w: u32, h: u32, x: i32, y: i32) -> Result<RgbaImage, String> {
    let source = svg_document(
        w,
        h,
        &format!(
            "<g transform=\"translate({x} {y})\">{}</g>",
            shapes.iter().map(shape_svg).collect::<String>()
        ),
    );
    render_svg(&source, w, h)
}

pub fn render_svg(source: &str, w: u32, h: u32) -> Result<RgbaImage, String> {
    if w == 0 || h == 0 || w > 32768 || h > 32768 || w as u64 * h as u64 > 64 * 1024 * 1024 {
        return Err("Surface de travail trop grande pour ce rendu".into());
    }
    let tree = usvg::Tree::from_str(source, &options())
        .map_err(|e| format!("Rendu SVG invalide : {e}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("Image vectorielle trop grande")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let mut bytes = pixmap.take();
    for p in bytes.chunks_exact_mut(4) {
        if p[3] > 0 {
            for c in 0..3 {
                p[c] = ((p[c] as u32 * 255 + p[3] as u32 / 2) / p[3] as u32).min(255) as u8;
            }
        }
    }
    RgbaImage::from_raw(w, h, bytes).ok_or("Rendu vectoriel invalide".into())
}

pub fn content_surface(layer: &Layer, w: u32, h: u32) -> Result<(RgbaImage, i32, i32), String> {
    match &layer.content {
        Content::Raster(img) => Ok(((**img).clone(), layer.x, layer.y)),
        Content::Vector(shapes) => {
            let (ww, hh, pad) = style_dimensions(&layer.effects, w, h)?;
            if pad == 0 && layer.x == 0 && layer.y == 0 {
                return Ok((rasterize(shapes, w, h)?, 0, 0));
            }
            Ok((
                rasterize_at(
                    shapes,
                    ww,
                    hh,
                    layer.x.saturating_add(pad as i32),
                    layer.y.saturating_add(pad as i32),
                )?,
                -(pad as i32),
                -(pad as i32),
            ))
        }
    }
}

pub fn style_dimensions(
    effects: &crate::effects::LayerEffects,
    w: u32,
    h: u32,
) -> Result<(u32, u32, u32), String> {
    effects.validate()?;
    let pad = effects.padding();
    let ww = w.saturating_add(pad.saturating_mul(2));
    let hh = h.saturating_add(pad.saturating_mul(2));
    if ww > 32768 || hh > 32768 || ww as u64 * hh as u64 > 64 * 1024 * 1024 {
        return Err("Les effets dépassent la surface de travail disponible ; réduisez leur rayon ou leur décalage".into());
    }
    Ok((ww, hh, pad))
}

fn content_svg(layer: &Layer) -> Result<String, String> {
    match &layer.content {
        Content::Vector(shapes) => Ok(shapes.iter().map(shape_svg).collect::<String>()),
        Content::Raster(img) => Ok(format!(
            "<image width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\"/>",
            img.width(),
            img.height(),
            STANDARD.encode(crate::io::png_bytes(img)?)
        )),
    }
}

fn styled_svg(layer: &Layer, w: u32, h: u32) -> Result<(String, u32, u32, u32), String> {
    let (ww, hh, pad) = style_dimensions(&layer.effects, w, h)?;
    let id = format!("fx{}", layer.id);
    let region = [
        -(pad as f32) - layer.x as f32,
        -(pad as f32) - layer.y as f32,
        ww as f32,
        hh as f32,
    ];
    let body = format!(
        "<defs>{}</defs><g transform=\"translate({} {})\" filter=\"url(#{id})\">{}</g>",
        layer.effects.svg(&id, region),
        layer.x as f64 + pad as f64,
        layer.y as f64 + pad as f64,
        content_svg(layer)?
    );
    Ok((svg_document(ww, hh, &body), ww, hh, pad))
}

pub fn layer_surface(layer: &Layer, w: u32, h: u32) -> Result<(RgbaImage, i32, i32), String> {
    if layer.effects.is_empty() {
        return content_surface(layer, w, h);
    }
    let (svg, ww, hh, pad) = styled_svg(layer, w, h)?;
    Ok((render_svg(&svg, ww, hh)?, -(pad as i32), -(pad as i32)))
}

pub fn flatten(doc: &Document) -> Result<RgbaImage, String> {
    crate::preview_jobs::checkpoint()?;
    let mut out = RgbaImage::new(doc.width, doc.height);
    for layer in doc.layers.iter().filter(|l| l.visible && l.opacity > 0.0) {
        crate::preview_jobs::checkpoint()?;
        let (pixels, x, y) = layer_surface(layer, doc.width, doc.height)?;
        crate::raster::composite(&mut out, &pixels, x, y, layer.opacity, layer.blend);
    }
    crate::preview_jobs::checkpoint()?;
    Ok(out)
}

pub fn export_svg(doc: &Document) -> Result<String, String> {
    fn has_group_blend(shapes: &[Shape]) -> bool {
        shapes.iter().any(|s| {
            s.group_style
                .as_ref()
                .is_some_and(|style| style.blend != "normal")
                || matches!(&s.kind,ShapeKind::Group{children,..} if has_group_blend(children))
        })
    }
    let mut body = String::new();
    for layer in doc.layers.iter().filter(|l| l.visible) {
        let (content, x, y) = if layer.effects.is_empty() {
            (content_svg(layer)?, layer.x, layer.y)
        } else {
            let (svg, _, _, pad) = styled_svg(layer, doc.width, doc.height)?;
            (svg, -(pad as i32), -(pad as i32))
        };
        let mode = serde_json::to_value(layer.blend)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let mode = if mode == "add" { "plus-lighter" } else { &mode };
        // Le rendu de chaque calque est autonome. Isoler les fusions des groupes
        // avant la fusion du calque ; garder le chemin SVG historique sinon,
        // pour éviter une surface intermédiaire et un arrondi alpha superflus.
        let isolate = if matches!(&layer.content,Content::Vector(shapes) if has_group_blend(shapes))
        {
            ";isolation:isolate"
        } else {
            ""
        };
        body.push_str(&format!("<g opacity=\"{}\" transform=\"translate({} {})\" style=\"mix-blend-mode:{}{isolate}\"><title>{}</title>{content}</g>",layer.opacity,x,y,mode,escape(&layer.name)));
    }
    Ok(svg_document(doc.width, doc.height, &body))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rectangle_render_and_svg_escape() {
        let s = Shape {
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
            transform: None,
            id: 1,
            kind: ShapeKind::Rect { radius: 0.0 },
            x: 2.0,
            y: 2.0,
            w: 4.0,
            h: 4.0,
            rotation: 0.0,
            fill: Some("#ff0000".into()),
            stroke: None,
            stroke_width: 0.0,
            opacity: 0.5,
        };
        let img = rasterize(&[s], 8, 8).unwrap();
        assert_eq!(img.get_pixel(3, 3).0, [255, 0, 0, 128]);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(escape("<&\""), "&lt;&amp;&quot;");
    }
}
