//! Réglages non linéaires en RGBA 8 bits. Les LUT ne changent jamais l'alpha.
use image::RgbaImage;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
fn number(p: &Value, key: &str, default: f32) -> f32 {
    p.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .map_or(default, |v| v as f32)
}
fn flag(p: &Value, key: &str) -> bool {
    p.get(key).and_then(Value::as_bool).unwrap_or(false)
}
fn byte(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
fn map<F: Fn([u8; 4]) -> [u8; 4] + Sync>(img: &RgbaImage, f: F) -> RgbaImage {
    let mut out = img.clone();
    let token=crate::preview_jobs::current();
    out.par_chunks_mut(4096).for_each(|chunk| {
      if token.cancelled(){return;}
      for p in chunk.chunks_exact_mut(4){
        let input = [p[0], p[1], p[2], p[3]];
        let output = f(input);
        p.copy_from_slice(&output);
      }
    });
    out
}

/// Interpolation Hermite monotone : aucune oscillation ajoutée entre les points.
pub fn curve(points: &[[f32; 2]]) -> Result<[u8; 256], String> {
    if !(2..=32).contains(&points.len())
        || points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || !(0.0..=255.0).contains(v))
    {
        return Err("Une courbe comporte 2 à 32 points entre 0 et 255".into());
    }
    if points.windows(2).any(|p| p[1][0] <= p[0][0]) {
        return Err("Les entrées de la courbe doivent être strictement croissantes".into());
    }
    let n = points.len();
    let dx = points
        .windows(2)
        .map(|p| p[1][0] - p[0][0])
        .collect::<Vec<_>>();
    let d = points
        .windows(2)
        .map(|p| (p[1][1] - p[0][1]) / (p[1][0] - p[0][0]))
        .collect::<Vec<_>>();
    let mut slope = vec![0f32; n];
    slope[0] = d[0];
    slope[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        if d[i - 1] * d[i] > 0.0 {
            let (a, b) = (2.0 * dx[i] + dx[i - 1], dx[i] + 2.0 * dx[i - 1]);
            slope[i] = (a + b) / (a / d[i - 1] + b / d[i]);
        }
    }
    let mut lut = [0; 256];
    let mut index = 0;
    for (x, out) in lut.iter_mut().enumerate() {
        let x = x as f32;
        if x <= points[0][0] {
            *out = byte(points[0][1]);
            continue;
        }
        if x >= points[n - 1][0] {
            *out = byte(points[n - 1][1]);
            continue;
        }
        while index + 1 < n - 1 && x > points[index + 1][0] {
            index += 1;
        }
        let t = (x - points[index][0]) / dx[index];
        let t2 = t * t;
        let t3 = t2 * t;
        *out = byte(
            (2.0 * t3 - 3.0 * t2 + 1.0) * points[index][1]
                + (t3 - 2.0 * t2 + t) * dx[index] * slope[index]
                + (-2.0 * t3 + 3.0 * t2) * points[index + 1][1]
                + (t3 - t2) * dx[index] * slope[index + 1],
        );
    }
    Ok(lut)
}

