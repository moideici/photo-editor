//! Moteur bitmap : réglages, filtres, fusion, remplissage, dégradés, rotations.
//! Toutes les images sont en RGBA 8 bits non prémultiplié.

use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use serde_json::Value;

use crate::document::BlendMode;

// ---------------------------------------------------------------- paramètres

fn num(p: &Value, key: &str, default: f32) -> f32 {
    p.get(key)
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .unwrap_or(default)
}

fn flag(p: &Value, key: &str) -> bool {
    p.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn parse_color(s: &str) -> Option<[u8; 4]> {
    let h = s.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some([byte(0)?, byte(2)?, byte(4)?, 255]),
        8 => Some([byte(0)?, byte(2)?, byte(4)?, byte(6)?]),
        _ => None,
    }
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn lum(px: [u8; 4]) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

fn map_pixels<F: Fn([u8; 4]) -> [u8; 4] + Sync>(img: &RgbaImage, f: F) -> RgbaImage {
    let mut out = img.clone();
    let token=crate::preview_jobs::current();
    out.par_chunks_mut(4096).for_each(|chunk| {
      if token.cancelled(){return;}
      for px in chunk.chunks_exact_mut(4){
        let r = f([px[0], px[1], px[2], px[3]]);
        px.copy_from_slice(&r);
      }
    });
    out
}
#[cfg(test)]
pub(crate) fn map_preview_test<F:Fn([u8;4])->[u8;4]+Sync>(img:&RgbaImage,f:F)->RgbaImage{map_pixels(img,f)}

// ---------------------------------------------------------------- couleurs

pub(crate) fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hue(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s <= 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    (
        hue(p, q, h + 1.0 / 3.0),
        hue(p, q, h),
        hue(p, q, h - 1.0 / 3.0),
    )
}

// ---------------------------------------------------------------- flou

/// Tampon RGBA flottant prémultiplié : évite les halos sombres autour des zones transparentes.
fn to_premul(img: &RgbaImage) -> Vec<[f32; 4]> {
    img.pixels()
        .map(|p| {
            let a = p[3] as f32 / 255.0;
            [
                p[0] as f32 * a,
                p[1] as f32 * a,
                p[2] as f32 * a,
                p[3] as f32,
            ]
        })
        .collect()
}

fn from_premul(buf: &[[f32; 4]], w: u32, h: u32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let mut out = RgbaImage::new(w, h);
    out.par_chunks_mut(4)
        .zip(buf.par_iter())
        .for_each(|(px, v)| {
            if cancel_task.cancelled(){return;}
            let a = v[3].clamp(0.0, 255.0);
            if a <= 0.0 {
                px.copy_from_slice(&[0, 0, 0, 0]);
            } else {
                let k = 255.0 / a;
                px.copy_from_slice(&[
                    clamp8(v[0] * k),
                    clamp8(v[1] * k),
                    clamp8(v[2] * k),
                    clamp8(a),
                ]);
            }
        });
    out
}

fn box_blur_rows(buf: &mut [[f32; 4]], w: usize, r: usize) {
    if r == 0 || w == 0 {
        return;
    }
    let token=crate::preview_jobs::current();
    buf.par_chunks_mut(w).for_each(|row| {
        if token.cancelled(){return;}
        let src = row.to_vec();
        let n = (2 * r + 1) as f32;
        let at = |i: isize| src[i.clamp(0, w as isize - 1) as usize];
        let mut acc = [0f32; 4];
        for i in -(r as isize)..=(r as isize) {
            let p = at(i);
            for c in 0..4 {
                acc[c] += p[c];
            }
        }
        for x in 0..w {
            for c in 0..4 {
                row[x][c] = acc[c] / n;
            }
            let add = at(x as isize + r as isize + 1);
            let sub = at(x as isize - r as isize);
            for c in 0..4 {
                acc[c] += add[c] - sub[c];
            }
        }
    });
}

fn transpose(buf: &[[f32; 4]], w: usize, h: usize) -> Vec<[f32; 4]> {
    let mut out = vec![[0f32; 4]; w * h];
    let token=crate::preview_jobs::current();
    out.par_chunks_mut(h).enumerate().for_each(|(x, col)| {
        if token.cancelled(){return;}
        for y in 0..h {
            col[y] = buf[y * w + x];
        }
    });
    out
}

fn gauss_boxes(sigma: f32) -> [usize; 3] {
    let n = 3.0;
    let w_ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let wlf = wl as f32;
    let m = ((12.0 * sigma * sigma - n * wlf * wlf - 4.0 * n * wlf - 3.0 * n) / (-4.0 * wlf - 4.0))
        .round() as i32;
    let r = |i: i32| (((if i < m { wl } else { wu }).max(1) - 1) / 2) as usize;
    [r(0), r(1), r(2)]
}

fn blur_premul(buf: &mut Vec<[f32; 4]>, w: usize, h: usize, sigma: f32) {
    let token=crate::preview_jobs::current();
    if sigma < 0.3 {
        return;
    }
    let boxes = gauss_boxes(sigma);
    for r in boxes {
        if token.cancelled(){return;}
        box_blur_rows(buf, w, r);
    }
    if token.cancelled(){return;}
    let mut t = transpose(buf, w, h);
    for r in boxes {
        if token.cancelled(){return;}
        box_blur_rows(&mut t, h, r);
    }
    if token.cancelled(){return;}
    *buf = transpose(&t, h, w);
}

pub fn gaussian_blur(img: &RgbaImage, sigma: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut buf = to_premul(img);
    blur_premul(&mut buf, w as usize, h as usize, sigma);
    from_premul(&buf, w, h)
}

fn motion_blur(img: &RgbaImage, angle_deg: f32, distance: f32) -> RgbaImage {
    let token=crate::preview_jobs::current();
    let (w, h) = img.dimensions();
    let src = to_premul(img);
    let steps = distance.round().max(1.0) as i32;
    let (dx, dy) = (angle_deg.to_radians().cos(), -angle_deg.to_radians().sin());
    let mut out = vec![[0f32; 4]; (w * h) as usize];
    out.par_chunks_mut(w as usize)
        .enumerate()
        .for_each(|(y, row)| {
            if token.cancelled(){return;}
            for (x, px) in row.iter_mut().enumerate() {
                let mut acc = [0f32; 4];
                for s in -steps / 2..=steps / 2 {
                    let sx = (x as f32 + dx * s as f32)
                        .round()
                        .clamp(0.0, w as f32 - 1.0) as usize;
                    let sy = (y as f32 + dy * s as f32)
                        .round()
                        .clamp(0.0, h as f32 - 1.0) as usize;
                    let p = src[sy * w as usize + sx];
                    for c in 0..4 {
                        acc[c] += p[c];
                    }
                }
                let n = (steps / 2 * 2 + 1) as f32;
                *px = [acc[0] / n, acc[1] / n, acc[2] / n, acc[3] / n];
            }
        });
    from_premul(&out, w, h)
}

fn unsharp(img: &RgbaImage, sigma: f32, amount: f32, threshold: f32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let blurred = gaussian_blur(img, sigma);
    let mut out = img.clone();
    out.par_chunks_mut(4)
        .zip(blurred.par_chunks(4))
        .for_each(|(px, b)| {
            if cancel_task.cancelled(){return;}
            for c in 0..3 {
                let d = px[c] as f32 - b[c] as f32;
                if d.abs() >= threshold {
                    px[c] = clamp8(px[c] as f32 + d * amount);
                }
            }
        });
    out
}

// ---------------------------------------------------------------- convolutions

fn convolve3(img: &RgbaImage, k: [f32; 9], bias: f32, gray: bool) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            for x in 0..w as usize {
                let mut acc = [0f32; 3];
                for ky in 0..3 {
                    for kx in 0..3 {
                        let sx = (x as i32 + kx - 1).clamp(0, w as i32 - 1) as u32;
                        let sy = (y as i32 + ky - 1).clamp(0, h as i32 - 1) as u32;
                        let p = img.get_pixel(sx, sy).0;
                        let kv = k[(ky * 3 + kx) as usize];
                        if gray {
                            acc[0] += lum(p) * kv;
                        } else {
                            for c in 0..3 {
                                acc[c] += p[c] as f32 * kv;
                            }
                        }
                    }
                }
                for c in 0..3 {
                    row[x * 4 + c] = clamp8(if gray { acc[0] } else { acc[c] } + bias);
                }
            }
        });
    out
}

