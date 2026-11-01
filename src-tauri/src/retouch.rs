//! Retouche bitmap. Un trait est calculé sur un instantané et n'engendre qu'une étape d'historique.
use crate::{document::BlendMode, raster};
use image::{Rgba, RgbaImage};
use rayon::prelude::*;

fn byte(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
fn hash(mut n: u32) -> u32 {
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846ca68b);
    n ^ (n >> 16)
}
fn random(seed: &mut u32) -> f32 {
    *seed = hash(seed.wrapping_add(0x9e3779b9));
    *seed as f32 / u32::MAX as f32
}

fn samples(points: &[[f32; 2]], spacing: f32, stationary: bool) -> Vec<[f32; 2]> {
    let Some(first) = points.first() else {
        return vec![];
    };
    let mut out = vec![*first];
    let mut next = spacing;
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        let distance = (b[0] - a[0]).hypot(b[1] - a[1]);
        if distance < 0.0001 {
            if stationary {
                out.push(b);
            }
            continue;
        }
        while next <= distance {
            let t = next / distance;
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            next += spacing;
        }
        next -= distance;
    }
    if let Some(last) = points.last() {
        if out
            .last()
            .is_some_and(|p| (p[0] - last[0]).hypot(p[1] - last[1]) > 0.01)
        {
            out.push(*last);
        }
    }
    out
}

fn weight(x: f32, y: f32, center: [f32; 2], radius: f32) -> f32 {
    let distance = (x - center[0]).hypot(y - center[1]);
    ((radius + 0.5 - distance) / (radius * 0.4).max(0.5)).clamp(0.0, 1.0)
}
fn area(center: [f32; 2], radius: f32, w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (center[0] - radius - 1.0).floor().clamp(0.0, w as f32) as u32,
        (center[1] - radius - 1.0).floor().clamp(0.0, h as f32) as u32,
        (center[0] + radius + 1.0).ceil().clamp(0.0, w as f32) as u32,
        (center[1] + radius + 1.0).ceil().clamp(0.0, h as f32) as u32,
    )
}
fn coverage(points: &[[f32; 2]], radius: f32, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0; w as usize * h as usize];
    for center in samples(points, (radius * 0.2).max(0.5), false) {
        let (x0, y0, x1, y1) = area(center, radius, w, h);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * w + x) as usize;
                out[i] = out[i].max(byte(
                    weight(x as f32 + 0.5, y as f32 + 0.5, center, radius) * 255.0,
                ));
            }
        }
    }
    out
}
fn sample(img: &RgbaImage, x: f32, y: f32) -> Option<[u8; 4]> {
    if x < 0.0 || y < 0.0 || x > img.width() as f32 - 1.0 || y > img.height() as f32 - 1.0 {
        return None;
    }
    let (ix, iy) = (x.floor() as u32, y.floor() as u32);
    let (tx, ty) = (x - x.floor(), y - y.floor());
    let mut sum = [0f32; 4];
    for (dx, dy, k) in [
        (0, 0, (1.0 - tx) * (1.0 - ty)),
        (1, 0, tx * (1.0 - ty)),
        (0, 1, (1.0 - tx) * ty),
        (1, 1, tx * ty),
    ] {
        let p = img.get_pixel(
            (ix + dx).min(img.width() - 1),
            (iy + dy).min(img.height() - 1),
        );
        let a = p[3] as f32 / 255.0;
        for c in 0..3 {
            sum[c] += p[c] as f32 * a * k;
        }
        sum[3] += a * k;
    }
    if sum[3] <= 0.0 {
        Some([0; 4])
    } else {
        Some([
            byte(sum[0] / sum[3]),
            byte(sum[1] / sum[3]),
            byte(sum[2] / sum[3]),
            byte(sum[3] * 255.0),
        ])
    }
}

pub struct Settings<'a> {
    pub tool: &'a str,
    pub points: &'a [[f32; 2]],
    pub size: f32,
    pub strength: f32,
    pub source_offset: Option<[f32; 2]>,
    pub color: [u8; 4],
    pub seed: u32,
    pub nozzle: &'a str,
    pub stamp: Option<&'a RgbaImage>,
}

