//! Masques de sélection partagés : géométrie, couleur et opérations morphologiques.
use crate::document::{SelShape, Selection};
use image::{GrayImage, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};

pub mod mask_data {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(data: &Arc<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(data.as_slice()))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<Vec<u8>>, D::Error> {
        let encoded = String::deserialize(d)?;
        if encoded.len() > 24 * 1024 * 1024 {
            return Err(serde::de::Error::custom("Masque trop volumineux"));
        }
        STANDARD
            .decode(encoded)
            .map(Arc::new)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Combine {
    #[default]
    Replace,
    Add,
    Subtract,
    Intersect,
}

pub fn validate(selection: &Selection, w: u32, h: u32) -> Result<(), String> {
    match &selection.shape {
        SelShape::Mask {
            width,
            height,
            data,
        } => {
            crate::io::dimensions(*width, *height)?;
            if data.len() != *width as usize * *height as usize {
                return Err("Taille de masque incohérente".into());
            }
        }
        SelShape::Rect { x, y, w, h } | SelShape::Ellipse { x, y, w, h } => {
            if [x, y, w, h].iter().any(|v| !v.is_finite() || v.abs() > 1e6) || *w < 0.0 || *h < 0.0
            {
                return Err("Sélection géométrique invalide".into());
            }
        }
        SelShape::Polygon { points } => {
            if !(3..=100_000).contains(&points.len())
                || points
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 1e6)
            {
                return Err("Polygone de sélection invalide".into());
            }
        }
    }
    crate::io::dimensions(w, h)
}

pub fn from_mask(width: u32, height: u32, data: Vec<u8>) -> Selection {
    debug_assert_eq!(data.len(), width as usize * height as usize);
    Selection {
        shape: SelShape::Mask {
            width,
            height,
            data: Arc::new(data),
        },
        inverted: false,
    }
}

pub fn bounds(data: &[u8], w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (i, &v) in data.iter().enumerate() {
        if v > 0 {
            let (x, y) = (i as u32 % w, i as u32 / w);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)))
}

pub fn combine(
    current: Option<&Selection>,
    incoming: Selection,
    w: u32,
    h: u32,
    mode: Combine,
) -> Selection {
    if matches!(mode, Combine::Replace) {
        return incoming;
    }
    let a = current.map_or_else(|| vec![0; w as usize * h as usize], |s| s.mask(w, h));
    let mut b = incoming.mask(w, h);
    b.par_iter_mut().zip(a.par_iter()).for_each(|(b, a)| {
        *b = match mode {
            Combine::Add => (*a).max(*b),
            Combine::Subtract => a.saturating_sub(*b),
            Combine::Intersect => (*a).min(*b),
            Combine::Replace => *b,
        }
    });
    from_mask(w, h, b)
}

fn matches_color(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    let premul = |p: [u8; 4]| {
        [
            (p[0] as u16 * p[3] as u16 / 255) as u8,
            (p[1] as u16 * p[3] as u16 / 255) as u8,
            (p[2] as u16 * p[3] as u16 / 255) as u8,
            p[3],
        ]
    };
    premul(a)
        .iter()
        .zip(premul(b))
        .all(|(a, b)| a.abs_diff(b) <= tolerance)
}