fn sobel(img: &RgbaImage) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let (w, h) = img.dimensions();
    let gray: Vec<f32> = img.pixels().map(|p| lum(p.0)).collect();
    let g = |x: i32, y: i32| {
        gray[(y.clamp(0, h as i32 - 1) * w as i32 + x.clamp(0, w as i32 - 1)) as usize]
    };
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            let y = y as i32;
            for x in 0..w as i32 {
                let gx = -g(x - 1, y - 1) - 2.0 * g(x - 1, y) - g(x - 1, y + 1)
                    + g(x + 1, y - 1)
                    + 2.0 * g(x + 1, y)
                    + g(x + 1, y + 1);
                let gy = -g(x - 1, y - 1) - 2.0 * g(x, y - 1) - g(x + 1, y - 1)
                    + g(x - 1, y + 1)
                    + 2.0 * g(x, y + 1)
                    + g(x + 1, y + 1);
                let v = clamp8((gx * gx + gy * gy).sqrt());
                let i = x as usize * 4;
                row[i..i + 3].copy_from_slice(&[v, v, v]);
            }
        });
    out
}

// ---------------------------------------------------------------- effets

fn pixelate(img: &RgbaImage, size: u32) -> RgbaImage {
    let size = size.max(1);
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    for by in (0..h).step_by(size as usize) {
        for bx in (0..w).step_by(size as usize) {
            let (bw, bh) = (size.min(w - bx), size.min(h - by));
            let mut acc = [0u64; 4];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let p = img.get_pixel(x, y);
                    for c in 0..4 {
                        acc[c] += p[c] as u64;
                    }
                }
            }
            let n = (bw * bh) as u64;
            let px = Rgba([
                (acc[0] / n) as u8,
                (acc[1] / n) as u8,
                (acc[2] / n) as u8,
                (acc[3] / n) as u8,
            ]);
            for y in by..by + bh {
                for x in bx..bx + bw {
                    out.put_pixel(x, y, px);
                }
            }
        }
    }
    out
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn add_noise(img: &RgbaImage, amount: f32, mono: bool, seed: u32) -> RgbaImage {
    let mut out = img.clone();
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        let base = hash(i as u32 ^ seed.wrapping_mul(0x9e37_79b9));
        for c in 0..3 {
            let r = if mono {
                base
            } else {
                hash(base.wrapping_add(c as u32 + 1))
            };
            let n = (r as f32 / u32::MAX as f32 - 0.5) * 2.0 * amount * 2.55;
            px[c] = clamp8(px[c] as f32 + n);
        }
    });
    out
}