pub fn apply(original: &RgbaImage, p: &Settings<'_>) -> Result<RgbaImage, String> {
    if p.points.is_empty() {
        return Ok(original.clone());
    }
    if !p.size.is_finite()
        || !(1.0..=1000.0).contains(&p.size)
        || !p.strength.is_finite()
        || !(0.0..=1.0).contains(&p.strength)
        || p.points.len() > 100_000
        || p.points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err("Paramètres du trait de retouche invalides".into());
    }
    let (w, h) = original.dimensions();
    let radius = p.size / 2.0;
    let mut out = original.clone();
    match p.tool {
        "blur" => {
            let blurred = raster::gaussian_blur(original, (radius * 0.25).clamp(0.4, 12.0));
            let mask = coverage(p.points, radius, w, h);
            out.par_chunks_mut(4)
                .zip(blurred.par_chunks(4))
                .zip(mask.par_iter())
                .for_each(|((pixel, blur), &m)| {
                    let amount = p.strength * m as f32 / 255.;
                    for c in 0..3 {
                        pixel[c] = byte(pixel[c] as f32 * (1. - amount) + blur[c] as f32 * amount);
                    }
                });
        }
        "clone" | "heal" => {
            let offset = p
                .source_offset
                .ok_or("Alt + clic définit le point source du tampon ou du correcteur")?;
            if offset.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                return Err("Point source invalide".into());
            }
            let mask = coverage(p.points, radius, w, h);
            let low = if p.tool == "heal" {
                Some(raster::gaussian_blur(original, (radius * 0.5).max(1.0)))
            } else {
                None
            };
            out.par_chunks_mut(4)
                .zip(mask.par_iter())
                .enumerate()
                .for_each(|(i, (pixel, &mask))| {
                    if mask == 0 {
                        return;
                    }
                    let (x, y) = ((i as u32 % w) as f32, (i as u32 / w) as f32);
                    let Some(mut source) = sample(original, x + offset[0], y + offset[1]) else {
                        return;
                    };
                    if let Some(low) = &low {
                        if let Some(source_low) = sample(low, x + offset[0], y + offset[1]) {
                            let target_low = low.get_pixel(x as u32, y as u32);
                            for c in 0..3 {
                                source[c] = byte(
                                    source[c] as f32 + target_low[c] as f32 - source_low[c] as f32,
                                );
                            }
                        }
                    }
                    let blended = raster::blend_pixel(
                        pixel.try_into().unwrap(),
                        source,
                        p.strength * mask as f32 / 255.0,
                        BlendMode::Normal,
                    );
                    pixel.copy_from_slice(&blended);
                });
        }
        "dodge" | "burn" => {
            let mask = coverage(p.points, radius, w, h);
            let sign = if p.tool == "dodge" { 1.0 } else { -1.0 };
            out.par_chunks_mut(4)
                .zip(mask.par_iter())
                .for_each(|(pixel, &mask)| {
                    let factor = 2.0f32.powf(sign * p.strength * mask as f32 / 255.0);
                    for c in &mut pixel[..3] {
                        *c = byte(
                            ((*c as f32 / 255.0).powf(2.2) * factor)
                                .max(0.0)
                                .powf(1.0 / 2.2)
                                * 255.0,
                        );
                    }
                });
        }
        "smudge" => {
            let r = radius.ceil() as i32;
            let side = r as usize * 2 + 1;
            let first = p.points[0];
            let mut carried = Vec::with_capacity(side * side);
            for dy in -r..=r {
                for dx in -r..=r {
                    carried.push(
                        sample(
                            original,
                            first[0] + dx as f32 - 0.5,
                            first[1] + dy as f32 - 0.5,
                        )
                        .unwrap_or([0; 4]),
                    );
                }
            }
            for center in samples(p.points, (radius * 0.15).max(1.0), false)
                .into_iter()
                .skip(1)
            {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (x, y) = (
                            (center[0] + dx as f32).floor() as i32,
                            (center[1] + dy as f32).floor() as i32,
                        );
                        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                            continue;
                        }
                        let amount =
                            weight(x as f32 + 0.5, y as f32 + 0.5, center, radius) * p.strength;
                        if amount <= 0.0 {
                            continue;
                        }
                        let i = ((dy + r) as usize * side) + (dx + r) as usize;
                        let target = out.get_pixel(x as u32, y as u32).0;
                        let result =
                            raster::blend_pixel(target, carried[i], amount, BlendMode::Normal);
                        out.put_pixel(x as u32, y as u32, Rgba(result));
                        carried[i] =
                            raster::blend_pixel(carried[i], target, 0.15, BlendMode::Normal);
                    }
                }
            }
        }
        "airbrush" => {
            let mut seed = p.seed;
            let count = (radius * radius * 0.08).round().clamp(2.0, 1500.0) as usize;
            for center in samples(p.points, (radius * 0.3).max(1.0), true) {
                for _ in 0..count {
                    let r = random(&mut seed).sqrt() * radius;
                    let angle = random(&mut seed) * std::f32::consts::TAU;
                    let (x, y) = (
                        (center[0] + r * angle.cos()).floor() as i32,
                        (center[1] + r * angle.sin()).floor() as i32,
                    );
                    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                        continue;
                    }
                    let current = out.get_pixel(x as u32, y as u32).0;
                    out.put_pixel(
                        x as u32,
                        y as u32,
                        Rgba(raster::blend_pixel(
                            current,
                            p.color,
                            p.strength * 0.3,
                            BlendMode::Normal,
                        )),
                    );
                }
            }
        }
        "nozzle" => {
            let stamp = if let Some(stamp) = p.stamp {
                let w = p.size.ceil() as u32;
                let h = (w as f32 * stamp.height() as f32 / stamp.width() as f32)
                    .round()
                    .clamp(1.0, 1000.0) as u32;
                image::imageops::resize(stamp, w, h, image::imageops::FilterType::Lanczos3)
            } else {
                nozzle(p.nozzle, p.size.ceil() as u32, p.color)?
            };
            for center in samples(p.points, (p.size * 0.8).max(1.0), false) {
                raster::composite(
                    &mut out,
                    &stamp,
                    (center[0] - stamp.width() as f32 / 2.0).round() as i32,
                    (center[1] - stamp.height() as f32 / 2.0).round() as i32,
                    p.strength,
                    BlendMode::Normal,
                );
            }
        }
        _ => return Err("Outil de retouche inconnu".into()),
    }
    Ok(out)
}

