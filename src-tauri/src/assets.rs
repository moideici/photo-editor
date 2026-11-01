//! Bibliothèque procédurale originale : aucune ressource PhotoFiltre reprise.
use crate::{document::BlendMode, raster};
use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Mask,
    Frame,
    Pattern,
    Texture,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub kind: Kind,
    pub name: String,
    pub scale: f32,
    pub strength: f32,
    pub color: String,
    pub secondary: String,
    pub inverted: bool,
    pub path: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            kind: Kind::Pattern,
            name: "checker".into(),
            scale: 16.0,
            strength: 1.0,
            color: "#000000".into(),
            secondary: "#ffffff".into(),
            inverted: false,
            path: None,
        }
    }
}
fn byte(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
fn noise(x: u32, y: u32) -> f32 {
    let mut n = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ 42;
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846ca68b);
    (n ^ (n >> 16)) as f32 / u32::MAX as f32
}
fn lerp(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut out = [0; 4];
    for c in 0..4 {
        out[c] = byte(a[c] as f32 * (1.0 - t) + b[c] as f32 * t);
    }
    out
}

pub fn apply_file(img: &RgbaImage, p: &Settings) -> Result<RgbaImage, String> {
    let custom = p
        .path
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|path| crate::io::import(path).and_then(|doc| crate::vector::flatten(&doc)))
        .transpose()?;
    apply(img, p, custom.as_ref())
}
pub fn apply(
    img: &RgbaImage,
    p: &Settings,
    custom: Option<&RgbaImage>,
) -> Result<RgbaImage, String> {
    if !p.scale.is_finite()
        || !(1.0..=512.0).contains(&p.scale)
        || !p.strength.is_finite()
        || !(0.0..=1.0).contains(&p.strength)
    {
        return Err("Paramètres de ressource invalides".into());
    }
    let names: &[&str] = match p.kind {
        Kind::Mask => &[
            "ellipse",
            "rounded",
            "radial",
            "horizontal",
            "vertical",
            "diagonal",
        ],
        Kind::Frame => &["plain", "double", "film", "bevel"],
        Kind::Pattern => &["checker", "stripes", "dots", "bricks"],
        Kind::Texture => &["paper", "canvas", "grain", "wood"],
    };
    if custom.is_none() && !names.contains(&p.name.as_str()) {
        return Err("Ressource inconnue".into());
    }
    let color = raster::parse_color(&p.color).ok_or("Couleur de ressource invalide")?;
    let secondary = raster::parse_color(&p.secondary).ok_or("Couleur secondaire invalide")?;
    let (w, h) = img.dimensions();
    let size = p.scale;
    let custom = custom.map(|source| {
        let (cw, ch) = if matches!(p.kind, Kind::Mask | Kind::Frame) {
            (w, h)
        } else {
            let cw = size.round() as u32;
            let ch = (cw as f32 * source.height() as f32 / source.width() as f32)
                .round()
                .clamp(1.0, 1024.0) as u32;
            (cw, ch)
        };
        image::imageops::resize(source, cw, ch, image::imageops::FilterType::Lanczos3)
    });
    let mut out = img.clone();
    out.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
        let (x, y) = (i as u32 % w, i as u32 / w);
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        let imported = custom
            .as_ref()
            .map(|source| source.get_pixel(x % source.width(), y % source.height()).0);
        match p.kind {
            Kind::Mask => {
                let mut mask = if let Some(source) = imported {
                    (0.299 * source[0] as f32 + 0.587 * source[1] as f32 + 0.114 * source[2] as f32)
                        / 255.0
                        * source[3] as f32
                        / 255.0
                } else {
                    match p.name.as_str() {
                        "ellipse" => {
                            let d = ((px - w as f32 / 2.0) / (w as f32 / 2.0))
                                .hypot((py - h as f32 / 2.0) / (h as f32 / 2.0));
                            ((1.0 - d) * w.min(h) as f32 / 2.0 + 0.5).clamp(0.0, 1.0)
                        }
                        "rounded" => {
                            let radius = size.min(w.min(h) as f32 / 2.0);
                            let qx = (px - w as f32 / 2.0).abs() - (w as f32 / 2.0 - radius);
                            let qy = (py - h as f32 / 2.0).abs() - (h as f32 / 2.0 - radius);
                            let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
                            (0.5 - d).clamp(0.0, 1.0)
                        }
                        "horizontal" => x as f32 / (w - 1).max(1) as f32,
                        "vertical" => y as f32 / (h - 1).max(1) as f32,
                        "diagonal" => {
                            0.5 * (x as f32 / (w - 1).max(1) as f32
                                + y as f32 / (h - 1).max(1) as f32)
                        }
                        _ => {
                            let r = ((px - w as f32 / 2.0) / (w as f32 / 2.0))
                                .hypot((py - h as f32 / 2.0) / (h as f32 / 2.0));
                            let t = (1.0 - r).clamp(0.0, 1.0);
                            t * t * (3.0 - 2.0 * t)
                        }
                    }
                };
                if p.inverted {
                    mask = 1.0 - mask;
                }
                pixel[3] = byte(pixel[3] as f32 * (1.0 - p.strength + p.strength * mask));
            }
            Kind::Pattern => {
                let source = imported.unwrap_or_else(|| {
                    let (tx, ty) = ((px / size).fract(), (py / size).fract());
                    let mix = match p.name.as_str() {
                        "stripes" => {
                            if ((px + py) / size).floor() as i32 % 2 == 0 {
                                0.0
                            } else {
                                1.0
                            }
                        }
                        "dots" => ((tx - 0.5).hypot(ty - 0.5) - 0.25) * size + 0.5,
                        "bricks" => {
                            let offset = if (py / size).floor() as i32 % 2 == 0 {
                                0.0
                            } else {
                                0.5
                            };
                            if ty < 0.12 || ((px / size + offset).fract()) < 0.06 {
                                1.0
                            } else {
                                0.0
                            }
                        }
                        _ => {
                            if ((px / size).floor() as i32 + (py / size).floor() as i32) % 2 == 0 {
                                0.0
                            } else {
                                1.0
                            }
                        }
                    };
                    lerp(color, secondary, mix.clamp(0.0, 1.0))
                });
                let changed = raster::blend_pixel(
                    pixel.try_into().unwrap(),
                    source,
                    p.strength,
                    BlendMode::Normal,
                );
                pixel.copy_from_slice(&changed);
            }
            Kind::Texture => {
                let n = noise(x, y);
                let value = match p.name.as_str() {
                    "paper" => 0.7 + 0.25 * n + 0.05 * (px * 0.7).sin(),
                    "canvas" => {
                        0.5 + 0.18 * (px / size * std::f32::consts::TAU).sin()
                            + 0.18 * (py / size * std::f32::consts::TAU).sin()
                            + 0.1 * (n - 0.5)
                    }
                    "wood" => {
                        0.5 + 0.35
                            * (py / size * std::f32::consts::TAU
                                + 0.5 * (px / size * 0.4).sin()
                                + n * 0.3)
                                .sin()
                    }
                    _ => n,
                };
                let texture = imported.unwrap_or([byte(value * 255.0); 4]);
                let amount = p.strength * imported.map_or(1.0, |p| p[3] as f32 / 255.0);
                for c in 0..3 {
                    let tone = if imported.is_some() {
                        texture[c] as f32 / 255.0
                    } else {
                        value
                    };
                    pixel[c] = byte(pixel[c] as f32 * (1.0 - amount + amount * (0.5 + tone)));
                }
            }
            Kind::Frame => {
                if let Some(source) = imported {
                    let changed = raster::blend_pixel(
                        pixel.try_into().unwrap(),
                        source,
                        p.strength,
                        BlendMode::Normal,
                    );
                    pixel.copy_from_slice(&changed);
                    return;
                }
                let border = size.min(w.min(h) as f32 / 2.0);
                let distance = px.min(py).min(w as f32 - px).min(h as f32 - py);
                if distance >= border {
                    return;
                }
                let source = match p.name.as_str() {
                    "double" => {
                        if distance < border / 3.0 || distance > border * 2.0 / 3.0 {
                            color
                        } else {
                            return;
                        }
                    }
                    "film" => {
                        let in_top = py < border || py > h as f32 - border;
                        let hole = ((px / (border * 1.6)).fract() - 0.5).abs() < 0.23
                            && (py.min(h as f32 - py) - border * 0.5).abs() < border * 0.2;
                        if in_top && hole {
                            secondary
                        } else {
                            color
                        }
                    }
                    "bevel" => {
                        let lit = px.min(py) < (w as f32 - px).min(h as f32 - py);
                        let mut c = color;
                        for v in &mut c[..3] {
                            *v = if lit {
                                byte(*v as f32 * 0.5 + 127.5)
                            } else {
                                byte(*v as f32 * 0.5)
                            };
                        }
                        c
                    }
                    _ => color,
                };
                let changed = raster::blend_pixel(
                    pixel.try_into().unwrap(),
                    source,
                    p.strength,
                    BlendMode::Normal,
                );
                pixel.copy_from_slice(&changed);
            }
        }
    });
    Ok(out)
}