fn median(img: &RgbaImage, radius: u32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let r = radius.clamp(1, 5) as i32;
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            let mut win: [Vec<u8>; 3] = [vec![], vec![], vec![]];
            for x in 0..w as i32 {
                for v in win.iter_mut() {
                    v.clear();
                }
                for dy in -r..=r {
                    for dx in -r..=r {
                        let sx = (x + dx).clamp(0, w as i32 - 1) as u32;
                        let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        let p = img.get_pixel(sx, sy);
                        for c in 0..3 {
                            win[c].push(p[c]);
                        }
                    }
                }
                for c in 0..3 {
                    let mid = win[c].len() / 2;
                    let (_, m, _) = win[c].select_nth_unstable(mid);
                    row[x as usize * 4 + c] = *m;
                }
            }
        });
    out
}

fn vignette(img: &RgbaImage, strength: f32, radius: f32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let (w, h) = img.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let maxd = (cx * cx + cy * cy).sqrt();
    let start = radius.clamp(0.0, 0.99);
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            for x in 0..w as usize {
                let d =
                    (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / maxd).min(1.0);
                let t = ((d - start) / (1.0 - start)).clamp(0.0, 1.0);
                let f = 1.0 - strength * t * t * (3.0 - 2.0 * t);
                for c in 0..3 {
                    row[x * 4 + c] = clamp8(row[x * 4 + c] as f32 * f);
                }
            }
        });
    out
}

