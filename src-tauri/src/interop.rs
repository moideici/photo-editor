//! Import de sélections PFS : nombre de sommets, puis lignes x;y.
//! Grammaire observée sur les fichiers de référence officiels, aucun asset embarqué.
use crate::document::{SelShape, Selection};
pub fn parse_pfs(
    bytes: &[u8],
    width: u32,
    height: u32,
    target: Option<(u32, u32, u32, u32)>,
) -> Result<Selection, String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Sélection PFS trop volumineuse".into());
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) {
        if bytes.len() % 2 != 0 {
            return Err("Encodage PFS invalide".into());
        }
        String::from_utf16(
            &bytes[2..]
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.to_string())?
    } else {
        std::str::from_utf8(bytes)
            .map_err(|e| e.to_string())?
            .trim_start_matches('\u{feff}')
            .to_owned()
    };
    let mut lines = text.lines().map(str::trim).filter(|s| !s.is_empty());
    let count = lines
        .next()
        .ok_or("PFS vide")?
        .parse::<usize>()
        .map_err(|_| "En-tête PFS invalide")?;
    if !(3..=100_000).contains(&count) {
        return Err("Le PFS doit contenir 3 à 100 000 sommets".into());
    }
    let mut points = Vec::with_capacity(count);
    for line in lines {
        let (a, b) = line
            .split_once(';')
            .ok_or("Point PFS invalide (x;y attendu)")?;
        let x = a
            .trim()
            .parse::<f32>()
            .map_err(|_| "Coordonnée PFS invalide")?;
        let y = b
            .trim()
            .parse::<f32>()
            .map_err(|_| "Coordonnée PFS invalide")?;
        if !x.is_finite() || !y.is_finite() || x.abs() > 1e6 || y.abs() > 1e6 {
            return Err("Coordonnée PFS hors limites".into());
        }
        points.push([x, y]);
        if points.len() > count {
            return Err("Nombre de sommets PFS incohérent".into());
        }
    }
    if points.len() != count {
        return Err("PFS tronqué : sommets manquants".into());
    }
    let (minx, miny, maxx, maxy) = points.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(a, b, c, d), p| (a.min(p[0]), b.min(p[1]), c.max(p[0]), d.max(p[1])),
    );
    let (w, h) = (maxx - minx, maxy - miny);
    if w <= 0.0 || h <= 0.0 {
        return Err("Sélection PFS sans surface".into());
    }
    let (x, y, tw, th) = target.unwrap_or((0, 0, width, height));
    let scale = (tw as f32 / w).min(th as f32 / h).min(1.0);
    let (ox, oy) = (
        x as f32 + (tw as f32 - w * scale) / 2.0,
        y as f32 + (th as f32 - h * scale) / 2.0,
    );
    for p in &mut points {
        p[0] = ox + (p[0] - minx) * scale;
        p[1] = oy + (p[1] - miny) * scale;
    }
    Ok(Selection {
        shape: SelShape::Polygon { points },
        inverted: false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_synthetic_polygon_and_fits_document() {
        let selection = parse_pfs(b"4\r\n0;0\r\n20;0\r\n20;20\r\n0;20\r\n", 10, 10, None).unwrap();
        assert_eq!(selection.bounds(10, 10), Some((0, 0, 10, 10)));
        assert_eq!(selection.mask(10, 10)[55], 255);
    }
    #[test]
    fn rejects_truncated_and_invalid_shapes() {
        for bytes in [
            b"4\n0;0\n20;0\n20;20".as_slice(),
            b"3\nNaN;0\n2;0\n3;3",
            b"3\n0;0\n0;1\n0;2",
        ] {
            assert!(parse_pfs(bytes, 10, 10, None).is_err());
        }
    }
}