pub fn apply(img: &RgbaImage, name: &str, p: &Value) -> Result<RgbaImage, String> {
    Ok(match name {
        "curves" => {
            let mut luts = Vec::new();
            for key in ["rgb", "red", "green", "blue"] {
                let points: Vec<[f32; 2]> = match p.get(key) {
                    Some(value) => serde_json::from_value(value.clone())
                        .map_err(|_| "Points de courbe invalides")?,
                    None => vec![[0.0, 0.0], [255.0, 255.0]],
                };
                luts.push(curve(&points)?);
            }
            map(img, |mut pixel| {
                for c in 0..3 {
                    pixel[c] = luts[c + 1][luts[0][pixel[c] as usize] as usize];
                }
                pixel
            })
        }
        "levels" => {
            let black = number(p, "inputBlack", 0.0).clamp(0.0, 254.0);
            let white = number(p, "inputWhite", 255.0).clamp(1.0, 255.0);
            if white <= black {
                return Err("Le blanc d'entrée doit être supérieur au noir d'entrée".into());
            }
            let gamma = number(p, "gamma", 1.0).clamp(0.1, 10.0);
            let out_black = number(p, "outputBlack", 0.0).clamp(0.0, 255.0);
            let out_white = number(p, "outputWhite", 255.0).clamp(0.0, 255.0);
            let mut lut = [0u8; 256];
            for (i, v) in lut.iter_mut().enumerate() {
                *v = byte(
                    out_black
                        + ((i as f32 - black) / (white - black))
                            .clamp(0.0, 1.0)
                            .powf(1.0 / gamma)
                            * (out_white - out_black),
                );
            }
            let channel = number(p, "channel", 0.0).round().clamp(0.0, 3.0) as usize;
            map(img, |mut pixel| {
                for (c, value) in pixel[..3].iter_mut().enumerate() {
                    if channel == 0 || channel == c + 1 {
                        *value = lut[*value as usize];
                    }
                }
                pixel
            })
        }
        "replace-color" => {
            let parse = |key: &str, fallback: &str| {
                crate::raster::parse_color(p.get(key).and_then(Value::as_str).unwrap_or(fallback))
                    .ok_or_else(|| "Couleur de remplacement invalide".to_string())
            };
            let source = parse("sourceColor", "#ff0000")?;
            let target = parse("targetColor", "#0000ff")?;
            let tolerance = number(p, "tolerance", 30.0).clamp(0.0, 255.0);
            let softness = number(p, "softness", 20.0).clamp(0.0, 255.0);
            let amount = number(p, "amount", 100.0).clamp(0.0, 100.0) / 100.0;
            map(img, |mut pixel| {
                let distance = (0..3).map(|c| pixel[c].abs_diff(source[c])).max().unwrap() as f32;
                let weight = if distance <= tolerance {
                    1.0
                } else if softness > 0.0 {
                    (1.0 - (distance - tolerance) / softness).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                for c in 0..3 {
                    let replacement = if flag(p, "preserveShading") {
                        pixel[c] as f32 + target[c] as f32 - source[c] as f32
                    } else {
                        target[c] as f32
                    };
                    pixel[c] = byte(
                        pixel[c] as f32 * (1.0 - weight * amount)
                            + replacement.clamp(0.0, 255.0) * weight * amount,
                    );
                }
                pixel
            })
        }
        "channel-mixer" => {
            let mut matrix = [[0f32; 4]; 3];
            for (row, label) in ["red", "green", "blue"].iter().enumerate() {
                for (column, input) in ["Red", "Green", "Blue", "Constant"].iter().enumerate() {
                    matrix[row][column] = number(
                        p,
                        &format!("{label}{input}"),
                        if row == column { 100.0 } else { 0.0 },
                    )
                    .clamp(-200.0, 200.0)
                        / 100.0;
                }
            }
            map(img, |pixel| {
                let mut out = pixel;
                for row in 0..3 {
                    out[row] = byte(
                        (0..3)
                            .map(|c| matrix[row][c] * pixel[c] as f32)
                            .sum::<f32>()
                            + matrix[row][3] * 255.0,
                    );
                }
                if flag(p, "monochrome") {
                    out[1] = out[0];
                    out[2] = out[0];
                }
                if flag(p, "preserveLuminosity") {
                    let luminance = |p: [u8; 4]| {
                        0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
                    };
                    let before = luminance(pixel);
                    let after = luminance(out);
                    if after > 0.0 {
                        for v in &mut out[..3] {
                            *v = byte(*v as f32 * before / after);
                        }
                    }
                }
                out
            })
        }
        "selective-saturation" => {
            let center = number(p, "hue", 0.0).rem_euclid(360.0);
            let width = number(p, "range", 60.0).clamp(0.0, 360.0) / 2.0;
            let softness = number(p, "softness", 30.0).clamp(0.0, 180.0);
            let amount = number(p, "saturation", -100.0).clamp(-100.0, 100.0) / 100.0;
            map(img, |pixel| {
                let (h, s, l) = crate::raster::rgb_to_hsl(
                    pixel[0] as f32 / 255.0,
                    pixel[1] as f32 / 255.0,
                    pixel[2] as f32 / 255.0,
                );
                let d = (h * 360.0 - center).abs();
                let d = d.min(360.0 - d);
                let weight = if d <= width {
                    1.0
                } else if softness > 0.0 {
                    (1.0 - (d - width) / softness).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let weight = if flag(p, "invert") {
                    1.0 - weight
                } else {
                    weight
                };
                let saturation = if amount <= 0.0 {
                    s * (1.0 + amount * weight)
                } else {
                    s + (1.0 - s) * amount * weight
                };
                let (r, g, b) = crate::raster::hsl_to_rgb(h, saturation, l);
                [byte(r * 255.0), byte(g * 255.0), byte(b * 255.0), pixel[3]]
            })
        }
        _ => return Err("Réglage avancé inconnu".into()),
    })
}

#[derive(Serialize)]
pub struct Histogram {
    pub red: Vec<u64>,
    pub green: Vec<u64>,
    pub blue: Vec<u64>,
    pub luma: Vec<u64>,
}
pub fn histogram(img: &RgbaImage) -> Histogram {
    let mut out = Histogram {
        red: vec![0; 256],
        green: vec![0; 256],
        blue: vec![0; 256],
        luma: vec![0; 256],
    };
    for p in img.pixels().filter(|p| p[3] > 0) {
        out.red[p[0] as usize] += 1;
        out.green[p[1] as usize] += 1;
        out.blue[p[2] as usize] += 1;
        let l = byte(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
        out.luma[l as usize] += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use serde_json::json;
    #[test]
    fn identity_adjustments_preserve_all_channels_exactly() {
        let img = RgbaImage::from_fn(256, 1, |x, _| {
            Rgba([x as u8, 255 - x as u8, (x / 2) as u8, 127])
        });
        for name in ["curves", "levels", "channel-mixer"] {
            assert_eq!(apply(&img, name, &json!({})).unwrap(), img);
        }
    }
    #[test]
    fn curves_support_channels_and_remain_monotone_between_control_points() {
        let points = [[0.0, 0.0], [64.0, 30.0], [160.0, 230.0], [255.0, 255.0]];
        let lut = curve(&points).unwrap();
        assert_eq!(lut[64], 30);
        assert_eq!(lut[160], 230);
        assert!(lut.windows(2).all(|v| v[0] <= v[1]));
        let img = RgbaImage::from_pixel(1, 1, Rgba([20, 60, 140, 88]));
        assert_eq!(
            apply(&img, "curves", &json!({"red":[[0,255],[255,0]]}))
                .unwrap()
                .get_pixel(0, 0)
                .0,
            [235, 60, 140, 88]
        );
        assert!(curve(&[[0.0, 0.0], [0.0, 255.0]]).is_err());
    }
    #[test]
    fn levels_clip_endpoints_and_reject_inverted_input_range() {
        let img = RgbaImage::from_fn(3, 1, |x, _| {
            Rgba([[40, 40, 40, 90], [120, 120, 120, 90], [200, 200, 200, 90]][x as usize])
        });
        let out = apply(
            &img,
            "levels",
            &json!({"inputBlack":40,"inputWhite":200,"outputBlack":10,"outputWhite":240}),
        )
        .unwrap();
        assert_eq!(out.get_pixel(0, 0).0, [10, 10, 10, 90]);
        assert_eq!(out.get_pixel(1, 0).0, [125, 125, 125, 90]);
        assert_eq!(out.get_pixel(2, 0).0, [240, 240, 240, 90]);
        assert!(apply(&img, "levels", &json!({"inputBlack":200,"inputWhite":100})).is_err());
    }
    #[test]
    fn replacement_and_mixer_respect_alpha() {
        let img = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 75]));
        assert_eq!(
            apply(
                &img,
                "replace-color",
                &json!({"sourceColor":"#ff0000","targetColor":"#00ff00","tolerance":0})
            )
            .unwrap()
            .get_pixel(0, 0)
            .0,
            [0, 255, 0, 75]
        );
        assert_eq!(
            apply(
                &img,
                "channel-mixer",
                &json!({"redRed":0,"blueRed":100,"blueBlue":0})
            )
            .unwrap()
            .get_pixel(0, 0)
            .0,
            [0, 0, 255, 75]
        );
    }
    #[test]
    fn selective_desaturation_can_preserve_selected_hues() {
        let img = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 120])
            }
        });
        let out = apply(
            &img,
            "selective-saturation",
            &json!({"hue":0,"range":20,"softness":0,"saturation":-100,"invert":true}),
        )
        .unwrap();
        assert_eq!(out.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(out.get_pixel(1, 0).0, [128, 128, 128, 120]);
    }
}