fn oil_paint(img: &RgbaImage, radius: u32, levels: u32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let r = radius.clamp(1, 8) as i32;
    let levels = levels.clamp(4, 64) as usize;
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            let mut count = vec![0u32; levels];
            let mut sum = vec![[0u32; 3]; levels];
            for x in 0..w as i32 {
                count.iter_mut().for_each(|c| *c = 0);
                sum.iter_mut().for_each(|s| *s = [0; 3]);
                for dy in -r..=r {
                    for dx in -r..=r {
                        let sx = (x + dx).clamp(0, w as i32 - 1) as u32;
                        let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        let p = img.get_pixel(sx, sy).0;
                        let bin = ((lum(p) / 256.0) * levels as f32) as usize;
                        count[bin] += 1;
                        for c in 0..3 {
                            sum[bin][c] += p[c] as u32;
                        }
                    }
                }
                let (best, n) = count
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, c)| **c)
                    .map(|(i, c)| (i, *c))
                    .unwrap();
                for c in 0..3 {
                    row[x as usize * 4 + c] = (sum[best][c] / n.max(1)) as u8;
                }
            }
        });
    out
}

fn crystallize(img: &RgbaImage, cell: u32) -> RgbaImage {
    let cancel_task=crate::preview_jobs::current();
    let cell = cell.max(2) as i32;
    let (w, h) = img.dimensions();
    let seed = |cx: i32, cy: i32| -> (i32, i32) {
        let hsh = hash((cx as u32).wrapping_mul(73_856_093) ^ (cy as u32).wrapping_mul(19_349_663));
        (
            cx * cell + (hsh % cell as u32) as i32,
            cy * cell + ((hsh >> 16) % cell as u32) as i32,
        )
    };
    let mut out = img.clone();
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel_task.cancelled(){return;}
            let y = y as i32;
            for x in 0..w as i32 {
                let (gx, gy) = (x / cell, y / cell);
                let mut best = (i64::MAX, 0, 0);
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        let (sx, sy) = seed(gx + ox, gy + oy);
                        let d = ((sx - x) as i64).pow(2) + ((sy - y) as i64).pow(2);
                        if d < best.0 {
                            best = (d, sx, sy);
                        }
                    }
                }
                let p = img.get_pixel(
                    best.1.clamp(0, w as i32 - 1) as u32,
                    best.2.clamp(0, h as i32 - 1) as u32,
                );
                let i = x as usize * 4;
                row[i..i + 4].copy_from_slice(&p.0);
            }
        });
    out
}

fn auto_levels(img: &RgbaImage) -> RgbaImage {
    let mut hist = [[0u64; 256]; 3];
    let mut total = 0u64;
    for p in img.pixels().filter(|p| p[3] > 0) {
        for c in 0..3 {
            hist[c][p[c] as usize] += 1;
        }
        total += 1;
    }
    if total == 0 {
        return img.clone();
    }
    let clip = total / 200;
    let mut lut = [[0u8; 256]; 3];
    for c in 0..3 {
        let (mut lo, mut hi, mut acc) = (0usize, 255usize, 0u64);
        for (i, n) in hist[c].iter().enumerate() {
            acc += n;
            if acc > clip {
                lo = i;
                break;
            }
        }
        acc = 0;
        for i in (0..256).rev() {
            acc += hist[c][i];
            if acc > clip {
                hi = i;
                break;
            }
        }
        let span = (hi.max(lo + 1) - lo) as f32;
        for i in 0..256 {
            lut[c][i] = clamp8((i as f32 - lo as f32) * 255.0 / span);
        }
    }
    map_pixels(img, |p| {
        [
            lut[0][p[0] as usize],
            lut[1][p[1] as usize],
            lut[2][p[2] as usize],
            p[3],
        ]
    })
}

