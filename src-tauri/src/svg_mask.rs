//! Masques SVG peints, autonomes et non destructifs. Coordonnées locales.
use crate::{
    document::{Shape, ShapeKind},
    groups::{self, Matrix},
};
use serde::{Deserialize, Serialize};
/// usvg may eliminate a whole subtree behind an empty/invalid mask. Detect
/// referenced definitions which disappeared so editable import cannot silently
/// turn that hidden, recoverable content into missing objects.
pub fn check_normalized_masks(source: &str, tree: &resvg::usvg::Tree) -> Result<(), String> {
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let definitions = doc
        .descendants()
        .filter(|n| n.has_tag_name("mask"))
        .filter_map(|n| n.attribute("id").map(|id| (id, n)))
        .collect::<std::collections::HashMap<_, _>>();
    if definitions.is_empty() {
        return Ok(());
    }
    fn refs(text: &str, out: &mut std::collections::HashSet<String>) {
        let lower = text.to_ascii_lowercase();
        let mut at = 0;
        while let Some(i) = lower[at..].find("url(") {
            let begin = at + i + 4;
            let Some(end) = text[begin..].find(')') else {
                break;
            };
            let value = text[begin..begin + end].trim().trim_matches(['\'', '"']);
            if let Some(id) = value.strip_prefix('#') {
                out.insert(id.to_string());
            }
            at = begin + end + 1;
        }
    }
    fn node_refs(node: roxmltree::Node<'_, '_>, out: &mut std::collections::HashSet<String>) {
        for a in node.attributes() {
            if a.name() == "mask" || a.name() == "style" {
                refs(a.value(), out);
            }
        }
        if node.has_tag_name("style") {
            for text in node.children().filter_map(|n| n.text()) {
                refs(text, out);
            }
        }
    }
    let mut wanted = std::collections::HashSet::new();
    for n in doc
        .descendants()
        .filter(|n| n.is_element() && !n.ancestors().any(|a| a.has_tag_name("mask")))
    {
        node_refs(n, &mut wanted);
    }
    loop {
        let before = wanted.len();
        for id in wanted.clone() {
            if let Some(mask) = definitions.get(id.as_str()) {
                for n in mask.descendants().filter(|n| n.is_element()) {
                    node_refs(n, &mut wanted);
                }
            }
        }
        if wanted.len() == before {
            break;
        }
    }
    let known = tree
        .masks()
        .iter()
        .map(|m| m.id())
        .collect::<std::collections::HashSet<_>>();
    for id in wanted {
        if definitions.contains_key(id.as_str()) && !known.contains(id.as_str()) {
            return Err(format!("Décomposition SVG impossible : le masque « {id} » élimine entièrement un contenu pendant la normalisation. Le bloc original est conservé."));
        }
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MaskMode {
    Alpha,
    Luminance,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorMask {
    pub transform: Matrix,
    pub region: [f32; 4],
    pub mode: MaskMode,
    pub children: Vec<Shape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intersect: Option<Box<VectorMask>>,
}
impl VectorMask {
    /// PDF luminosity masks do not use SVG's RGB luminance coefficients in
    /// every viewer. Preserve the actual coverage as an alpha soft mask at
    /// page resolution. Document geometry and the source mask stay editable.
    pub fn pdf_alpha(
        &self,
        parent: Matrix,
        crop: [f32; 4],
        pixels: &mut u64,
    ) -> Result<Self, String> {
        let mut world = self.clone();
        world.apply(parent);
        let [x, y, w, h] = world.region;
        let points = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]
            .map(|p| groups::point(world.transform, p));
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for p in points {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        for i in 0..2 {
            lo[i] = (lo[i].floor() - 1.).max(crop[i]);
            hi[i] = (hi[i].ceil() + 1.).min(crop[i + 2]);
        }
        if hi[0] <= lo[0] || hi[1] <= lo[1] {
            return Ok(Self {
                transform: groups::IDENTITY,
                region: [0., 0., 1., 1.],
                mode: MaskMode::Alpha,
                children: vec![],
                intersect: None,
            });
        }
        let (w, h) = ((hi[0] - lo[0]) as u32, (hi[1] - lo[1]) as u32);
        crate::io::dimensions(w, h)
            .map_err(|_| "Export PDF : couverture de masque trop grande ; exportez en SVG")?;
        *pixels += w as u64 * h as u64;
        if *pixels > 16_777_216 {
            return Err("Export PDF : plus de 16 millions de pixels d’images et masques à préparer ; exportez en SVG".into());
        }
        let id = crate::paint::next_id();
        let svg=crate::vector::svg_document(w,h,&format!("<defs>{}</defs><g transform=\"translate({} {})\"><rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"white\" mask=\"url(#{id})\"/></g>",world.definition(&id),-lo[0],-lo[1],lo[0],lo[1]));
        let bytes = crate::io::png_bytes(&crate::vector::render_svg(&svg, w, h)?)?;
        let asset = crate::embedded::ImageAsset::from_bytes("image/png", &bytes, "optimizeSpeed")?;
        let det = parent[0] * parent[3] - parent[1] * parent[2];
        if !det.is_finite() || det.abs() < 1e-12 {
            return Err("Export PDF : repère de masque non inversible".into());
        }
        let inverse = [
            parent[3] / det,
            -parent[1] / det,
            -parent[2] / det,
            parent[0] / det,
            (parent[2] * parent[5] - parent[3] * parent[4]) / det,
            (parent[1] * parent[4] - parent[0] * parent[5]) / det,
        ];
        Ok(Self {
            transform: inverse,
            region: [lo[0], lo[1], w as f32, h as f32],
            mode: MaskMode::Alpha,
            children: vec![Shape {
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
                id: 0,
                kind: ShapeKind::Image { asset },
                x: lo[0],
                y: lo[1],
                w: w as f32,
                h: h as f32,
                rotation: 0.,
                fill: None,
                stroke: None,
                stroke_width: 0.,
                opacity: 1.,
            }],
            intersect: None,
        })
    }
    pub fn apply(&mut self, m: Matrix) {
        self.transform = groups::multiply(m, self.transform);
        if let Some(next) = &mut self.intersect {
            next.apply(m);
        }
    }
    pub fn imported(
        mask: &resvg::usvg::Mask,
        outer: Matrix,
        depth: usize,
        nodes: &mut usize,
        total: &mut usize,
    ) -> Result<Self, String> {
        if depth > 64 {
            return Err("Masque SVG trop imbriqué ; le bloc original est conservé".into());
        }
        let r = mask.rect();
        let result = Self {
            transform: outer,
            region: [r.x(), r.y(), r.width(), r.height()],
            mode: match mask.kind() {
                resvg::usvg::MaskType::Alpha => MaskMode::Alpha,
                _ => MaskMode::Luminance,
            },
            children: crate::svg_edit::mask_children(mask.root(), depth + 1, nodes, total)?,
            intersect: mask
                .mask()
                .map(|m| Self::imported(m, outer, depth + 1, nodes, total).map(Box::new))
                .transpose()?,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), String> {
        validate_mask(self, 0, &mut 0, &mut 0, groups::IDENTITY)
    }
    pub fn preview(&self) -> Result<image::RgbaImage, String> {
        self.validate()?;
        let [x, y, w, h] = self.region;
        let points = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]
            .map(|p| groups::point(self.transform, p));
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for p in points {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        let (bw, bh) = (hi[0] - lo[0], hi[1] - lo[1]);
        if bw <= 0. || bh <= 0. || ![bw, bh].iter().all(|v| v.is_finite()) {
            return Err("Région de masque trop petite ou hors limites pour l’aperçu".into());
        }
        let scale = 192. / bw.max(bh);
        let (pw, ph) = (
            (bw * scale).round().max(1.) as u32,
            (bh * scale).round().max(1.) as u32,
        );
        let id = crate::paint::next_id();
        let source=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pw}\" height=\"{ph}\" viewBox=\"{} {} {bw} {bh}\"><defs>{}</defs><rect x=\"{}\" y=\"{}\" width=\"{bw}\" height=\"{bh}\" fill=\"white\" mask=\"url(#{id})\"/></svg>",lo[0],lo[1],self.definition(&id),lo[0],lo[1]);
        crate::vector::render_svg(&source, pw, ph)
    }
    pub fn definition(&self, id: &str) -> String {
        let [x, y, w, h] = self.region;
        let m = self.transform;
        let corners = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]].map(|p| groups::point(m, p));
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for p in corners {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        let clip = crate::paint::next_id();
        let mut defs=format!("<clipPath id=\"{clip}\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/></clipPath>");
        let extra = if let Some(next) = &self.intersect {
            let next_id = crate::paint::next_id();
            defs += &next.definition(&next_id);
            format!("mask=\"url(#{next_id})\"")
        } else {
            String::new()
        };
        // SVG mask regions are axis-aligned. Clip the original region inside
        // its affine frame and pad the outer bounding box to avoid a second
        // antialiasing operation at exactly the same boundary.
        format!("{defs}<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" maskContentUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" mask-type=\"{}\" {extra}><g transform=\"matrix({} {} {} {} {} {})\" clip-path=\"url(#{clip})\">{}</g></mask>",lo[0]-1.,lo[1]-1.,hi[0]-lo[0]+2.,hi[1]-lo[1]+2.,if self.mode==MaskMode::Alpha{"alpha"}else{"luminance"},m[0],m[1],m[2],m[3],m[4],m[5],self.children.iter().map(crate::vector::shape_svg).collect::<String>())
    }
}
pub fn attach(group: &mut Shape, mut mask: VectorMask) {
    mask.apply(groups::translation(-group.x, -group.y));
    group.mask = Some(mask);
}

/// Keep the selected branch's ancestor frames/styles/masks. Sampling just a
/// 1x1 surface uses the same alpha/luminance/interpolation as document rendering.
pub fn pick(
    layer: &crate::document::Layer,
    candidates: &[u64],
    point: [f32; 2],
) -> Result<Option<u64>, String> {
    use crate::document::Content;
    if candidates.len() > 20_000
        || candidates
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != candidates.len()
        || point.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err("Sélection vectorielle hors limites".into());
    }
    let Content::Vector(shapes) = &layer.content else {
        return Err("Sélectionnez un calque vectoriel".into());
    };
    fn branch(s: &Shape, id: u64) -> Option<Shape> {
        if s.id == id {
            return Some(s.clone());
        }
        if let ShapeKind::Group { children, .. } = &s.kind {
            for child in children {
                if let Some(child) = branch(child, id) {
                    let mut parent = s.clone();
                    if let ShapeKind::Group { children, .. } = &mut parent.kind {
                        *children = vec![child];
                    }
                    return Some(parent);
                }
            }
        }
        None
    }
    for id in candidates {
        let s = shapes
            .iter()
            .find_map(|s| branch(s, *id))
            .ok_or("La forme sélectionnée a changé")?;
        let body = crate::vector::shape_svg(&s);
        let tree = crate::vector::internal_tree(&crate::vector::svg_document(1, 1, &body))?;
        // A filter needs the surrounding source pixels (offset, blur,
        // convolution...). A 1x1 source canvas clips those pixels too early.
        // Render its complete bounded layer region on the same pixel grid.
        if !tree.filters().is_empty() {
            let r = tree.root().abs_layer_bounding_box();
            let (x, y) = (r.x().floor(), r.y().floor());
            let (w, h) = (
                (r.right().ceil() - x).max(1.) as u32,
                (r.bottom().ceil() - y).max(1.) as u32,
            );
            crate::io::dimensions(w, h)
                .map_err(|_| "Sélection des effets : région supérieure à 16 millions de pixels")?;
            let (px, py) = (point[0].floor() - x, point[1].floor() - y);
            if px < 0. || py < 0. || px >= w as f32 || py >= h as f32 {
                continue;
            }
            let svg = crate::vector::svg_document(
                w,
                h,
                &format!("<g transform=\"translate({} {})\">{body}</g>", -x, -y),
            );
            if crate::vector::render_svg(&svg, w, h)?.get_pixel(px as u32, py as u32)[3] > 0 {
                return Ok(Some(*id));
            }
            continue;
        }
        let svg = crate::vector::svg_document(
            1,
            1,
            &format!(
                "<g transform=\"translate({} {})\">{}</g>",
                -point[0].floor(),
                -point[1].floor(),
                crate::vector::shape_svg(&s)
            ),
        );
        if crate::vector::render_svg(&svg, 1, 1)?.get_pixel(0, 0)[3] > 0 {
            return Ok(Some(*id));
        }
    }
    Ok(None)
}
fn matrix(m: Matrix) -> Result<(), String> {
    if m.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
        || (m[0] * m[3] - m[1] * m[2]).abs() < 1e-12
    {
        Err("Transformation de masque invalide ou hors limites".into())
    } else {
        Ok(())
    }
}
fn validate_mask(
    mask: &VectorMask,
    depth: usize,
    count: &mut usize,
    nodes: &mut usize,
    parent: Matrix,
) -> Result<(), String> {
    if depth > 8 {
        return Err("Au plus huit niveaux de masques SVG".into());
    }
    if mask.region.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
        || mask.region[2] <= 0.
        || mask.region[3] <= 0.
    {
        return Err("Région de masque invalide".into());
    }
    matrix(mask.transform)?;
    let m = groups::multiply(parent, mask.transform);
    matrix(m)?;
    validate_shapes(&mask.children, 0, depth, count, nodes, m)?;
    if let Some(next) = &mask.intersect {
        validate_mask(next, depth + 1, count, nodes, parent)?;
    }
    Ok(())
}
fn validate_shapes(
    shapes: &[Shape],
    shape_depth: usize,
    mask_depth: usize,
    count: &mut usize,
    nodes: &mut usize,
    parent: Matrix,
) -> Result<(), String> {
    if shape_depth > 16 {
        return Err("Au plus seize niveaux de groupes dans un masque".into());
    }
    for s in shapes {
        *count += 1;
        if s.text_path.as_ref().is_some_and(|p| p.source_id.is_some()) {
            return Err("Détachez le chemin du texte avant de l’intégrer à un masque".into());
        }
        if *count > 1000 {
            return Err("Au plus 1 000 formes par masque SVG".into());
        }
        let mut fields = s.clone();
        fields.mask = None;
        fields.clip = None;
        crate::io::validate_shape_fields(&fields)?;
        let m = groups::multiply(
            parent,
            groups::multiply(s.transform.unwrap_or(groups::IDENTITY), groups::rotation(s)),
        );
        matrix(m)?;
        match &s.kind {
            ShapeKind::Group {
                children,
                intrinsic_w,
                intrinsic_h,
            } => {
                if children.is_empty()
                    || ![*intrinsic_w, *intrinsic_h, s.w, s.h]
                        .iter()
                        .all(|v| v.is_finite() && *v >= 0.001)
                    || s.fill.is_some()
                    || s.stroke.is_some()
                    || s.stroke_width != 0.
                {
                    return Err("Groupe de masque invalide".into());
                }
                let frame = groups::multiply(parent, groups::frame(s));
                matrix(frame)?;
                validate_shapes(children, shape_depth + 1, mask_depth, count, nodes, frame)?;
                if let Some(c) = &s.clip {
                    c.validate()?;
                }
                if let Some(mask) = &s.mask {
                    validate_mask(mask, mask_depth + 1, count, nodes, frame)?;
                }
            }
            _ => {
                if s.mask.is_some() || s.clip.is_some() {
                    return Err("Placez le masque ou la découpe sur un groupe".into());
                }
                if let ShapeKind::Path { contours, .. } = &s.kind {
                    *nodes += contours.iter().map(|c| c.nodes.len()).sum::<usize>();
                    if *nodes > 20_000 {
                        return Err("Au plus 20 000 nœuds par masque".into());
                    }
                }
            }
        }
    }
    Ok(())
}
