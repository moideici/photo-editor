//! Courbes cubiques éditables, stockées dans les coordonnées normalisées de la forme.
use crate::document::{Shape, ShapeKind};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub point: [f32; 2],
    pub incoming: [f32; 2],
    pub outgoing: [f32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contour {
    pub nodes: Vec<Node>,
    pub closed: bool,
}
pub fn validate(contours: &[Contour]) -> Result<(), String> {
    let n: usize = contours.iter().map(|c| c.nodes.len()).sum();
    if contours.len() > 1000
        || n > 20_000
        || contours.iter().any(|c| {
            c.nodes.len() < 2
                || c.nodes.iter().any(|n| {
                    [n.point, n.incoming, n.outgoing]
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite() || v.abs() > 16.0)
                })
        })
    {
        return Err("Tracé invalide (2 à 20 000 nœuds, coordonnées bornées)".into());
    }
    Ok(())
}
pub fn data(s: &Shape, contours: &[Contour]) -> String {
    let p = |p: [f32; 2]| format!("{} {}", s.x + p[0] * s.w, s.y + p[1] * s.h);
    let mut d = String::new();
    for c in contours {
        if c.nodes.is_empty() {
            continue;
        }
        d += &format!("M{}", p(c.nodes[0].point));
        for i in 1..c.nodes.len() + usize::from(c.closed) {
            let a = &c.nodes[i - 1];
            let b = &c.nodes[i % c.nodes.len()];
            if a.outgoing == a.point && b.incoming == b.point {
                d += &format!("L{}", p(b.point));
            } else {
                d += &format!("C{} {} {}", p(a.outgoing), p(b.incoming), p(b.point));
            }
        }
        if c.closed {
            d.push('Z');
        }
    }
    d
}
pub fn normalize(mut s: Shape, mut contours: Vec<Contour>) -> Shape {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for n in contours.iter().flat_map(|c| &c.nodes) {
        for p in [n.point, n.incoming, n.outgoing] {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    if !lo[0].is_finite() {
        lo = [0.0; 2];
        hi = [1.0; 2];
    }
    s.x = lo[0];
    s.y = lo[1];
    s.w = (hi[0] - lo[0]).max(0.001);
    s.h = (hi[1] - lo[1]).max(0.001);
    s.rotation = 0.0;
    for n in contours.iter_mut().flat_map(|c| &mut c.nodes) {
        for p in [&mut n.point, &mut n.incoming, &mut n.outgoing] {
            p[0] = (p[0] - s.x) / s.w;
            p[1] = (p[1] - s.y) / s.h;
        }
    }
    s.kind = ShapeKind::Path {
        contours,
        even_odd: true,
    };
    s
}

/// Les booléens utilisent des polygones à une tolérance maximale de 0,25 px.
pub fn boolean(a: &Shape, b: &Shape, kind: &str) -> Result<Shape, String> {
    use i_overlay::{
        core::{fill_rule::FillRule, overlay_rule::OverlayRule},
        float::single::SingleFloatOverlay,
    };
    fn polygons(s: &Shape) -> Result<Vec<Vec<[f64; 2]>>, String> {
        let ShapeKind::Path { contours, even_odd } = &s.kind else {
            return Err("Convertissez les deux formes en tracés".into());
        };
        validate(contours)?;
        if !even_odd && contours.len() > 1 {
            return Err("Le booléen exige la règle pair-impair pour les tracés composés".into());
        }
        if s.fill.is_none() || contours.iter().any(|c| !c.closed) {
            return Err("Deux tracés fermés avec un fond sont nécessaires".into());
        }
        let angle = s.rotation.to_radians();
        let (cx, cy) = (s.x + s.w / 2.0, s.y + s.h / 2.0);
        let convert = |p: [f32; 2]| {
            let x = s.x + p[0] * s.w - cx;
            let y = s.y + p[1] * s.h - cy;
            crate::groups::point(
                s.transform.unwrap_or(crate::groups::IDENTITY),
                [
                    cx + x * angle.cos() - y * angle.sin(),
                    cy + x * angle.sin() + y * angle.cos(),
                ],
            )
            .map(|v| v as f64)
        };
        let mut out = Vec::new();
        let mut total = 0;
        for c in contours {
            let mut points = vec![convert(c.nodes[0].point)];
            for i in 0..c.nodes.len() {
                let a = &c.nodes[i];
                let b = &c.nodes[(i + 1) % c.nodes.len()];
                flatten(
                    [
                        convert(a.point),
                        convert(a.outgoing),
                        convert(b.incoming),
                        convert(b.point),
                    ],
                    0,
                    &mut points,
                );
                if points.len() + total > 100_000 {
                    return Err("Tracé trop complexe pour le booléen".into());
                }
            }
            points.pop();
            total += points.len();
            out.push(points);
        }
        Ok(out)
    }
    let rule = match kind {
        "union" => OverlayRule::Union,
        "intersection" => OverlayRule::Intersect,
        "difference" => OverlayRule::Difference,
        "xor" => OverlayRule::Xor,
        _ => return Err("Opération booléenne inconnue".into()),
    };
    let pa = polygons(a)?;
    let pb = polygons(b)?;
    let result = pa.overlay(&pb, rule, FillRule::EvenOdd);
    let contours = result
        .into_iter()
        .flatten()
        .filter(|p| p.len() >= 3)
        .map(|p| Contour {
            closed: true,
            nodes: p
                .into_iter()
                .map(|p| {
                    let p = [p[0] as f32, p[1] as f32];
                    Node {
                        point: p,
                        incoming: p,
                        outgoing: p,
                    }
                })
                .collect(),
        })
        .collect();
    let mut result = normalize(a.clone(), contours);
    for gradient in [&mut result.fill_gradient, &mut result.stroke_gradient]
        .into_iter()
        .flatten()
    {
        if matches!(gradient.units, crate::paint::Units::ObjectBoundingBox) {
            let [x, y, w, h] = crate::paint::bounds(a)?;
            gradient.transform = crate::groups::multiply([w, 0., 0., h, x, y], gradient.transform);
            gradient.units = crate::paint::Units::UserSpaceOnUse;
        }
        gradient.transform = crate::groups::multiply(
            crate::groups::multiply(
                a.transform.unwrap_or(crate::groups::IDENTITY),
                crate::groups::rotation(a),
            ),
            gradient.transform,
        );
    }
    for pattern in [&mut result.fill_pattern, &mut result.stroke_pattern]
        .into_iter()
        .flatten()
    {
        if matches!(pattern.units, crate::paint::Units::ObjectBoundingBox) {
            let [x, y, w, h] = crate::paint::bounds(a)?;
            pattern.transform = crate::groups::multiply([w, 0., 0., h, x, y], pattern.transform);
            pattern.units = crate::paint::Units::UserSpaceOnUse;
        }
        pattern.transform = crate::groups::multiply(
            crate::groups::multiply(
                a.transform.unwrap_or(crate::groups::IDENTITY),
                crate::groups::rotation(a),
            ),
            pattern.transform,
        );
    }
    result.transform = None;
    result.id = 0;
    validate(match &result.kind {
        ShapeKind::Path { contours, .. } => contours,
        _ => unreachable!(),
    })?;
    Ok(result)
}
fn flatten(p: [[f64; 2]; 4], depth: u8, out: &mut Vec<[f64; 2]>) {
    let distance = |q: [f64; 2]| {
        let dx = p[3][0] - p[0][0];
        let dy = p[3][1] - p[0][1];
        let len = dx * dx + dy * dy;
        let t = if len > 0.0 {
            ((q[0] - p[0][0]) * dx + (q[1] - p[0][1]) * dy) / len
        } else {
            0.0
        };
        let t = t.clamp(0.0, 1.0);
        (q[0] - p[0][0] - t * dx).hypot(q[1] - p[0][1] - t * dy)
    };
    if depth >= 16 || (distance(p[1]) <= 0.25 && distance(p[2]) <= 0.25) {
        out.push(p[3]);
        return;
    }
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let a = mid(p[0], p[1]);
    let b = mid(p[1], p[2]);
    let c = mid(p[2], p[3]);
    let d = mid(a, b);
    let e = mid(b, c);
    let f = mid(d, e);
    flatten([p[0], a, d, f], depth + 1, out);
    flatten([f, e, c, p[3]], depth + 1, out);
}

pub fn convert(s: &Shape) -> Result<Shape, String> {
    let node = |p| Node {
        point: p,
        incoming: p,
        outgoing: p,
    };
    let mut result = s.clone();
    let contours=match &s.kind {
        ShapeKind::Path{..}=>return Ok(result),
        ShapeKind::Rect{radius} if *radius==0.0=>vec![Contour{closed:true,nodes:vec![node([0.0,0.0]),node([1.0,0.0]),node([1.0,1.0]),node([0.0,1.0])]}],
        ShapeKind::Line=>vec![Contour{closed:false,nodes:vec![node([0.0,0.0]),node([1.0,1.0])]}],
        ShapeKind::Ellipse=>{let k=0.2761424;vec![Contour{closed:true,nodes:vec![Node{point:[0.5,0.0],incoming:[0.5-k,0.0],outgoing:[0.5+k,0.0]},Node{point:[1.0,0.5],incoming:[1.0,0.5-k],outgoing:[1.0,0.5+k]},Node{point:[0.5,1.0],incoming:[0.5+k,1.0],outgoing:[0.5-k,1.0]},Node{point:[0.0,0.5],incoming:[0.0,0.5+k],outgoing:[0.0,0.5-k]}]}]},
        _=>return Err("Conversion disponible pour rectangle sans arrondi, ellipse et ligne ; les autres formes gardent leur édition d’origine".into())
    };
    result.kind = ShapeKind::Path {
        contours,
        even_odd: true,
    };
    Ok(result)
}
