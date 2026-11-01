//! Découpes vectorielles non destructives, union de formes et intersections.
use crate::{
    document::{Shape, ShapeKind},
    groups::{self, Matrix},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub transform: Matrix,
    pub paths: Vec<Shape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intersect: Option<Box<Clip>>,
}
impl Clip {
    pub fn apply(&mut self, m: Matrix) {
        self.transform = groups::multiply(m, self.transform);
        if let Some(c) = &mut self.intersect {
            c.apply(m);
        }
    }
    pub fn rectangle(x: f32, y: f32, w: f32, h: f32, transform: Matrix) -> Self {
        let mut s = base();
        s.kind = ShapeKind::Path {
            contours: vec![crate::paths::Contour {
                closed: true,
                nodes: [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
                    .into_iter()
                    .map(|p| crate::paths::Node {
                        point: p,
                        incoming: p,
                        outgoing: p,
                    })
                    .collect(),
            }],
            even_odd: false,
        };
        if let ShapeKind::Path { contours, .. } = &s.kind {
            s = crate::paths::normalize(s.clone(), contours.clone());
        }
        Self {
            transform,
            paths: vec![s],
            intersect: None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        validate_clip(self, 0, &mut 0, &mut 0, groups::IDENTITY)
    }
    pub fn definition(&self, id: &str) -> String {
        let mut defs = String::new();
        let content = self
            .paths
            .iter()
            .map(|s| clip_shape(s, groups::IDENTITY, None, &mut defs))
            .collect::<String>();
        let attr = if let Some(c) = &self.intersect {
            let next = crate::paint::next_id();
            defs += &c.definition(&next);
            format!("clip-path=\"url(#{next})\"")
        } else {
            String::new()
        };
        let m = self.transform;
        format!("{defs}<clipPath id=\"{id}\" clipPathUnits=\"userSpaceOnUse\" transform=\"matrix({} {} {} {} {} {})\" {attr}>{content}</clipPath>",m[0],m[1],m[2],m[3],m[4],m[5])
    }
    pub fn imported(
        clip: &resvg::usvg::ClipPath,
        outer: Matrix,
        depth: usize,
        nodes: &mut usize,
        total: &mut usize,
    ) -> Result<Self, String> {
        if depth > 8 {
            return Err("Décomposition SVG impossible : plus de huit niveaux de découpes. Le bloc original est conservé.".into());
        }
        let paths = collect(clip.root(), groups::IDENTITY, depth, nodes, total)?;
        let intersect = clip
            .clip_path()
            .map(|c| Self::imported(c, outer, depth + 1, nodes, total).map(Box::new))
            .transpose()?;
        let result = Self {
            transform: groups::multiply(outer, crate::paint::matrix(clip.transform())),
            paths,
            intersect,
        };
        result.validate()?;
        Ok(result)
    }
}
pub(crate) fn base() -> Shape {
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
        id: 0,
        x: 0.,
        y: 0.,
        w: 1.,
        h: 1.,
        rotation: 0.,
        fill: Some("#000000".into()),
        stroke: None,
        stroke_width: 0.,
        opacity: 1.,
        kind: ShapeKind::Line,
    }
}
fn collect(
    group: &resvg::usvg::Group,
    parent: Matrix,
    depth: usize,
    nodes: &mut usize,
    total: &mut usize,
) -> Result<Vec<Shape>, String> {
    if depth > 16 {
        return Err("Découpe SVG : arbre trop profond ; le bloc original est conservé".into());
    }
    let m = groups::multiply(parent, crate::paint::matrix(group.transform()));
    let mut out = Vec::new();
    for node in group.children() {
        match node {
            resvg::usvg::Node::Path(p) if p.is_visible() => {
                let path = p
                    .data()
                    .clone()
                    .transform(resvg::tiny_skia::Transform::from_row(
                        m[0], m[1], m[2], m[3], m[4], m[5],
                    ))
                    .ok_or("Transformation de découpe invalide")?;
                let contours = crate::svg_edit::contours(&path, |p| [p.x, p.y], nodes)?;
                if !contours.is_empty() {
                    let mut s = crate::paths::normalize(base(), contours);
                    if let ShapeKind::Path { even_odd, .. } = &mut s.kind {
                        *even_odd = p
                            .fill()
                            .is_some_and(|f| f.rule() == resvg::usvg::FillRule::EvenOdd);
                    }
                    out.push(s);
                    *total += 1;
                    if *total > 1000 {
                        return Err(
                            "Découpe SVG : plus de 1 000 éléments ; le bloc original est conservé"
                                .into(),
                        );
                    }
                }
            }
            resvg::usvg::Node::Group(g) => out.extend(collect(g, m, depth + 1, nodes, total)?),
            resvg::usvg::Node::Text(t) => {
                out.extend(collect(t.flattened(), m, depth + 1, nodes, total)?)
            }
            resvg::usvg::Node::Path(_) => (),
            _ => {
                return Err(
                    "Élément de découpe non pris en charge ; le bloc original est conservé".into(),
                )
            }
        }
    }
    if let Some(c) = group.clip_path() {
        if !out.is_empty() {
            let clip = Clip::imported(c, m, depth + 1, nodes, total)?;
            let mut s = crate::svg_edit::wrap_group(out, 1., None)?;
            attach(&mut s, clip);
            return Ok(vec![s]);
        }
    }
    Ok(out)
}
pub fn attach(group: &mut Shape, mut clip: Clip) {
    clip.apply(groups::translation(-group.x, -group.y));
    group.clip = Some(clip);
}
fn check_matrix(m: Matrix) -> Result<(), String> {
    if m.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
        || (m[0] * m[3] - m[1] * m[2]).abs() < 1e-12
    {
        Err("Transformation de découpe invalide ou hors limites".into())
    } else {
        Ok(())
    }
}
fn validate_clip(
    c: &Clip,
    depth: usize,
    count: &mut usize,
    nodes: &mut usize,
    parent: Matrix,
) -> Result<(), String> {
    if depth > 8 || c.paths.is_empty() {
        return Err("Découpe vide ou plus de huit niveaux de découpes".into());
    }
    check_matrix(c.transform)?;
    let m = groups::multiply(parent, c.transform);
    check_matrix(m)?;
    for s in &c.paths {
        validate_shape(s, depth, count, nodes, m)?;
    }
    if let Some(c) = &c.intersect {
        validate_clip(c, depth + 1, count, nodes, parent)?;
    }
    Ok(())
}
fn validate_shape(
    s: &Shape,
    depth: usize,
    count: &mut usize,
    nodes: &mut usize,
    parent: Matrix,
) -> Result<(), String> {
    *count += 1;
    if *count > 1000 || depth > 16 {
        return Err("Découpe : arbre trop profond ou plus de 1 000 formes".into());
    }
    if s.svg_viewport
        || s.filters.is_some()
        || s.mask.is_some()
        || s.group_style.is_some()
        || s.fill_gradient.is_some()
        || s.stroke_gradient.is_some()
        || s.fill_pattern.is_some()
        || s.stroke_pattern.is_some()
        || s.stroke.is_some()
        || s.stroke_width != 0.
        || s.opacity != 1.
    {
        return Err(
            "La découpe contient uniquement des géométries opaques sans peinture ni trait".into(),
        );
    }
    let mut fields = s.clone();
    fields.clip = None;
    crate::io::validate_shape_fields(&fields)?;
    match &s.kind {
        ShapeKind::Path { contours, .. } => {
            if s.clip.is_some() {
                return Err("Placez la découpe sur un groupe de géométries".into());
            }
            *nodes += contours.iter().map(|c| c.nodes.len()).sum::<usize>();
            if *nodes > 20_000 {
                return Err("Découpe : plus de 20 000 nœuds".into());
            }
            check_matrix(groups::multiply(
                parent,
                groups::multiply(s.transform.unwrap_or(groups::IDENTITY), groups::rotation(s)),
            ))?;
        }
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
            {
                return Err("Groupe de découpe invalide".into());
            }
            let m = groups::multiply(parent, groups::frame(s));
            check_matrix(m)?;
            for s in children {
                validate_shape(s, depth + 1, count, nodes, m)?;
            }
            if let Some(c) = &s.clip {
                validate_clip(c, depth + 1, count, nodes, m)?;
            }
        }
        _ => return Err("La découpe doit contenir des tracés ou groupes de tracés".into()),
    }
    Ok(())
}
fn clip_shape(s: &Shape, parent: Matrix, inherited: Option<Clip>, defs: &mut String) -> String {
    match &s.kind {
        ShapeKind::Path { contours, even_odd } => {
            let m = groups::multiply(
                parent,
                groups::multiply(s.transform.unwrap_or(groups::IDENTITY), groups::rotation(s)),
            );
            let xy = |p: [f32; 2]| groups::point(m, [s.x + p[0] * s.w, s.y + p[1] * s.h]);
            let absolute = contours
                .iter()
                .map(|c| crate::paths::Contour {
                    closed: c.closed,
                    nodes: c
                        .nodes
                        .iter()
                        .map(|n| crate::paths::Node {
                            point: xy(n.point),
                            incoming: xy(n.incoming),
                            outgoing: xy(n.outgoing),
                        })
                        .collect(),
                })
                .collect::<Vec<_>>();
            let attr = if let Some(c) = inherited {
                let id = crate::paint::next_id();
                *defs += &c.definition(&id);
                format!("clip-path=\"url(#{id})\"")
            } else {
                String::new()
            };
            format!(
                "<path d=\"{}\" clip-rule=\"{}\" {attr}/>",
                crate::paths::data(&base(), &absolute),
                if *even_odd { "evenodd" } else { "nonzero" }
            )
        }
        ShapeKind::Group { children, .. } => {
            let m = groups::multiply(parent, groups::frame(s));
            let combined = if let Some(c) = &s.clip {
                let mut c = c.clone();
                c.apply(m);
                let mut tail = &mut c;
                while let Some(ref mut next) = tail.intersect {
                    tail = next;
                }
                tail.intersect = inherited.map(Box::new);
                Some(c)
            } else {
                inherited
            };
            children
                .iter()
                .map(|s| clip_shape(s, m, combined.clone(), defs))
                .collect()
        }
        _ => String::new(),
    }
}
