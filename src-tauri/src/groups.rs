//! Hiérarchie vectorielle ; matrices SVG [a b c d e f], coordonnées locales.
use crate::document::{Shape, ShapeKind};
pub type Matrix = [f32; 6];
pub const IDENTITY: Matrix = [1., 0., 0., 1., 0., 0.];
pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}
pub fn point(m: Matrix, p: [f32; 2]) -> [f32; 2] {
    [
        m[0] * p[0] + m[2] * p[1] + m[4],
        m[1] * p[0] + m[3] * p[1] + m[5],
    ]
}
pub fn translation(x: f32, y: f32) -> Matrix {
    [1., 0., 0., 1., x, y]
}
pub fn rotation(s: &Shape) -> Matrix {
    let (sn, cs) = s.rotation.to_radians().sin_cos();
    let (x, y) = (s.x + s.w / 2., s.y + s.h / 2.);
    [cs, sn, -sn, cs, x - cs * x + sn * y, y - sn * x - cs * y]
}
pub fn frame(s: &Shape) -> Matrix {
    let ShapeKind::Group {
        intrinsic_w,
        intrinsic_h,
        ..
    } = &s.kind
    else {
        return IDENTITY;
    };
    multiply(
        multiply(s.transform.unwrap_or(IDENTITY), rotation(s)),
        [s.w / intrinsic_w, 0., 0., s.h / intrinsic_h, s.x, s.y],
    )
}
pub fn apply(s: &mut Shape, m: Matrix) {
    s.transform = Some(multiply(m, s.transform.unwrap_or(IDENTITY)));
}
pub fn find(shapes: &[Shape], id: u64) -> Option<&Shape> {
    for s in shapes {
        if s.id == id {
            return Some(s);
        }
        if let ShapeKind::Group { children, .. } = &s.kind {
            if let Some(found) = find(children, id) {
                return Some(found);
            }
        }
    }
    None
}
pub fn find_mut(shapes: &mut [Shape], id: u64) -> Option<&mut Shape> {
    for s in shapes {
        if s.id == id {
            return Some(s);
        }
        if let ShapeKind::Group { children, .. } = &mut s.kind {
            if let Some(found) = find_mut(children, id) {
                return Some(found);
            }
        }
    }
    None
}
pub fn siblings_mut(shapes: &mut Vec<Shape>, id: u64) -> Option<&mut Vec<Shape>> {
    if shapes.iter().any(|s| s.id == id) {
        return Some(shapes);
    }
    for s in shapes {
        if let ShapeKind::Group { children, .. } = &mut s.kind {
            if let Some(found) = siblings_mut(children, id) {
                return Some(found);
            }
        }
    }
    None
}
pub fn maximum_id(shapes: &[Shape]) -> u64 {
    shapes
        .iter()
        .map(|s| {
            s.id.max(match &s.kind {
                ShapeKind::Group { children, .. } => maximum_id(children),
                _ => 0,
            })
        })
        .max()
        .unwrap_or(0)
}
pub fn repair_legacy_ids(shapes: &mut [Shape]) -> Result<(), String> {
    fn walk(shapes: &mut [Shape], next: &mut u64, seen: &mut std::collections::HashSet<u64>) {
        for s in shapes {
            if s.id == 0 || !seen.insert(s.id) {
                *next += 1;
                s.id = *next;
                seen.insert(s.id);
            }
            if let ShapeKind::Group { children, .. } = &mut s.kind {
                walk(children, next, seen)
            }
        }
    }
    let mut next = maximum_id(shapes);
    if next > 9_007_199_254_700_000 {
        return Err("Identifiant vectoriel hors limites".into());
    }
    walk(shapes, &mut next, &mut Default::default());
    Ok(())
}
pub fn siblings(shapes: &[Shape], id: u64) -> Option<&[Shape]> {
    if shapes.iter().any(|s| s.id == id) {
        return Some(shapes);
    }
    for s in shapes {
        if let ShapeKind::Group { children, .. } = &s.kind {
            if let Some(found) = siblings(children, id) {
                return Some(found);
            }
        }
    }
    None
}
pub fn remove(shapes: &mut Vec<Shape>, id: u64) -> bool {
    if let Some(at) = shapes.iter().position(|s| s.id == id) {
        shapes.remove(at);
        return true;
    }
    for at in 0..shapes.len() {
        if let ShapeKind::Group { children, .. } = &mut shapes[at].kind {
            if remove(children, id) {
                if children.is_empty() {
                    shapes.remove(at);
                }
                return true;
            }
        }
    }
    false
}
pub fn assign_ids(shapes: &mut [Shape], next: &mut impl FnMut() -> u64) {
    fn walk(
        shapes: &mut [Shape],
        next: &mut impl FnMut() -> u64,
        map: &mut std::collections::HashMap<u64, u64>,
    ) {
        for s in shapes {
            let old = s.id;
            s.id = next();
            if old != 0 {
                map.insert(old, s.id);
            }
            if let ShapeKind::Group { children, .. } = &mut s.kind {
                walk(children, next, map);
            }
        }
    }
    fn links(shapes: &mut [Shape], map: &std::collections::HashMap<u64, u64>) {
        for s in shapes {
            if let Some(p) = &mut s.text_path {
                if let Some(id) = p.source_id {
                    if let Some(new) = map.get(&id) {
                        p.source_id = Some(*new);
                    }
                }
            }
            if let ShapeKind::Group { children, .. } = &mut s.kind {
                links(children, map);
            }
        }
    }
    let mut map = std::collections::HashMap::new();
    walk(shapes, next, &mut map);
    links(shapes, &map);
}
pub fn validate(shapes: &[Shape]) -> Result<(), String> {
    fn walk(
        shapes: &[Shape],
        depth: usize,
        count: &mut usize,
        ids: &mut std::collections::HashSet<u64>,
        parent: Matrix,
    ) -> Result<(), String> {
        if depth > 16 {
            return Err("Au plus 16 niveaux de groupes vectoriels".into());
        }
        for s in shapes {
            *count += 1;
            if *count > 20_000 {
                return Err("Au plus 20 000 formes par calque".into());
            }
            if s.id == 0 || s.id > 9_007_199_254_740_991 || !ids.insert(s.id) {
                return Err("Identifiants de formes nuls ou dupliqués".into());
            }
            crate::io::validate_shape_fields(s)?;
            if let ShapeKind::Group {
                children,
                intrinsic_w,
                intrinsic_h,
            } = &s.kind
            {
                if children.is_empty()
                    || !intrinsic_w.is_finite()
                    || !intrinsic_h.is_finite()
                    || *intrinsic_w < 0.001
                    || *intrinsic_h < 0.001
                    || s.w < 0.001
                    || s.h < 0.001
                    || s.fill.is_some()
                    || s.stroke.is_some()
                    || s.stroke_width != 0.
                {
                    return Err("Groupe vide ou dimensions de groupe invalides".into());
                }
                let matrix = multiply(parent, frame(s));
                if matrix.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
                    || (matrix[0] * matrix[3] - matrix[1] * matrix[2]).abs() < 1e-12
                {
                    return Err("Transformation cumulée de groupe hors limites".into());
                }
                walk(children, depth + 1, count, ids, matrix)?;
            }
        }
        Ok(())
    }
    walk(shapes, 0, &mut 0, &mut Default::default(), IDENTITY)?;
    crate::text_path::validate_links(shapes)
}
pub fn group(shapes: &mut Vec<Shape>, ids: &[u64], group_id: u64) -> Result<(), String> {
    if ids.len() < 2
        || ids.len() > 20_000
        || ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != ids.len()
    {
        return Err("Choisissez au moins deux formes distinctes".into());
    }
    let siblings = siblings_mut(shapes, ids[0]).ok_or("Forme introuvable")?;
    let indexes: Vec<_> = siblings
        .iter()
        .enumerate()
        .filter(|(_, s)| ids.contains(&s.id))
        .map(|(i, _)| i)
        .collect();
    if indexes.len() != ids.len() {
        return Err("Les formes doivent appartenir au même groupe parent".into());
    }
    let first = indexes[0];
    let last = *indexes.last().unwrap();
    if last - first + 1 != ids.len() {
        return Err(
            "Choisissez des formes consécutives pour conserver leur ordre de superposition".into(),
        );
    }
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for s in &siblings[first..=last] {
        let m = multiply(s.transform.unwrap_or(IDENTITY), rotation(s));
        let pad = if s.stroke.is_some() {
            s.stroke_width * 2.
        } else {
            0.
        };
        for p in [
            [s.x - pad, s.y - pad],
            [s.x + s.w + pad, s.y - pad],
            [s.x + s.w + pad, s.y + s.h + pad],
            [s.x - pad, s.y + s.h + pad],
        ] {
            let p = point(m, p);
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    let mut children: Vec<_> = siblings.drain(first..=last).collect();
    for s in &mut children {
        apply(s, translation(-lo[0], -lo[1]))
    }
    let (w, h) = ((hi[0] - lo[0]).max(0.001), (hi[1] - lo[1]).max(0.001));
    siblings.insert(
        first,
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
            id: group_id,
            transform: None,
            kind: ShapeKind::Group {
                children,
                intrinsic_w: w,
                intrinsic_h: h,
            },
            x: lo[0],
            y: lo[1],
            w,
            h,
            rotation: 0.,
            fill: None,
            stroke: None,
            stroke_width: 0.,
            opacity: 1.,
        },
    );
    Ok(())
}
pub fn ungroup(shapes: &mut Vec<Shape>, id: u64) -> Result<(), String> {
    let siblings = siblings_mut(shapes, id).ok_or("Groupe introuvable")?;
    let at = siblings.iter().position(|s| s.id == id).unwrap();
    let group = &siblings[at];
    let ShapeKind::Group { children, .. } = &group.kind else {
        return Err("Sélectionnez un groupe".into());
    };
    // Distribuer une opacité sur des enfants qui se recouvrent altère leur rendu.
    if group.opacity != 1. {
        return Err("Pour dissocier sans changer le rendu, remettez l’opacité du groupe à 100 % ; les opacités des enfants sont conservées".into());
    }
    if group.group_style.as_ref().is_some_and(|s| !s.neutral()) {
        return Err("Pour dissocier sans changer le rendu, remettez la fusion du groupe à Normal et désactivez son isolation ; les styles des enfants sont conservés".into());
    }
    if group.clip.is_some() {
        return Err("Pour dissocier sans perdre la découpe, retirez-la explicitement dans le panneau Découpe vectorielle".into());
    }
    if group.svg_viewport {
        return Err("Le cadre SVG préserve le rendu et les limites de ses effets ; conservez ce groupe et éditez ses enfants".into());
    }
    if group.filters.is_some() {
        return Err(
            "Pour dissocier sans perdre les filtres, retirez-les explicitement du groupe".into(),
        );
    }
    if group.mask.is_some() {
        return Err(
            "Pour dissocier sans perdre le masque, retirez-le explicitement du groupe".into(),
        );
    }
    let matrix = frame(group);
    let mut children = children.clone();
    for s in &mut children {
        apply(s, matrix)
    }
    siblings.splice(at..=at, children);
    Ok(())
}