pub fn by_color(
    img: &RgbaImage,
    x: i32,
    y: i32,
    tolerance: u8,
    contiguous: bool,
) -> Result<Selection, String> {
    let (w, h) = img.dimensions();
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return Err("Point de sélection hors de l'image".into());
    }
    let target = img.get_pixel(x as u32, y as u32).0;
    let mut mask = vec![0; w as usize * h as usize];
    if !contiguous {
        mask.par_iter_mut()
            .zip(img.par_chunks(4))
            .for_each(|(m, p)| {
                if matches_color(p.try_into().unwrap(), target, tolerance) {
                    *m = 255;
                }
            });
    } else {
        let mut seen = vec![false; mask.len()];
        let mut stack = vec![y as usize * w as usize + x as usize];
        while let Some(i) = stack.pop() {
            if seen[i] {
                continue;
            }
            seen[i] = true;
            if !matches_color(
                img.get_pixel(i as u32 % w, i as u32 / w).0,
                target,
                tolerance,
            ) {
                continue;
            }
            mask[i] = 255;
            let (x, y) = (i as u32 % w, i as u32 / w);
            if x > 0 {
                stack.push(i - 1);
            }
            if x + 1 < w {
                stack.push(i + 1);
            }
            if y > 0 {
                stack.push(i - w as usize);
            }
            if y + 1 < h {
                stack.push(i + w as usize);
            }
        }
    }
    Ok(from_mask(w, h, mask))
}

// Extrema d'une fenêtre glissante, O(n), avec extérieur non sélectionné.
fn extrema_line(input: &[u8], radius: usize, dilate: bool) -> Vec<u8> {
    let n = input.len();
    let mut out = vec![0; n];
    let mut deque: VecDeque<(isize, u8)> = VecDeque::new();
    let r = radius as isize;
    for i in -r..n as isize + r {
        let value = if i < 0 || i >= n as isize {
            0
        } else {
            input[i as usize]
        };
        while deque
            .back()
            .is_some_and(|&(_, v)| if dilate { v <= value } else { v >= value })
        {
            deque.pop_back();
        }
        deque.push_back((i, value));
        let center = i - r;
        while deque.front().is_some_and(|&(index, _)| index < center - r) {
            deque.pop_front();
        }
        if center >= 0 && center < n as isize {
            out[center as usize] = deque.front().unwrap().1;
        }
    }
    out
}

fn morphology(data: &[u8], w: u32, h: u32, radius: usize, dilate: bool) -> Vec<u8> {
    let mut rows = vec![0; data.len()];
    rows.par_chunks_mut(w as usize)
        .zip(data.par_chunks(w as usize))
        .for_each(|(out, row)| out.copy_from_slice(&extrema_line(row, radius, dilate)));
    let columns: Vec<Vec<u8>> = (0..w)
        .into_par_iter()
        .map(|x| {
            extrema_line(
                &(0..h)
                    .map(|y| rows[(y * w + x) as usize])
                    .collect::<Vec<_>>(),
                radius,
                dilate,
            )
        })
        .collect();
    let mut out = vec![0; data.len()];
    out.par_chunks_mut(w as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w as usize {
                row[x] = columns[x][y];
            }
        });
    out
}

pub fn modify(
    selection: &Selection,
    w: u32,
    h: u32,
    kind: &str,
    radius: f32,
) -> Result<Selection, String> {
    if !radius.is_finite() || !(0.0..=256.0).contains(&radius) {
        return Err("Rayon de sélection : 0 à 256 pixels".into());
    }
    let data = selection.mask(w, h);
    let r = radius.ceil() as usize;
    let data = match kind {
        "feather" => {
            if radius == 0.0 {
                data
            } else {
                image::imageops::blur(&GrayImage::from_raw(w, h, data).unwrap(), radius).into_raw()
            }
        }
        "grow" => morphology(&data, w, h, r, true),
        "shrink" => morphology(&data, w, h, r, false),
        "border" => {
            let mut outer = morphology(&data, w, h, r, true);
            let inner = morphology(&data, w, h, r, false);
            outer
                .par_iter_mut()
                .zip(inner)
                .for_each(|(o, i)| *o = o.saturating_sub(i));
            outer
        }
        _ => return Err("Modification de sélection inconnue".into()),
    };
    Ok(from_mask(w, h, data))
}