pub fn filter(img: &RgbaImage, name: &str, p: &Value) -> Result<RgbaImage, String> {
    crate::preview_jobs::checkpoint()?;
    let result=match name {
        "invert" => map_pixels(img, |p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]]),
        "grayscale" => map_pixels(img, |p| {
            let v = clamp8(lum(p));
            [v, v, v, p[3]]
        }),
        "sepia" => map_pixels(img, |p| {
            let [r, g, b, a] = p;
            let (r, g, b) = (r as f32, g as f32, b as f32);
            [
                clamp8(r * 0.393 + g * 0.769 + b * 0.189),
                clamp8(r * 0.349 + g * 0.686 + b * 0.168),
                clamp8(r * 0.272 + g * 0.534 + b * 0.131),
                a,
            ]
        }),
        "brightness" => {
            let b = num(p, "brightness", 0.0).clamp(-100.0, 100.0) * 2.55;
            let c = num(p, "contrast", 0.0).clamp(-100.0, 100.0) * 2.55;
            let factor = 259.0 * (c + 255.0) / (255.0 * (259.0 - c));
            map_pixels(img, |mut p| {
                for v in &mut p[..3] {
                    *v = clamp8(factor * (*v as f32 - 128.0) + 128.0 + b);
                }
                p
            })
        }
        "hsl" => {
            let (dh, ds, dl) = (
                num(p, "hue", 0.0) / 360.0,
                num(p, "saturation", 0.0) / 100.0,
                num(p, "lightness", 0.0) / 100.0,
            );
            map_pixels(img, |p| {
                let (h, s, l) = rgb_to_hsl(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
                let (r, g, b) = hsl_to_rgb(
                    (h + dh).rem_euclid(1.0),
                    (s + ds).clamp(0.0, 1.0),
                    (l + dl).clamp(0.0, 1.0),
                );
                [
                    clamp8(r * 255.0),
                    clamp8(g * 255.0),
                    clamp8(b * 255.0),
                    p[3],
                ]
            })
        }
        "gamma" => {
            let g = num(p, "gamma", 1.0).clamp(0.1, 10.0);
            map_pixels(img, |mut p| {
                for v in &mut p[..3] {
                    *v = clamp8((*v as f32 / 255.0).powf(1.0 / g) * 255.0);
                }
                p
            })
        }
        "balance" => map_pixels(img, |mut px| {
            for (i, k) in ["red", "green", "blue"].iter().enumerate() {
                px[i] = clamp8(px[i] as f32 + num(p, k, 0.0).clamp(-255.0, 255.0));
            }
            px
        }),
        "threshold" => {
            let t = num(p, "threshold", 128.0);
            map_pixels(img, |p| {
                let v = if lum(p) >= t { 255 } else { 0 };
                [v, v, v, p[3]]
            })
        }
        "posterize" => {
            let n = num(p, "levels", 6.0).clamp(2.0, 256.0) - 1.0;
            map_pixels(img, |mut p| {
                for v in &mut p[..3] {
                    *v = clamp8((*v as f32 / 255.0 * n).round() / n * 255.0);
                }
                p
            })
        }
        "auto-levels" => auto_levels(img),
        "auto-contrast" => {
            let (mut lo, mut hi) = (255.0f32, 0.0f32);
            for p in img.pixels().filter(|p| p[3] > 0) {
                let v = lum(p.0);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            if hi <= lo {
                img.clone()
            } else {
                map_pixels(img, |p| {
                    [
                        clamp8((p[0] as f32 - lo) * 255.0 / (hi - lo)),
                        clamp8((p[1] as f32 - lo) * 255.0 / (hi - lo)),
                        clamp8((p[2] as f32 - lo) * 255.0 / (hi - lo)),
                        p[3],
                    ]
                })
            }
        }
        "old-photo" => {
            let sepia = filter(img, "sepia", &serde_json::json!({}))?;
            filter(&sepia, "vignette", &serde_json::json!({"strength":0.35}))?
        }
        "blur" => gaussian_blur(img, num(p, "radius", 3.0).clamp(0.0, 100.0)),
        "motion" => motion_blur(
            img,
            num(p, "angle", 0.0),
            num(p, "distance", 15.0).clamp(1.0, 500.0),
        ),
        "sharpen" => convolve3(
            img,
            [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
            0.0,
            false,
        ),
        "unsharp" => unsharp(
            img,
            num(p, "radius", 2.0).clamp(0.0, 100.0),
            num(p, "amount", 1.5).clamp(0.0, 10.0),
            num(p, "threshold", 3.0),
        ),
        "emboss" => convolve3(
            img,
            [-2.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 2.0],
            128.0,
            true,
        ),
        "edges" => sobel(img),
        "pixelate" => pixelate(img, num(p, "size", 10.0).clamp(1.0, 1000.0) as u32),
        "noise" => add_noise(
            img,
            num(p, "amount", 15.0).clamp(0.0, 100.0),
            flag(p, "mono"),
            num(p, "seed", 42.0) as u32,
        ),
        "median" => median(img, num(p, "radius", 2.0) as u32),
        "vignette" => vignette(
            img,
            num(p, "strength", 0.7).clamp(0.0, 1.0),
            num(p, "radius", 0.3),
        ),
        "oil" => oil_paint(
            img,
            num(p, "radius", 3.0) as u32,
            num(p, "levels", 16.0) as u32,
        ),
        "crystallize" => crystallize(img, num(p, "size", 20.0).clamp(2.0, 1000.0) as u32),
        "curves" | "levels" | "replace-color" | "channel-mixer" | "selective-saturation" => {
            crate::adjustments::apply(img, name, p)?
        }
        _ => return Err(format!("Filtre inconnu : {name}")),
    };
    crate::preview_jobs::checkpoint()?;
    Ok(result)
}

/// Fusion source-over avec couleurs droites, y compris pour un fond transparent.
pub fn blend_pixel(dst: [u8; 4], src: [u8; 4], opacity: f32, mode: BlendMode) -> [u8; 4] {
    let sa = src[3] as f32 / 255.0 * opacity.clamp(0.0, 1.0);
    let da = dst[3] as f32 / 255.0;
    let a = sa + da * (1.0 - sa);
    if a <= 0.0 {
        return [0; 4];
    }
    let mut out = [0, 0, 0, clamp8(a * 255.0)];
    for c in 0..3 {
        let (s, d) = (src[c] as f32 / 255.0, dst[c] as f32 / 255.0);
        let b = match mode {
            BlendMode::Normal => s,
            BlendMode::Multiply => s * d,
            BlendMode::Screen => s + d - s * d,
            BlendMode::Overlay => {
                if d <= 0.5 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                }
            }
            BlendMode::Darken => s.min(d),
            BlendMode::Lighten => s.max(d),
            BlendMode::Difference => (s - d).abs(),
            BlendMode::Add => (s + d).min(1.0),
        };
        out[c] = clamp8(((1.0 - sa) * da * d + (1.0 - da) * sa * s + sa * da * b) / a * 255.0);
    }
    out
}

