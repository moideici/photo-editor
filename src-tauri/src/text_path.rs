//! Texte éditable sur un guide lié du même calque, ou sur une copie détachée.
use crate::{
    document::{Content, Document, Shape, ShapeKind},
    groups::{self, Matrix},
    paths::{Contour, Node},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPath {
    pub source_id: Option<u64>,
    pub contour_index: usize,
    pub contour: Contour,
    pub intrinsic_w: f32,
    pub intrinsic_h: f32,
    pub start_offset: f32,
    pub percent: bool,
    pub normal_offset: f32,
    pub anchor: String,
    pub reverse: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub source_id: Option<u64>,
    pub contour_index: usize,
    pub start_offset: f32,
    pub percent: bool,
    pub normal_offset: f32,
    pub anchor: String,
    pub reverse: bool,
}
fn error(s: &str) -> String {
    format!("Texte sur tracé : {s}")
}
impl TextPath {
    pub fn validate(&self, s: &Shape) -> Result<(), String> {
        if !matches!(s.kind, ShapeKind::Text { .. }) {
            return Err(error("réservé aux formes de texte"));
        }
        if let ShapeKind::Text { text, .. } = &s.kind {
            if text.len() > 10_000 || text.contains(['\n', '\r']) {
                return Err(error("une ligne, 10 000 octets maximum"));
            }
        }
        if !["start", "middle", "end"].contains(&self.anchor.as_str())
            || self.contour_index >= 1000
            || ![self.intrinsic_w, self.intrinsic_h]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.001 && *v <= 1e6)
            || !self.start_offset.is_finite()
            || self.start_offset.abs() > if self.percent { 1000. } else { 1e6 }
            || !self.normal_offset.is_finite()
            || self.normal_offset.abs() > 4096.
            || self
                .source_id
                .is_some_and(|id| id == 0 || id > 9_007_199_254_740_991)
            || self.contour.nodes.len() < 2
            || self.contour.nodes.len() > 20_000
            || self
                .contour
                .nodes
                .iter()
                .flat_map(|n| [n.point, n.incoming, n.outgoing])
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e9)
        {
            return Err(error("guide ou paramètres invalides"));
        }
        if s.w < 0.001 || s.h < 0.001 {
            return Err(error("dimensions de texte nulles"));
        }
        Ok(())
    }
    pub fn data(&self) -> String {
        let mut c = self.contour.clone();
        if self.reverse {
            c.nodes.reverse();
            for n in &mut c.nodes {
                std::mem::swap(&mut n.incoming, &mut n.outgoing);
            }
        }
        data(&c)
    }
}
fn data(c: &Contour) -> String {
    let xy = |p: [f32; 2]| format!("{} {}", p[0], p[1]);
    let mut out = format!("M{}", xy(c.nodes[0].point));
    for i in 1..c.nodes.len() + usize::from(c.closed) {
        let a = &c.nodes[i - 1];
        let b = &c.nodes[i % c.nodes.len()];
        out += &format!("C{} {} {}", xy(a.outgoing), xy(b.incoming), xy(b.point));
    }
    if c.closed {
        out.push('Z');
    }
    out
}
pub fn guide(s: &Shape) -> Result<Vec<Contour>, String> {
    let p = crate::paths::convert(s).map_err(|_| {
        error("choisissez un tracé, une ligne, une ellipse ou un rectangle sans arrondi")
    })?;
    let ShapeKind::Path { mut contours, .. } = p.kind else {
        return Err(error("guide non vectoriel"));
    };
    for n in contours.iter_mut().flat_map(|c| &mut c.nodes) {
        for q in [&mut n.point, &mut n.incoming, &mut n.outgoing] {
            *q = [s.x + q[0] * s.w, s.y + q[1] * s.h];
        }
    }
    Ok(contours)
}
fn frames(shapes: &[Shape], parent: Matrix, map: &mut HashMap<u64, (Shape, Matrix)>) {
    for s in shapes {
        map.insert(s.id, (s.clone(), parent));
        if let ShapeKind::Group { children, .. } = &s.kind {
            frames(children, groups::multiply(parent, groups::frame(s)), map);
        }
    }
}
pub fn validate_links(shapes: &[Shape]) -> Result<(), String> {
    if !has_links(shapes) {
        return Ok(());
    }
    let mut map = HashMap::new();
    frames(shapes, groups::IDENTITY, &mut map);
    for (s, _) in map.values() {
        if let Some(p) = &s.text_path {
            if let Some(id) = p.source_id {
                let (source,_)=map.get(&id).ok_or_else(||error("le guide lié est absent ; détachez le texte avant de supprimer ou remplacer ce guide"))?;
                let contours = guide(source)?;
                if p.contour_index >= contours.len() {
                    return Err(error("le contour lié n’existe plus ; choisissez un autre contour ou détachez le texte"));
                }
            }
        }
    }
    Ok(())
}
fn place(s: &mut Shape, mut c: Contour) -> Result<(), String> {
    let mut p = s
        .text_path
        .clone()
        .ok_or_else(|| error("paramètres absents"))?;
    p.contour = c.clone();
    p.intrinsic_w = 1.;
    p.intrinsic_h = 1.;
    let mut sample = s.clone();
    sample.x = 0.;
    sample.y = 0.;
    sample.w = 1.;
    sample.h = 1.;
    sample.rotation = 0.;
    sample.transform = None;
    sample.text_path = Some(p);
    // Bounds come from actual native glyphs, including spaces, accents and curves.
    let tree = crate::vector::internal_tree(&crate::vector::svg_document(
        1,
        1,
        &crate::vector::shape_svg(&sample),
    ))?;
    let r = tree.root().abs_stroke_bounding_box();
    let (x, y, w, h) = if r.width() > 0.001 && r.height() > 0.001 {
        (r.x(), r.y(), r.width(), r.height())
    } else {
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for n in &c.nodes {
            for q in [n.point, n.incoming, n.outgoing] {
                for i in 0..2 {
                    lo[i] = lo[i].min(q[i]);
                    hi[i] = hi[i].max(q[i]);
                }
            }
        }
        (
            lo[0],
            lo[1],
            (hi[0] - lo[0]).max(1.),
            (hi[1] - lo[1]).max(1.),
        )
    };
    for n in &mut c.nodes {
        for q in [&mut n.point, &mut n.incoming, &mut n.outgoing] {
            q[0] -= x;
            q[1] -= y;
        }
    }
    let p = s.text_path.as_mut().unwrap();
    p.contour = c;
    p.intrinsic_w = w;
    p.intrinsic_h = h;
    s.x = x;
    s.y = y;
    s.w = w;
    s.h = h;
    s.transform = None;
    s.rotation = 0.;
    Ok(())
}
pub fn refresh(shapes: &mut [Shape]) -> Result<(), String> {
    if !has_links(shapes) {
        return Ok(());
    }
    validate_links(shapes)?;
    let mut map = HashMap::new();
    frames(shapes, groups::IDENTITY, &mut map);
    fn walk(
        shapes: &mut [Shape],
        parent: Matrix,
        map: &HashMap<u64, (Shape, Matrix)>,
    ) -> Result<(), String> {
        for s in shapes {
            if let Some(p) = &s.text_path {
                if let Some(id) = p.source_id {
                    let (source, world) = map.get(&id).ok_or_else(|| error("guide absent"))?;
                    // Keep the text's own affine font frame. Grouping and
                    // ungrouping transfer transforms onto the child; dropping
                    // that matrix keeps the baseline but changes glyph size.
                    let own = groups::multiply(
                        s.transform.unwrap_or(groups::IDENTITY),
                        groups::rotation(s),
                    );
                    let m = groups::multiply(
                        crate::svg_filter::inverse(groups::multiply(parent, own)),
                        groups::multiply(
                            *world,
                            groups::multiply(
                                source.transform.unwrap_or(groups::IDENTITY),
                                groups::rotation(source),
                            ),
                        ),
                    );
                    let mut c = guide(source)?
                        .get(p.contour_index)
                        .ok_or_else(|| error("contour absent"))?
                        .clone();
                    for n in &mut c.nodes {
                        for q in [&mut n.point, &mut n.incoming, &mut n.outgoing] {
                            *q = groups::point(m, *q);
                        }
                    }
                    place(s, c)?;
                    s.transform = if own == groups::IDENTITY {
                        None
                    } else {
                        Some(own)
                    };
                }
            }
            if let ShapeKind::Group { children, .. } = &mut s.kind {
                // Clone frame before borrowing children to keep their own parent coordinates.
                let frame =
                    groups::multiply(parent, groups::frame(map.get(&s.id).map(|v| &v.0).unwrap()));
                walk(children, frame, map)?;
            }
        }
        Ok(())
    }
    walk(shapes, groups::IDENTITY, &map)
}
pub fn refresh_document(doc: &mut Document) -> Result<(), String> {
    for l in &mut doc.layers {
        if let Content::Vector(shapes) = &mut l.content {
            refresh(shapes)?;
            groups::validate(shapes)?;
        }
    }
    Ok(())
}
fn has_links(shapes: &[Shape]) -> bool {
    shapes.iter().any(|s| {
        s.text_path.as_ref().is_some_and(|p| p.source_id.is_some())
            || matches!(&s.kind,ShapeKind::Group{children,..} if has_links(children))
    })
}
pub fn layout(mut shapes: Vec<Shape>, id: u64, settings: Settings) -> Result<Shape, String> {
    let s = groups::find_mut(&mut shapes, id).ok_or_else(|| error("texte introuvable"))?;
    if !matches!(s.kind, ShapeKind::Text { .. }) {
        return Err(error("sélectionnez un texte"));
    }
    let old = s.text_path.clone();
    let dummy = Node {
        point: [0., 0.],
        incoming: [0., 0.],
        outgoing: [0., 0.],
    };
    let mut p = old.clone().unwrap_or(TextPath {
        source_id: settings.source_id,
        contour_index: 0,
        contour: Contour {
            nodes: vec![
                dummy.clone(),
                Node {
                    point: [1., 1.],
                    ..dummy
                },
            ],
            closed: false,
        },
        intrinsic_w: s.w.max(0.001),
        intrinsic_h: s.h.max(0.001),
        start_offset: 0.,
        percent: false,
        normal_offset: 0.,
        anchor: "start".into(),
        reverse: false,
    });
    if settings.source_id.is_none() && old.is_none() {
        return Err(error("choisissez un guide avant de détacher le texte"));
    }
    p.source_id = settings.source_id;
    p.contour_index = settings.contour_index;
    p.start_offset = settings.start_offset;
    p.percent = settings.percent;
    p.normal_offset = settings.normal_offset;
    p.anchor = settings.anchor;
    p.reverse = settings.reverse;
    p.validate(s)?;
    s.text_path = Some(p);
    refresh(&mut shapes)?;
    groups::validate(&shapes)?;
    Ok(groups::find(&shapes, id).unwrap().clone())
}
pub fn outlines(s: &Shape) -> Result<Shape, String> {
    let body = crate::vector::shape_svg(s);
    let tree = crate::vector::internal_tree(&crate::vector::svg_document(1, 1, &body))?;
    let r = tree.root().abs_stroke_bounding_box();
    if r.width() <= 0.001 || r.height() <= 0.001 {
        // Empty text, transparent paint or a start beyond the guide remains empty
        // in an export/outline copy; preserve the editable source in the document.
        let mut empty = s.clone();
        empty.text_path = None;
        empty.kind = ShapeKind::Path {
            contours: vec![],
            even_odd: false,
        };
        empty.fill = None;
        empty.stroke = None;
        empty.fill_gradient = None;
        empty.stroke_gradient = None;
        empty.fill_pattern = None;
        empty.stroke_pattern = None;
        empty.stroke_width = 0.;
        return Ok(empty);
    }
    // Convert the already laid-out glyph tree directly. Re-parsing it through
    // a tight SVG viewport can clip antialias coverage along the outer glyphs.
    let children = crate::svg_filter::with_definitions(&tree, || {
        crate::svg_edit::mask_children(tree.root(), 0, &mut 0, &mut 0)
    })?;
    let mut out = crate::svg_edit::wrap_group(children, 1., None)?;
    out.id = s.id;
    Ok(out)
}