pub fn overlay(selection: &Selection, w: u32, h: u32) -> RgbaImage {
    let mask = selection.mask(w, h);
    RgbaImage::from_fn(w, h, |x, y| {
        let i = (y * w + x) as usize;
        let edge = mask[i] >= 128
            && (x == 0
                || y == 0
                || x + 1 == w
                || y + 1 == h
                || mask[i - 1] < 128
                || mask[i + 1] < 128
                || mask[i - w as usize] < 128
                || mask[i + w as usize] < 128);
        if edge {
            let c = if (x + y) % 8 < 4 { 255 } else { 0 };
            image::Rgba([c, c, c, 255])
        } else {
            image::Rgba([0; 4])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    #[test]
    fn wand_stays_connected_while_color_selection_does_not() {
        let mut img = RgbaImage::from_pixel(5, 3, Rgba([255, 255, 255, 255]));
        for y in 0..3 {
            img.put_pixel(2, y, Rgba([0, 0, 0, 255]));
        }
        assert_eq!(
            by_color(&img, 0, 1, 0, true)
                .unwrap()
                .mask(5, 3)
                .iter()
                .filter(|&&m| m == 255)
                .count(),
            6
        );
        assert_eq!(
            by_color(&img, 0, 1, 0, false)
                .unwrap()
                .mask(5, 3)
                .iter()
                .filter(|&&m| m == 255)
                .count(),
            12
        );
        assert!(by_color(&img, -1, 0, 0, true).is_err());
    }
    #[test]
    fn hidden_rgb_does_not_break_transparent_color_selection() {
        let img = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 0])
            } else {
                Rgba([0, 255, 0, 0])
            }
        });
        assert_eq!(
            by_color(&img, 0, 0, 0, false).unwrap().mask(2, 1),
            vec![255, 255]
        );
    }
    #[test]
    fn mask_combines_coverage_and_survives_serialization() {
        let a = from_mask(3, 1, vec![255, 100, 0]);
        let b = from_mask(3, 1, vec![0, 200, 255]);
        assert_eq!(
            combine(Some(&a), b.clone(), 3, 1, Combine::Add).mask(3, 1),
            vec![255, 200, 255]
        );
        assert_eq!(
            combine(Some(&a), b.clone(), 3, 1, Combine::Subtract).mask(3, 1),
            vec![255, 0, 0]
        );
        assert_eq!(
            combine(Some(&a), b, 3, 1, Combine::Intersect).mask(3, 1),
            vec![0, 100, 0]
        );
        let bytes = serde_json::to_vec(&a).unwrap();
        let restored: Selection = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.mask(3, 1), a.mask(3, 1));
    }
    #[test]
    fn morphology_and_feather_change_real_coverage() {
        let mut data = vec![0; 49];
        data[24] = 255;
        let s = from_mask(7, 7, data);
        let grown = modify(&s, 7, 7, "grow", 1.0).unwrap();
        assert_eq!(grown.mask(7, 7).iter().filter(|&&m| m == 255).count(), 9);
        assert_eq!(
            modify(&grown, 7, 7, "shrink", 1.0).unwrap().mask(7, 7),
            s.mask(7, 7)
        );
        let blurred = modify(&grown, 7, 7, "feather", 1.0).unwrap().mask(7, 7);
        assert!(blurred[24] > 0 && blurred[24] < 255);
        assert!(blurred[16] > 0 && blurred[16] < 255);
        let border = modify(&grown, 7, 7, "border", 1.0).unwrap().mask(7, 7);
        assert_eq!(border[24], 0);
        assert_eq!(border[8], 255);
    }
    #[test]
    fn inverted_bounds_follow_pixels_instead_of_always_returning_canvas() {
        let mut s = from_mask(3, 2, vec![255, 255, 0, 255, 255, 0]);
        s.inverted = true;
        assert_eq!(s.bounds(3, 2), Some((2, 0, 1, 2)));
        let s = from_mask(2, 1, vec![0, 0]);
        assert_eq!(s.bounds(2, 1), None);
        assert_eq!(overlay(&s, 2, 1).get_pixel(0, 0)[3], 0);
    }
}
