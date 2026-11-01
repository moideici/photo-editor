//! Strict import of the observed PhotoFiltre Path binary format.
//! The file stores geometry only, not a document's paints, size or layers.
use crate::{
    document::{BlendMode, Content, Document, Layer, Shape},
    paths::{Contour, Node},
};
const MAGIC: &[u8; 16] = b"PhotoFiltre Path";
pub fn parse(bytes: &[u8]) -> Result<Shape, String> {
    let unsupported = || {
        "PFV non pris en charge ou corrompu : signature, paramètres, sommets ou longueur inconnus. Aucun tracé partiel n’est importé.".to_string()
    };
    if bytes.len() < 60
        || bytes.len() > 640_060
        || &bytes[..16] != MAGIC
        || bytes[16..48].iter().any(|b| *b != 0)
    {
        return Err(unsupported());
    }
    let int = |at: usize| i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let (bezier, closed, count) = (int(48), int(52), int(56));
    if ![0, 1].contains(&bezier) || ![0, 1].contains(&closed) || !(2..=20_000).contains(&count) {
        return Err(unsupported());
    }
    let stride = if bezier == 1 { 32 } else { 8 };
    if bytes.len() != 60 + count as usize * stride {
        return Err(unsupported());
    }
    let point = |at: usize| -> Result<[f32; 2], String> {
        let (x, y) = (int(at), int(at + 4));
        if x.abs_diff(0) > 1_000_000 || y.abs_diff(0) > 1_000_000 {
            return Err(unsupported());
        }
        Ok([x as f32, y as f32])
    };
    let mut nodes = Vec::with_capacity(count as usize);
    let mut flags = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let at = 60 + i * stride;
        let p = point(at)?;
        let mut n = Node {
            point: p,
            incoming: p,
            outgoing: p,
        };
        let mut f = [false; 2];
        if bezier == 1 {
            n.incoming = point(at + 8)?;
            n.outgoing = point(at + 20)?;
            for (j, off) in [16, 28].into_iter().enumerate() {
                let flag = int(at + off);
                if ![0, 1].contains(&flag) {
                    return Err(unsupported());
                }
                f[j] = flag == 1;
            }
        }
        nodes.push(n);
        flags.push(f);
    }
    // A zero flag is an automatic handle, not a disabled handle. PhotoFiltre
    // places it one third toward the neighbouring anchor with integer rounding.
    if bezier == 1 {
        let anchors = nodes.iter().map(|n| n.point).collect::<Vec<_>>();
        let length = nodes.len();
        for i in 0..length {
            for (j, delta) in [(0, -1isize), (1, 1)] {
                if !flags[i][j] {
                    let neighbor = if i as isize + delta < 0 {
                        (closed == 1 && length > 2).then_some(length - 1)
                    } else if i as isize + delta >= length as isize {
                        (closed == 1 && length > 2).then_some(0)
                    } else {
                        Some((i as isize + delta) as usize)
                    };
                    let p = neighbor.map_or(anchors[i], |k| {
                        [
                            (anchors[i][0] as f64 + (anchors[k][0] - anchors[i][0]) as f64 / 3.)
                                .round_ties_even() as f32,
                            (anchors[i][1] as f64 + (anchors[k][1] - anchors[i][1]) as f64 / 3.)
                                .round_ties_even() as f32,
                        ]
                    });
                    if j == 0 {
                        nodes[i].incoming = p
                    } else {
                        nodes[i].outgoing = p
                    }
                }
            }
        }
    }
    let mut base = crate::clip::base();
    base.fill = None;
    base.stroke = Some("#000000".into());
    base.stroke_width = 1.;
    let shape = crate::paths::normalize(
        base,
        vec![Contour {
            nodes,
            closed: closed == 1,
        }],
    );
    crate::io::validate_shape(&shape)?;
    Ok(shape)
}
pub fn document(bytes: &[u8], name: &str) -> Result<Document, String> {
    let mut shape = parse(bytes)?;
    // PFV has no canvas. Translate the full control frame onto a 16px margin.
    let original = [shape.x, shape.y];
    shape.x = 16.;
    shape.y = 16.;
    shape.id = 2;
    let (w, h) = (
        (shape.w.ceil() as u32).saturating_add(32),
        (shape.h.ceil() as u32).saturating_add(32),
    );
    crate::io::dimensions(w, h)?;
    let mut metadata = crate::metadata::Metadata::default();
    metadata.notices.push(format!("PFV : géométrie seule, trait noir de 1 px attribué par Prisme. Pas de couleur ni de taille de document dans le fichier. Origine du cadre ({}, {}) replacée à (16, 16). Poignées automatiques converties en points explicites.",original[0],original[1]));
    Ok(Document {
        text_styles: vec![],
        width: w,
        height: h,
        name: name.into(),
        selection: None,
        selections: Default::default(),
        metadata,
        indexed: None,
        animation: None,
        layers: vec![Layer {
            locks: Default::default(),
            id: 1,
            name: name.into(),
            visible: true,
            opacity: 1.,
            blend: BlendMode::Normal,
            effects: Default::default(),
            x: 0,
            y: 0,
            version: 1,
            content: Content::Vector(vec![shape]),
        }],
    })
}
pub fn import(e: &mut crate::document::Editor, path: &str) -> Result<(), String> {
    if e.doc()?.layers.len() >= 256 {
        return Err("256 calques maximum".into());
    }
    if e.doc()?.indexed.is_some() || e.doc()?.animation.is_some() {
        return Err("Le PFV nécessite un document RVB fixe".into());
    }
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > 640_060 {
        return Err("PFV supérieur à la limite de 20 000 sommets".into());
    }
    let shape = parse(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    let name = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    // Import retains the PFV coordinates; opening creates an adapted canvas.
    let layer = e.new_vector_layer(&name, vec![shape]);
    e.active_hint = Some(layer.id);
    e.doc_mut()?.layers.push(layer);
    e.commit("Importer un tracé PFV");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ShapeKind;
    fn bytes(bezier: bool, closed: bool, points: &[[i32; 8]]) -> Vec<u8> {
        let mut b = vec![0; 48];
        b[..16].copy_from_slice(MAGIC);
        for n in [bezier as i32, closed as i32, points.len() as i32] {
            b.extend(n.to_le_bytes())
        }
        for p in points {
            for n in &p[..if bezier { 8 } else { 2 }] {
                b.extend(n.to_le_bytes())
            }
        }
        b
    }
    #[test]
    fn independent_cubic_reference_and_auto_handles() {
        let b = bytes(
            true,
            false,
            &[
                [10, 30, 99, 99, 0, 30, 0, 1],
                [110, 30, 90, 60, 1, 99, 99, 0],
            ],
        );
        let s = parse(&b).unwrap();
        let raw = r##"<svg xmlns="http://www.w3.org/2000/svg" width="140" height="80"><path d="M10 30 C30 0 90 60 110 30" fill="none" stroke="#000" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
        assert_eq!(
            crate::vector::rasterize(&[s], 140, 80).unwrap(),
            crate::vector::render_svg(raw, 140, 80).unwrap()
        );
        let s = parse(&bytes(
            true,
            false,
            &[[0, 0, 30, 99, 0, 99, 30, 0], [100, 100, 0, 0, 0, 0, 0, 0]],
        ))
        .unwrap();
        let ShapeKind::Path { contours, .. } = s.kind else {
            panic!()
        };
        let n = &contours[0].nodes;
        assert_eq!(n[0].incoming, n[0].point);
        assert_eq!(n[1].outgoing, n[1].point);
        assert_eq!(n[0].outgoing, [0.33, 0.33]);
        assert_eq!(n[1].incoming, [0.67, 0.67]);
    }
    #[test]
    fn polygon_and_open_line_preserve_negative_coordinates() {
        let s = parse(&bytes(
            false,
            true,
            &[
                [-10, -20, 0, 0, 0, 0, 0, 0],
                [20, -20, 0, 0, 0, 0, 0, 0],
                [10, 30, 0, 0, 0, 0, 0, 0],
            ],
        ))
        .unwrap();
        assert_eq!((s.x, s.y, s.w, s.h), (-10., -20., 30., 50.));
        let ShapeKind::Path { contours, .. } = s.kind else {
            panic!()
        };
        assert!(contours[0].closed);
        assert!(contours[0]
            .nodes
            .iter()
            .all(|n| n.incoming == n.point && n.outgoing == n.point));
        let d = document(
            &bytes(
                false,
                false,
                &[[-20, 0, 0, 0, 0, 0, 0, 0], [20, 0, 0, 0, 0, 0, 0, 0]],
            ),
            "line.pfv",
        )
        .unwrap();
        assert_eq!((d.width, d.height), (72, 33));
    }
    #[test]
    fn all_corrupt_headers_lengths_flags_coordinates_and_limits_rejected() {
        let original = bytes(
            true,
            true,
            &[
                [0, 0, 0, 0, 1, 20, 10, 1],
                [100, 100, 80, 90, 1, 100, 100, 1],
            ],
        );
        for (at, value) in [
            (0, 255),
            (16, 1),
            (48, 2),
            (52, 2),
            (56, 1),
            (76, 2),
            (88, 2),
        ] {
            let mut b = original.clone();
            b[at] = value;
            assert!(parse(&b).is_err(), "{at}");
        }
        for length in 0..original.len() {
            assert!(parse(&original[..length]).is_err());
        }
        let mut b = original.clone();
        b.push(0);
        assert!(parse(&b).is_err());
        let mut b = original;
        b[60..64].copy_from_slice(&i32::MIN.to_le_bytes());
        assert!(parse(&b).is_err());
    }
    #[test]
    fn project_roundtrip_preserves_all_pfv_geometry() {
        let b = bytes(
            true,
            true,
            &[
                [20, 0, 10, 10, 1, 30, 10, 1],
                [40, 40, 30, 30, 1, 30, 40, 1],
                [0, 40, 0, 30, 1, 10, 30, 1],
            ],
        );
        let d = document(&b, "original.pfv").unwrap();
        let loaded = crate::io::read_project(&crate::io::project_bytes(&d).unwrap()).unwrap();
        assert_eq!(
            crate::vector::flatten(&d).unwrap(),
            crate::vector::flatten(&loaded).unwrap()
        );
    }
}
