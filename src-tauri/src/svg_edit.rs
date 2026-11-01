//! Décomposition explicite des SVG simples. Aucun résultat partiel n'est installé.
use crate::document::{Shape, ShapeKind};
use crate::paths::{Contour, Node};
use resvg::{
    tiny_skia::{self, PathSegment, Point},
    usvg,
};

const LIMIT: usize = 20_000;
thread_local! {static FILTER_RESOURCE:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub(crate) fn in_filter_resource<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            FILTER_RESOURCE.with(|b| b.set(self.0));
        }
    }
    let _scope = Restore(FILTER_RESOURCE.with(|b| b.replace(true)));
    f()
}
pub(crate) fn filter_resource_children(
    group: &usvg::Group,
    depth: usize,
    nodes: &mut usize,
    total: &mut usize,
) -> Result<Vec<Shape>, String> {
    in_filter_resource(|| mask_children(group, depth, nodes, total))
}
fn unsupported(why: &str) -> String {
    format!("Décomposition SVG impossible : {why}. Le bloc original est conservé.")
}

/// Les courbes/quadratiques/arcs sont normalisés par usvg, les traits deviennent
/// des surfaces avant transformation (caps, joins et pointillés conservés).
pub fn decompose(original: &Shape) -> Result<Vec<Shape>, String> {
    let source = match &original.kind {
        ShapeKind::Svg { source, .. } => std::borrow::Cow::Borrowed(source.as_str()),
        ShapeKind::Image { asset } if asset.mime == "image/svg+xml" => {
            std::borrow::Cow::Owned(String::from_utf8(asset.bytes()?).map_err(|e| e.to_string())?)
        }
        _ => return Err("Sélectionnez un bloc SVG ou une image SVG intégrée".into()),
    };
    let notices = crate::embedded::diagnostics(&source)?;
    if !notices.is_empty() {
        return Err(unsupported(&notices.join(" ; ")));
    }
    let tree = crate::vector::parse(&source)?;
    crate::svg_mask::check_normalized_masks(&source, &tree)?;
    crate::svg_filter::check_source(&source, &tree)?;
    let sw = tree.size().width();
    let sh = tree.size().height();
    if original.w <= 0.0 || original.h <= 0.0 {
        return Err(unsupported("dimensions nulles"));
    }
    let scale = (original.w / sw).min(original.h / sh);
    let (sx, sy) = if matches!(original.kind, ShapeKind::Image { .. }) {
        (original.w / sw, original.h / sh)
    } else {
        (scale, scale)
    };
    let offset = [
        original.x + (original.w - sw * sx) / 2.,
        original.y + (original.h - sh * sy) / 2.,
    ];
    let angle = original.rotation.to_radians();
    let center = [original.x + original.w / 2., original.y + original.h / 2.];
    let (sn, cs) = angle.sin_cos();
    let outer = [
        cs * sx,
        sn * sx,
        -sn * sy,
        cs * sy,
        center[0] + cs * (offset[0] - center[0]) - sn * (offset[1] - center[1]),
        center[1] + sn * (offset[0] - center[0]) + cs * (offset[1] - center[1]),
    ];
    let mut overflow = false;
    let filtered_viewport = !tree.filters().is_empty();
    let mut out = crate::svg_filter::with_definitions(&tree, || {
        convert_group(
            tree.root(),
            crate::groups::IDENTITY,
            if filtered_viewport {
                crate::groups::IDENTITY
            } else {
                outer
            },
            [sw, sh],
            0,
            &mut 0,
            &mut 0,
            &mut overflow,
        )
    })?;
    if out.is_empty() {
        return Err(unsupported("aucune surface visible"));
    }
    if filtered_viewport {
        // Filters are evaluated in their source SVG viewport by resvg. Keep
        // that rendering boundary as an editable frame, rather than applying
        // only a final clip on a larger document surface.
        let mut frame = crate::clip::base();
        frame.kind = ShapeKind::Group {
            children: out,
            intrinsic_w: sw,
            intrinsic_h: sh,
        };
        frame.fill = None;
        frame.x = 0.;
        frame.y = 0.;
        frame.w = sw;
        frame.h = sh;
        frame.svg_viewport = true;
        frame.opacity = original.opacity;
        crate::groups::apply(&mut frame, outer);
        out = vec![frame];
    } else if overflow {
        let mut wrapped = wrap_group(out, 1., None)?;
        crate::clip::attach(
            &mut wrapped,
            crate::clip::Clip::rectangle(0., 0., sw, sh, outer),
        );
        out = vec![wrapped];
    }
    // Garder l'opacité d'ensemble sur son propre groupe : elle ne se distribue
    // pas sur des enfants qui se recouvrent, même après une décomposition.
    if original.opacity != 1.0 && !filtered_viewport {
        out = vec![wrap_group(out, original.opacity, None)?];
    }
    if let Some(matrix) = original.transform {
        for s in &mut out {
            crate::groups::apply(s, matrix);
        }
    }
    let mut next = 0;
    crate::groups::assign_ids(&mut out, &mut || {
        next += 1;
        next
    });
    crate::groups::validate(&out)?;
    Ok(out)
}
pub(crate) fn wrap_group(
    mut children: Vec<Shape>,
    opacity: f32,
    style: Option<crate::group_style::GroupStyle>,
) -> Result<Shape, String> {
    let tree = crate::vector::internal_tree(&crate::vector::svg_document(
        1,
        1,
        &children
            .iter()
            .map(crate::vector::shape_svg)
            .collect::<String>(),
    ))?;
    let r = tree.root().abs_stroke_bounding_box();
    let (x, y, w, h) = (r.x(), r.y(), r.width().max(0.001), r.height().max(0.001));
    for s in &mut children {
        crate::groups::apply(s, crate::groups::translation(-x, -y));
    }
    Ok(Shape {
        font_options: None,
        text_path: None,
        svg_unclipped: false,
        svg_viewport: false,
        filters: None,
        mask: None,
        clip: None,
        group_style: style,
        fill_pattern: None,
        stroke_pattern: None,
        fill_gradient: None,
        stroke_gradient: None,
        transform: None,
        id: 0,
        kind: ShapeKind::Group {
            children,
            intrinsic_w: w,
            intrinsic_h: h,
        },
        x,
        y,
        w,
        h,
        rotation: 0.,
        fill: None,
        stroke: None,
        stroke_width: 0.,
        opacity,
    })
}
fn convert_group(
    group: &usvg::Group,
    parent: crate::groups::Matrix,
    outer: crate::groups::Matrix,
    viewport: [f32; 2],
    depth: usize,
    nodes: &mut usize,
    total: &mut usize,
    overflow: &mut bool,
) -> Result<Vec<Shape>, String> {
    if depth > 64 {
        return Err(unsupported("plus de 64 niveaux de groupes SVG"));
    }
    let matrix = crate::groups::multiply(parent, crate::paint::matrix(group.transform()));
    let filtered_frame =
        (!group.filters().is_empty()).then(|| crate::groups::multiply(outer, matrix));
    // Preserve the filter's local coordinate frame. Baking an affine matrix
    // into paths then undoing it changes resvg's integer filter offsets near
    // exact scale boundaries (e.g. 9 becomes 8.999999 -> 8).
    let (matrix, outer) = if filtered_frame.is_some() {
        (crate::groups::IDENTITY, crate::groups::IDENTITY)
    } else {
        (matrix, outer)
    };
    let filters = crate::svg_filter::FilterChain::imported(
        group,
        crate::groups::multiply(outer, matrix),
        depth,
        nodes,
        total,
    )?;
    if filters.is_some() {
        *overflow = true;
    }
    let mut children = Vec::new();
    for node in group.children() {
        let mut result = match node {
            usvg::Node::Group(g) => convert_group(
                g,
                matrix,
                outer,
                viewport,
                depth + 1,
                nodes,
                total,
                overflow,
            )?,
            usvg::Node::Text(t) => convert_group(
                t.flattened(),
                matrix,
                outer,
                viewport,
                depth + 1,
                nodes,
                total,
                overflow,
            )?,
            usvg::Node::Path(p) if p.is_visible() => {
                let r = p.abs_stroke_bounding_box();
                if r.left() < -0.001
                    || r.top() < -0.001
                    || r.right() > viewport[0] + 0.001
                    || r.bottom() > viewport[1] + 0.001
                {
                    *overflow = true;
                }
                let result = surfaces(vec![(p, matrix)], outer, None, true, nodes)?;
                *total += result.len();
                if *total > 1000 {
                    return Err(unsupported("plus de 1 000 surfaces"));
                }
                result
            }
            usvg::Node::Path(_) => Vec::new(),
            usvg::Node::Image(image) if image.is_visible() => {
                let r = image.abs_bounding_box();
                if r.left() < 0.
                    || r.top() < 0.
                    || r.right() > viewport[0]
                    || r.bottom() > viewport[1]
                {
                    *overflow = true;
                }
                *total += 1;
                if *total > 1000 {
                    return Err(unsupported("plus de 1 000 éléments"));
                }
                let frame = crate::groups::multiply(outer, matrix);
                if FILTER_RESOURCE.with(|b| b.get())
                    && matches!(image.kind(), usvg::ImageKind::SVG(_))
                {
                    let usvg::ImageKind::SVG(tree) = image.kind() else {
                        unreachable!()
                    };
                    let children = crate::svg_filter::with_definitions(tree, || {
                        mask_children(tree.root(), depth + 1, nodes, total)
                    })?;
                    if children.is_empty() {
                        return Err(unsupported("image SVG de filtre sans contenu conservable"));
                    }
                    let (w, h) = (tree.size().width(), tree.size().height());
                    let mut s = crate::clip::base();
                    s.fill = None;
                    s.w = w;
                    s.h = h;
                    s.svg_viewport = true;
                    s.svg_unclipped = true;
                    s.kind = ShapeKind::Group {
                        children,
                        intrinsic_w: w,
                        intrinsic_h: h,
                    };
                    crate::groups::apply(&mut s, frame);
                    vec![s]
                } else {
                    vec![crate::embedded::imported(image, frame)?]
                }
            }
            usvg::Node::Image(_) => Vec::new(),
        };
        children.append(&mut result);
    }
    if children.is_empty() && filters.is_none() {
        return Ok(children);
    }
    // Source-independent generators (flood/turbulence) need a harmless frame
    // even when the SVG has no painted source geometry.
    if children.is_empty() {
        let f = filters.as_ref().unwrap();
        let [x, y, w, h] = f.filters[0].region;
        let mut frame = crate::clip::base();
        frame.kind = ShapeKind::Rect { radius: 0. };
        frame.x = x;
        frame.y = y;
        frame.w = w;
        frame.h = h;
        frame.opacity = 0.;
        crate::groups::apply(&mut frame, f.transform);
        children.push(frame);
    }
    let clip = group
        .clip_path()
        .map(|c| {
            crate::clip::Clip::imported(c, crate::groups::multiply(outer, matrix), 0, nodes, total)
        })
        .transpose()?;
    let style = crate::group_style::GroupStyle::imported(group);
    let mask = group
        .mask()
        .map(|m| {
            crate::svg_mask::VectorMask::imported(
                m,
                crate::groups::multiply(outer, matrix),
                depth + 1,
                nodes,
                total,
            )
        })
        .transpose()?;
    if group.opacity().get() != 1.
        || !style.neutral()
        || clip.is_some()
        || mask.is_some()
        || filters.is_some()
    {
        let mut wrapped = if let Some(f) = &filters {
            // Keep the SVG origin, including the exact affine translation.
            // Recentring then undoing the translation can shift feTile's
            // integer sampling grid by a pixel through f32 cancellation.
            let [x, y, w, h] = f.filters.last().unwrap().region;
            let (w, h) = ((x + w).max(1.), (y + h).max(1.));
            let mut s = crate::clip::base();
            s.kind = ShapeKind::Group {
                children,
                intrinsic_w: w,
                intrinsic_h: h,
            };
            s.w = w;
            s.h = h;
            s.fill = None;
            s.opacity = group.opacity().get();
            s.group_style = Some(style);
            s
        } else {
            wrap_group(children, group.opacity().get(), Some(style))?
        };
        if let Some(filters) = filters {
            crate::svg_filter::attach(&mut wrapped, filters);
        }
        if let Some(clip) = clip {
            crate::clip::attach(&mut wrapped, clip);
        }
        if let Some(mask) = mask {
            crate::svg_mask::attach(&mut wrapped, mask);
        }
        if let Some(frame) = filtered_frame {
            crate::groups::apply(&mut wrapped, frame);
        }
        Ok(vec![wrapped])
    } else {
        Ok(children)
    }
}
pub(crate) fn mask_children(
    group: &usvg::Group,
    depth: usize,
    nodes: &mut usize,
    total: &mut usize,
) -> Result<Vec<Shape>, String> {
    convert_group(
        group,
        crate::groups::IDENTITY,
        crate::groups::IDENTITY,
        [f32::MAX, f32::MAX],
        depth,
        nodes,
        total,
        &mut false,
    )
}