pub fn composite(
    dst: &mut RgbaImage,
    src: &RgbaImage,
    x: i32,
    y: i32,
    opacity: f32,
    mode: BlendMode,
) {
    let cancel_task=crate::preview_jobs::current();
    let (w, h) = dst.dimensions();
    dst.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(dy, row)| {
            if cancel_task.cancelled(){return;}
            let sy = dy as i64 - y as i64;
            if sy < 0 || sy >= src.height() as i64 {
                return;
            }
            for dx in 0..w {
                let sx = dx as i64 - x as i64;
                if sx < 0 || sx >= src.width() as i64 {
                    continue;
                }
                let i = dx as usize * 4;
                let p = blend_pixel(
                    row[i..i + 4].try_into().unwrap(),
                    src.get_pixel(sx as u32, sy as u32).0,
                    opacity,
                    mode,
                );
                row[i..i + 4].copy_from_slice(&p);
            }
        });
    let _ = h;
}

pub fn apply_mask(
    original: &RgbaImage,
    changed: &mut RgbaImage,
    mask: &[u8],
    doc_w: u32,
    doc_h: u32,
    x: i32,
    y: i32,
) {
    let cancel_task=crate::preview_jobs::current();
    let w = original.width();
    changed
        .par_chunks_mut(4)
        .zip(original.par_chunks(4))
        .enumerate()
        .for_each(|(i, (p, o))| {
            if cancel_task.cancelled(){return;}
            let (dx, dy) = (
                i as i64 % w as i64 + x as i64,
                i as i64 / w as i64 + y as i64,
            );
            let m = if dx >= 0 && dy >= 0 && dx < doc_w as i64 && dy < doc_h as i64 {
                mask[(dy * doc_w as i64 + dx) as usize] as f32 / 255.0
            } else {
                0.0
            };
            // Interpoler en alpha prémultiplié, pour que la gomme et les filtres respectent le masque.
            let a = o[3] as f32 * (1.0 - m) + p[3] as f32 * m;
            for c in 0..3 {
                p[c] = if a > 0.0 {
                    clamp8(
                        (o[c] as f32 * o[3] as f32 * (1.0 - m) + p[c] as f32 * p[3] as f32 * m) / a,
                    )
                } else {
                    0
                };
            }
            p[3] = clamp8(a);
        });
}