fn nozzle(kind: &str, size: u32, color: [u8; 4]) -> Result<RgbaImage, String> {
    if !["star", "leaf", "bubble"].contains(&kind) {
        return Err("Tube inconnu".into());
    }
    Ok(RgbaImage::from_fn(size, size, |x, y| {
        let mut coverage = 0.0;
        for sy in 0..2 {
            for sx in 0..2 {
                let px = (x as f32 + (sx as f32 + 0.5) / 2.0) / size as f32 * 2.0 - 1.0;
                let py = (y as f32 + (sy as f32 + 0.5) / 2.0) / size as f32 * 2.0 - 1.0;
                let r = px.hypot(py);
                let inside = match kind {
                    "bubble" => r < 0.85 && r > 0.6,
                    "leaf" => ((px + py) * 0.75).powi(2) + ((px - py) * 1.5).powi(2) < 0.8,
                    _ => {
                        let a = py.atan2(px) + std::f32::consts::FRAC_PI_2;
                        let sector = (a / std::f32::consts::TAU * 5.0).rem_euclid(1.0);
                        r < 0.38 + 0.5 * (1.0 - (sector * 2.0 - 1.0).abs())
                    }
                };
                if inside {
                    coverage += 0.25;
                }
            }
        }
        let mut p = color;
        p[3] = byte(p[3] as f32 * coverage);
        Rgba(p)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings<'a>(tool: &'a str, points: &'a [[f32; 2]]) -> Settings<'a> {
        Settings {
            tool,
            points,
            size: 3.0,
            strength: 1.0,
            source_offset: Some([-4.0, 0.0]),
            color: [0, 0, 0, 255],
            seed: 42,
            nozzle: "star",
            stamp: None,
        }
    }
    #[test]
    fn clone_samples_original_without_feedback() {
        let img = RgbaImage::from_fn(12, 3, |x, _| Rgba([(x * 20) as u8, 0, 0, 255]));
        let points = [[4.5, 1.5], [9.5, 1.5]];
        let out = apply(&img, &settings("clone", &points)).unwrap();
        assert_eq!(out.get_pixel(8, 1).0, img.get_pixel(4, 1).0);
        assert_eq!(out.get_pixel(4, 1).0, img.get_pixel(0, 1).0);
    }
    #[test]
    fn local_blur_changes_only_covered_rgb_and_keeps_alpha() {
        let img = RgbaImage::from_fn(20, 10, |x, _| {
            if x < 10 {
                Rgba([0, 0, 0, 128])
            } else {
                Rgba([255, 255, 255, 128])
            }
        });
        let points = [[9.5, 5.5]];
        let mut p = settings("blur", &points);
        p.size = 8.0;
        let out = apply(&img, &p).unwrap();
        assert!(out.get_pixel(9, 5)[0] > 0);
        assert_eq!(out.get_pixel(0, 5), img.get_pixel(0, 5));
        assert!(out.pixels().all(|p| p[3] == 128));
    }
    #[test]
    fn heal_transfers_texture_while_matching_destination_tone() {
        let mut img = RgbaImage::from_fn(18, 7, |x, _| {
            Rgba(if x < 9 {
                [80, 40, 20, 255]
            } else {
                [180, 140, 120, 255]
            })
        });
        img.put_pixel(3, 3, Rgba([100, 60, 40, 255]));
        let points = [[12.5, 3.5]];
        let mut p = settings("heal", &points);
        p.size = 5.0;
        p.source_offset = Some([-9.0, 0.0]);
        let out = apply(&img, &p).unwrap();
        assert!(out.get_pixel(12, 3)[0] > 180);
        assert!(out.get_pixel(12, 3)[0] < 205);
        assert_eq!(out.get_pixel(12, 3)[3], 255);
    }
    #[test]
    fn exposure_preserves_alpha_and_is_independent_of_pointer_density() {
        let img = RgbaImage::from_pixel(12, 4, Rgba([120, 90, 60, 128]));
        let a = [[2.5, 1.5], [9.5, 1.5]];
        let b = [[2.5, 1.5], [4.5, 1.5], [6.5, 1.5], [9.5, 1.5]];
        let light = apply(&img, &settings("dodge", &a)).unwrap();
        assert!(light.get_pixel(5, 1)[0] > 120);
        assert_eq!(light.get_pixel(5, 1)[3], 128);
        assert_eq!(light, apply(&img, &settings("dodge", &b)).unwrap());
        assert!(apply(&img, &settings("burn", &a)).unwrap().get_pixel(5, 1)[0] < 120);
    }
    #[test]
    fn smudge_drags_color_across_a_boundary() {
        let img = RgbaImage::from_fn(20, 7, |x, _| {
            if x < 8 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let points = [[5.5, 3.5], [13.5, 3.5]];
        let out = apply(&img, &settings("smudge", &points)).unwrap();
        assert!(out.get_pixel(10, 3)[1] < 255);
    }
    #[test]
    fn airbrush_accumulates_and_nozzles_are_original_alpha_stamps() {
        let img = RgbaImage::new(30, 30);
        let points = [[15.5, 15.5]];
        let mut p = settings("airbrush", &points);
        p.size = 20.0;
        let a = apply(&img, &p).unwrap();
        let held = [[15.5, 15.5]; 6];
        p.points = &held;
        let b = apply(&img, &p).unwrap();
        assert!(
            b.pixels().map(|p| p[3] as u32).sum::<u32>()
                > a.pixels().map(|p| p[3] as u32).sum::<u32>()
        );
        for kind in ["star", "leaf", "bubble"] {
            let stamp = nozzle(kind, 21, [30, 60, 90, 255]).unwrap();
            assert!(stamp.pixels().any(|p| p[3] > 0));
            assert_eq!(stamp.get_pixel(0, 0)[3], 0);
        }
    }
}