pub fn pattern_children(group: &usvg::Group) -> Result<Vec<Shape>, String> {
    thread_local! {static DEPTH:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
    struct Scope;impl Drop for Scope{fn drop(&mut self){DEPTH.with(|d|d.set(d.get()-1));}}
    DEPTH.with(|d|{if d.get()>=8{return Err(unsupported("plus de huit niveaux de motifs"));}d.set(d.get()+1);Ok(())})?;
    let _scope=Scope;
    mask_children(group,0,&mut 0,&mut 0)
}
fn surfaces(
    paths: Vec<(&usvg::Path, crate::groups::Matrix)>,
    outer: crate::groups::Matrix,
    shape_transform: Option<crate::groups::Matrix>,
    allow_patterns: bool,
    count: &mut usize,
) -> Result<Vec<Shape>, String> {
    let mut out = Vec::new();
    for (p, transform) in paths {
        if p.rendering_mode() != usvg::ShapeRendering::GeometricPrecision {
            return Err(unsupported("rendu crispEdges/optimizeSpeed"));
        }
        if p.data().len() > LIMIT {
            return Err(unsupported("plus de 20 000 segments dans une forme"));
        }
        let mut paints = Vec::new();
        if let Some(fill) = p.fill() {
            if fill.opacity().get() > 0.0 {
                paints.push((
                    p.data().clone(),
                    fill.paint(),
                    fill.opacity().get(),
                    fill.rule() == usvg::FillRule::EvenOdd,
                ));
            }
        }
        if let Some(stroke) = p.stroke() {
            if stroke.opacity().get() > 0.0 {
                let mut style = stroke.to_tiny_skia();
                let resolution = tiny_skia::PathStroker::compute_resolution_scale(
                    &tiny_skia::Transform::from_row(
                        transform[0],
                        transform[1],
                        transform[2],
                        transform[3],
                        transform[4],
                        transform[5],
                    ),
                )
                .max(1.0)
                    * tiny_skia::PathStroker::compute_resolution_scale(
                        &tiny_skia::Transform::from_row(
                            outer[0], outer[1], outer[2], outer[3], outer[4], outer[5],
                        ),
                    )
                    .max(1.0);
                let dashed = if let Some(dash) = style.dash.take() {
                    Some(
                        p.data()
                            .dash(&dash, resolution)
                            .ok_or_else(|| unsupported("pointillés trop complexes"))?,
                    )
                } else {
                    None
                };
                let outlined = dashed
                    .as_ref()
                    .unwrap_or(p.data())
                    .stroke(&style, resolution)
                    .ok_or_else(|| unsupported("contour trop complexe"))?;
                paints.push((outlined, stroke.paint(), stroke.opacity().get(), false));
            }
        }
        if p.paint_order() == usvg::PaintOrder::StrokeAndFill {
            paints.reverse();
        }
        for (path, fill, opacity, even_odd) in paints {
            let (fill, gradient, pattern) = crate::paint::imported(
                fill,
                crate::groups::multiply(outer, transform),
                allow_patterns,
            )?;
            let path = path
                .transform(tiny_skia::Transform::from_row(
                    transform[0],
                    transform[1],
                    transform[2],
                    transform[3],
                    transform[4],
                    transform[5],
                ))
                .ok_or_else(|| unsupported("transformation singulière"))?;
            let map = |p: Point| {
                [
                    outer[0] * p.x + outer[2] * p.y + outer[4],
                    outer[1] * p.x + outer[3] * p.y + outer[5],
                ]
            };
            let contours = contours(&path, map, count)?;
            if contours.is_empty() {
                continue;
            }
            let mut shape = crate::paths::normalize(
                Shape {
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
                    transform: shape_transform,
                    id: 0,
                    kind: ShapeKind::Line,
                    x: 0.0,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                    rotation: 0.0,
                    fill: Some(fill),
                    stroke: None,
                    stroke_width: 0.0,
                    opacity,
                },
                contours,
            );
            if let ShapeKind::Path { even_odd: rule, .. } = &mut shape.kind {
                *rule = even_odd;
            }
            shape.fill_gradient = gradient;
            shape.fill_pattern = pattern;
            crate::io::validate_shape(&shape)?;
            out.push(shape);
            if out.len() > 1000 {
                return Err(unsupported("plus de 1 000 surfaces"));
            }
        }
    }
    Ok(out)
}
pub(crate) fn contours(
    path: &tiny_skia::Path,
    map: impl Fn(Point) -> [f32; 2],
    count: &mut usize,
) -> Result<Vec<Contour>, String> {
    let mut out = Vec::new();
    let mut c = Contour {
        nodes: Vec::new(),
        closed: false,
    };
    let node = |p| Node {
        point: p,
        incoming: p,
        outgoing: p,
    };
    for segment in path.segments() {
        if c.closed && !matches!(segment, PathSegment::MoveTo(_) | PathSegment::Close) {
            let start = c.nodes[0].point;
            if c.nodes.len() >= 2 {
                out.push(c);
            }
            c = Contour {
                nodes: vec![node(start)],
                closed: false,
            };
            *count += 1;
        }
        match segment {
            PathSegment::MoveTo(p) => {
                if c.nodes.len() >= 2 {
                    out.push(c);
                }
                c = Contour {
                    nodes: vec![node(map(p))],
                    closed: false,
                };
                *count += 1;
            }
            PathSegment::LineTo(p) => {
                c.nodes.push(node(map(p)));
                *count += 1;
            }
            PathSegment::QuadTo(q, p) => {
                let q = map(q);
                let p = map(p);
                let a = c
                    .nodes
                    .last_mut()
                    .ok_or_else(|| unsupported("courbe sans point initial"))?;
                a.outgoing = [
                    a.point[0] + 2.0 / 3.0 * (q[0] - a.point[0]),
                    a.point[1] + 2.0 / 3.0 * (q[1] - a.point[1]),
                ];
                let mut b = node(p);
                b.incoming = [
                    p[0] + 2.0 / 3.0 * (q[0] - p[0]),
                    p[1] + 2.0 / 3.0 * (q[1] - p[1]),
                ];
                c.nodes.push(b);
                *count += 1;
            }
            PathSegment::CubicTo(a, b, p) => {
                c.nodes
                    .last_mut()
                    .ok_or_else(|| unsupported("courbe sans point initial"))?
                    .outgoing = map(a);
                let mut n = node(map(p));
                n.incoming = map(b);
                c.nodes.push(n);
                *count += 1;
            }
            PathSegment::Close => {
                c.closed = true;
                if c.nodes.len() > 2 && c.nodes[0].point == c.nodes.last().unwrap().point {
                    let last = c.nodes.pop().unwrap();
                    c.nodes[0].incoming = last.incoming;
                }
            }
        }
        if *count > LIMIT {
            return Err(unsupported("plus de 20 000 nœuds"));
        }
    }
    if c.nodes.len() >= 2 {
        out.push(c);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        document::{Content, Editor},
        io, operations, vector,
    };
    use serde_json::json;
    fn block(body: &str) -> Shape {
        Shape {
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
            id: 5,
            kind: ShapeKind::Svg {
                source: format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="180">{body}</svg>"#
                ),
                intrinsic_w: 240.0,
                intrinsic_h: 180.0,
            },
            x: 0.0,
            y: 0.0,
            w: 240.0,
            h: 180.0,
            rotation: 0.0,
            fill: None,
            stroke: None,
            stroke_width: 0.0,
            opacity: 1.0,
        }
    }
    fn compare(s: &Shape, epsilon: f64) {
        let before = vector::rasterize(&[s.clone()], 400, 350).unwrap();
        let shapes = decompose(s).unwrap();
        assert!(shapes
            .iter()
            .all(|s| matches!(s.kind, ShapeKind::Path { .. })));
        let after = vector::rasterize(&shapes, 400, 350).unwrap();
        let mean = before
            .as_raw()
            .iter()
            .zip(after.as_raw())
            .map(|(a, b)| a.abs_diff(*b) as f64)
            .sum::<f64>()
            / before.as_raw().len() as f64;
        assert!(mean <= epsilon, "erreur moyenne {mean} > {epsilon}");
    }
    #[test]
    fn svg_primitives_arcs_quadratics_holes_and_nested_transforms() {
        let s = block(
            r##"<g transform="translate(8 5)"><g transform="rotate(10 80 60)"><rect x="20" y="20" width="80" height="45" rx="8" fill="#60a030"/><ellipse cx="165" cy="65" rx="25" ry="35" fill="#8040cc"/><path d="M20 120Q80 60 120 130A30 18 20 0 1 180 140L20 150Z" fill="#4080e0" fill-opacity=".6"/></g></g><path d="M15 15H60V60H15ZM25 25V50H50V25Z" fill-rule="evenodd"/>"##,
        );
        compare(&s, 0.03);
        let mut placed = s.clone();
        placed.x = 65.0;
        placed.y = 45.0;
        placed.w = 270.0;
        placed.h = 210.0;
        placed.rotation = 17.0;
        compare(&placed, 0.04);
    }
    #[test]
    fn svg_stroke_styles_paint_order_and_nonuniform_transform_survive_as_surfaces() {
        let s = block(
            r##"<g transform="translate(15 15) scale(1.2 .7)"><path d="M10 30C30 5 80 90 140 50" fill="none" stroke="#ff0080" stroke-width="8" stroke-linecap="round" stroke-dasharray="8 6"/><path d="M10 100L50 130L90 95" fill="#0080ff" stroke="#804000" stroke-width="12" stroke-linejoin="bevel" stroke-opacity=".7" paint-order="stroke fill"/></g>"##,
        );
        compare(&s, 0.08);
    }
    #[test]
    fn svg_text_becomes_editable_outlines() {
        let s = block(
            r##"<text x="15" y="60" font-family="Segoe UI" font-size="30" fill="#6040c0">Prisme</text>"##,
        );
        let out = decompose(&s).unwrap();
        assert!(!out.is_empty());
        compare(&s, 0.04);
    }
    #[test]
    fn close_then_continue_subpath_and_open_editable_are_safe() {
        let s = block(r##"<path d="M20 20L80 20L80 80ZL120 100L20 120Z" fill="#2080e0"/>"##);
        compare(&s, 0.03);
        let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work");
        std::fs::create_dir_all(&work).unwrap();
        let path = work.join("svg-edit-open.svg");
        let ShapeKind::Svg { source, .. } = &s.kind else {
            panic!()
        };
        std::fs::write(&path, source).unwrap();
        let mut e = Editor::default();
        operations::execute(
            &mut e,
            operations::Operation::OpenSvgEditable {
                path: path.to_string_lossy().into_owned(),
            },
        )
        .unwrap();
        let Content::Vector(shapes) = &e.doc().unwrap().layers[0].content else {
            panic!()
        };
        assert!(shapes
            .iter()
            .all(|s| matches!(s.kind, ShapeKind::Path { .. })));
        assert!(e.path.is_none());
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        let history = e.info().history.len();
        std::fs::write(&path,r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><filter id="c"><feImage href="#missing"/></filter></defs><rect width="80" height="80" filter="url(#c)"/></svg>"##).unwrap();
        assert!(operations::execute(
            &mut e,
            operations::Operation::OpenSvgEditable {
                path: path.to_string_lossy().into_owned()
            }
        )
        .is_err());
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
        assert_eq!(e.info().history.len(), history);
    }
    #[test]
    fn unsupported_svg_is_rejected_atomically_and_decomposition_is_one_undo() {
        fn command(e: &mut Editor, v: serde_json::Value) -> Result<(), String> {
            operations::execute(e, serde_json::from_value(v).unwrap())
        }
        let mut e = Editor::default();
        command(
            &mut e,
            json!({"type":"new","width":240,"height":180,"background":null}),
        )
        .unwrap();
        command(&mut e, json!({"type":"addLayer","kind":"vector"})).unwrap();
        let id = e.doc().unwrap().layers[1].id;
        for body in [
            r##"<defs><pattern id="g" width="10" height="10" patternUnits="userSpaceOnUse"><image width="5" height="10" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg=="/></pattern></defs><rect x="15" y="15" width="80" height="50" fill="url(#g)"/>"##,
            r##"<defs><filter id="c"><feImage href="#missing"/></filter></defs><rect width="80" height="80" filter="url(#c)"/>"##,
        ] {
            command(&mut e, json!({"type":"shape","id":id,"shape":block(body)})).unwrap();
            let before = io::project_bytes(e.doc().unwrap()).unwrap();
            let history = e.info().history.len();
            assert!(
                command(&mut e, json!({"type":"decomposeSvg","id":id,"shapeId":5}))
                    .unwrap_err()
                    .contains("conservé")
            );
            assert_eq!(io::project_bytes(e.doc().unwrap()).unwrap(), before);
            assert_eq!(e.info().history.len(), history);
        }
        let good = block(
            r##"<rect x="15" y="15" width="80" height="50" fill="#2080e0"/><circle cx="150" cy="100" r="20"/>"##,
        );
        command(&mut e, json!({"type":"shape","id":id,"shape":good})).unwrap();
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        let history = e.info().history.len();
        command(&mut e, json!({"type":"decomposeSvg","id":id,"shapeId":5})).unwrap();
        assert_eq!(e.info().history.len(), history + 1);
        let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
            panic!()
        };
        assert_eq!(shapes.len(), 2);
        assert_ne!(shapes[0].id, shapes[1].id);
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
        let reopened = io::read_project(&io::project_bytes(e.doc().unwrap()).unwrap()).unwrap();
        assert_eq!(vector::flatten(&reopened).unwrap(), before);
        command(&mut e, json!({"type":"undo"})).unwrap();
        let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
            panic!()
        };
        assert!(matches!(shapes[0].kind, ShapeKind::Svg { .. }));
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
    }
}