pub fn stroke(
    img: &mut RgbaImage,
    points: &[[f32; 2]],
    radius: f32,
    color: [u8; 4],
    opacity: f32,
    erase: bool,
) {
    if points.is_empty() {
        return;
    }
    let radius = radius.clamp(0.5, 1000.0);
    // Une couverture maximale par pixel : l'opacité ne dépend pas du nombre d'événements souris.
    let mut coverage = vec![0u8; img.width() as usize * img.height() as usize];
    let mut stamp = |x: f32, y: f32| {
        let x0 = (x - radius - 1.0).floor().max(0.0) as u32;
        let y0 = (y - radius - 1.0).floor().max(0.0) as u32;
        let x1 = (x + radius + 1.0).ceil().clamp(0.0, img.width() as f32) as u32;
        let y1 = (y + radius + 1.0).ceil().clamp(0.0, img.height() as f32) as u32;
        for py in y0..y1 {
            for px in x0..x1 {
                let d = ((px as f32 + 0.5 - x).powi(2) + (py as f32 + 0.5 - y).powi(2)).sqrt();
                let a = clamp8((radius + 0.5 - d).clamp(0.0, 1.0) * 255.0);
                let i = (py * img.width() + px) as usize;
                coverage[i] = coverage[i].max(a);
            }
        }
    };
    stamp(points[0][0], points[0][1]);
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = ((b[0] - a[0]).hypot(b[1] - a[1]) / (radius * 0.3).max(0.5))
            .ceil()
            .max(1.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            stamp(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
        }
    }
    img.par_chunks_mut(4)
        .zip(coverage.par_iter())
        .for_each(|(p, c)| {
            let amount = *c as f32 / 255.0 * opacity.clamp(0.0, 1.0);
            if erase {
                p[3] = clamp8(p[3] as f32 * (1.0 - amount));
            } else {
                let out = blend_pixel(p.try_into().unwrap(), color, amount, BlendMode::Normal);
                p.copy_from_slice(&out);
            }
        });
}