pub fn preview(p: &Settings) -> Result<RgbaImage, String> {
    let img = RgbaImage::from_fn(180, 120, |x, y| {
        Rgba([
            byte(80.0 + x as f32 * 0.8),
            byte(70.0 + y as f32 * 1.1),
            byte(180.0 - y as f32 * 0.6),
            255,
        ])
    });
    apply_file(&img, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_original_resource_has_a_visible_result_and_keeps_dimensions() {
        let img = RgbaImage::from_pixel(40, 30, Rgba([100, 120, 140, 255]));
        for (kind, names) in [
            (
                Kind::Mask,
                vec![
                    "ellipse",
                    "rounded",
                    "radial",
                    "horizontal",
                    "vertical",
                    "diagonal",
                ],
            ),
            (Kind::Frame, vec!["plain", "double", "film", "bevel"]),
            (Kind::Pattern, vec!["checker", "stripes", "dots", "bricks"]),
            (Kind::Texture, vec!["paper", "canvas", "grain", "wood"]),
        ] {
            for name in names {
                let p = Settings {
                    kind,
                    name: name.into(),
                    scale: 6.0,
                    ..Settings::default()
                };
                let result = apply(&img, &p, None).unwrap();
                assert_eq!(result.dimensions(), img.dimensions());
                assert_ne!(result, img, "{name}");
            }
        }
    }
    #[test]
    fn masks_keep_rgb_and_multiply_alpha_including_imported_alpha() {
        let img = RgbaImage::from_pixel(10, 8, Rgba([70, 80, 90, 200]));
        let p = Settings {
            kind: Kind::Mask,
            name: "ellipse".into(),
            ..Settings::default()
        };
        let result = apply(&img, &p, None).unwrap();
        assert_eq!(result.get_pixel(5, 4).0, [70, 80, 90, 200]);
        assert_eq!(result.get_pixel(0, 0)[3], 0);
        let custom = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 128]));
        assert_eq!(
            apply(&img, &p, Some(&custom)).unwrap().get_pixel(5, 4).0,
            [70, 80, 90, 100]
        );
        let p = Settings { strength: 0.0, ..p };
        assert_eq!(apply(&img, &p, None).unwrap(), img);
    }
    #[test]
    fn frame_does_not_overwrite_center_and_texture_preserves_alpha() {
        let img = RgbaImage::from_pixel(30, 20, Rgba([70, 80, 90, 120]));
        let p = Settings {
            kind: Kind::Frame,
            name: "double".into(),
            scale: 3.0,
            ..Settings::default()
        };
        let result = apply(&img, &p, None).unwrap();
        assert_eq!(result.get_pixel(15, 10).0, img.get_pixel(15, 10).0);
        let p = Settings {
            kind: Kind::Texture,
            name: "canvas".into(),
            ..Settings::default()
        };
        assert!(apply(&img, &p, None).unwrap().pixels().all(|p| p[3] == 120));
    }
    #[test]
    fn custom_patterns_tile_and_procedural_textures_are_repeatable() {
        let img = RgbaImage::new(8, 2);
        let tile = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let p = Settings {
            scale: 2.0,
            ..Settings::default()
        };
        let result = apply(&img, &p, Some(&tile)).unwrap();
        assert_eq!(result.get_pixel(0, 0), result.get_pixel(2, 0));
        assert_ne!(result.get_pixel(0, 0), result.get_pixel(1, 0));
        let p = Settings {
            kind: Kind::Texture,
            name: "grain".into(),
            ..Settings::default()
        };
        let img = RgbaImage::from_pixel(8, 2, Rgba([100; 4]));
        assert_eq!(
            apply(&img, &p, None).unwrap(),
            apply(&img, &p, None).unwrap()
        );
    }
}