pub fn flood_fill(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    color: [u8; 4],
    tolerance: u8,
    allowed: Option<&[u8]>,
) {
    let (w, h) = img.dimensions();
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let target = img.get_pixel(x as u32, y as u32).0;
    let mut seen = vec![false; (w * h) as usize];
    let mut stack = vec![(x as u32, y as u32)];
    while let Some((x, y)) = stack.pop() {
        let i = (y * w + x) as usize;
        if seen[i] {
            continue;
        }
        seen[i] = true;
        if allowed.is_some_and(|m| m[i] == 0) {
            continue;
        }
        if img
            .get_pixel(x, y)
            .0
            .iter()
            .zip(target)
            .any(|(a, b)| a.abs_diff(b) > tolerance)
        {
            continue;
        }
        img.put_pixel(x, y, Rgba(color));
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
}

pub fn gradient(img: &mut RgbaImage, start: [f32; 2], end: [f32; 2], a: [u8; 4], b: [u8; 4]) {
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    let len = (dx * dx + dy * dy).max(0.001);
    let w = img.width();
    img.par_chunks_mut(4).enumerate().for_each(|(i, p)| {
        let t = (((i as u32 % w) as f32 + 0.5 - start[0]) * dx
            + ((i as u32 / w) as f32 + 0.5 - start[1]) * dy)
            / len;
        let t = t.clamp(0.0, 1.0);
        let mut color = [0; 4];
        for c in 0..4 {
            color[c] = clamp8(a[c] as f32 * (1.0 - t) + b[c] as f32 * t);
        }
        let out = blend_pixel(p.try_into().unwrap(), color, 1.0, BlendMode::Normal);
        p.copy_from_slice(&out);
    });
}

/// Rotation libre, échantillonnage bilinéaire en alpha prémultiplié.
pub fn rotate(img: &RgbaImage, degrees: f32) -> Result<RgbaImage, String> {
    let angle = degrees.to_radians();
    let (sn, cs) = angle.sin_cos();
    let (w, h) = (
        (img.width() as f32 * cs.abs() + img.height() as f32 * sn.abs()).ceil() as u32,
        (img.width() as f32 * sn.abs() + img.height() as f32 * cs.abs()).ceil() as u32,
    );
    crate::io::dimensions(w, h)?;
    let mut out = RgbaImage::new(w, h);
    out.par_chunks_mut(w as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let (dx, dy) = (
                    x as f32 + 0.5 - w as f32 / 2.0,
                    y as f32 + 0.5 - h as f32 / 2.0,
                );
                let (sx, sy) = (
                    cs * dx + sn * dy + img.width() as f32 / 2.0 - 0.5,
                    -sn * dx + cs * dy + img.height() as f32 / 2.0 - 0.5,
                );
                let (ix, iy) = (sx.floor() as i32, sy.floor() as i32);
                let (tx, ty) = (sx - sx.floor(), sy - sy.floor());
                let mut sum = [0.0f32; 4];
                for (ox, oy, weight) in [
                    (0, 0, (1.0 - tx) * (1.0 - ty)),
                    (1, 0, tx * (1.0 - ty)),
                    (0, 1, (1.0 - tx) * ty),
                    (1, 1, tx * ty),
                ] {
                    let (px, py) = (ix + ox, iy + oy);
                    if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                        continue;
                    }
                    let p = img.get_pixel(px as u32, py as u32);
                    let a = p[3] as f32 / 255.0;
                    for c in 0..3 {
                        sum[c] += p[c] as f32 * a * weight;
                    }
                    sum[3] += a * weight;
                }
                let i = x as usize * 4;
                if sum[3] > 0.0 {
                    for c in 0..3 {
                        row[i + c] = clamp8(sum[c] / sum[3]);
                    }
                    row[i + 3] = clamp8(sum[3] * 255.0);
                }
            }
        });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blending_on_transparency_preserves_source() {
        for mode in [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
        ] {
            assert_eq!(
                blend_pixel([0; 4], [220, 80, 30, 128], 1.0, mode),
                [220, 80, 30, 128]
            );
        }
        assert_eq!(
            blend_pixel([0, 0, 255, 255], [255, 0, 0, 128], 1.0, BlendMode::Normal),
            [128, 0, 127, 255]
        );
    }
    #[test]
    fn blur_has_no_black_halo() {
        let mut img = RgbaImage::new(9, 1);
        img.put_pixel(4, 0, Rgba([255, 0, 0, 255]));
        for p in gaussian_blur(&img, 2.0).pixels().filter(|p| p[3] > 0) {
            assert_eq!(p[0], 255);
            assert_eq!(p[1], 0);
        }
    }
    #[test]
    fn fill_stops_at_selection_and_edges() {
        let mut img = RgbaImage::from_pixel(3, 1, Rgba([255; 4]));
        flood_fill(&mut img, 0, 0, [0, 0, 0, 255], 0, Some(&[255, 0, 255]));
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(img.get_pixel(2, 0).0, [255; 4]);
    }
    #[test]
    fn stroke_opacity_does_not_accumulate_at_samples() {
        let mut a = RgbaImage::new(10, 10);
        let mut b = a.clone();
        stroke(&mut a, &[[5.0, 5.0]], 2.0, [255; 4], 0.5, false);
        stroke(
            &mut b,
            &[[5.0, 5.0], [5.0, 5.0], [5.0, 5.0]],
            2.0,
            [255; 4],
            0.5,
            false,
        );
        assert_eq!(a, b);
    }
}
