//! Image indexée : palette RGBA et indices conservés séparément, PNG PLTE/tRNS.
use image::{Rgba, RgbaImage};
use serde::Serialize;
use std::{collections::HashMap, io::Cursor, sync::Arc};
#[derive(Clone)]
pub struct IndexedImage {
    pub palette: Vec<[u8; 4]>,
    pub indices: Arc<Vec<u8>>,
    pub counts: Vec<u64>,
}
#[derive(Serialize)]
pub struct IndexedInfo {
    pub palette: Vec<[u8; 4]>,
    pub counts: Vec<u64>,
}
impl IndexedImage {
    pub fn new(palette: Vec<[u8; 4]>, indices: Vec<u8>, w: u32, h: u32) -> Result<Self, String> {
        crate::io::dimensions(w, h)?;
        if palette.is_empty()
            || palette.len() > 256
            || indices.len() != w as usize * h as usize
            || indices.iter().any(|&i| i as usize >= palette.len())
        {
            return Err("Palette ou indices incohérents".into());
        }
        let mut counts = vec![0; palette.len()];
        for &index in &indices {
            counts[index as usize] += 1;
        }
        Ok(Self {
            palette,
            indices: Arc::new(indices),
            counts,
        })
    }
    pub fn info(&self) -> IndexedInfo {
        IndexedInfo {
            palette: self.palette.clone(),
            counts: self.counts.clone(),
        }
    }
    pub fn expand(&self, w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba(self.palette[self.indices[(y * w + x) as usize] as usize])
        })
    }
    pub fn png(&self, w: u32, h: u32) -> Result<Vec<u8>, String> {
        self.png_metadata(w, h, &[])
    }
    pub fn png_metadata(&self, w: u32, h: u32, exif: &[u8]) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        {
            let mut info = png::Info::with_size(w, h);
            if !exif.is_empty() {
                info.exif_metadata = Some(std::borrow::Cow::Borrowed(exif));
            }
            let mut encoder =
                png::Encoder::with_info(&mut bytes, info).map_err(|e| e.to_string())?;
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(
                self.palette
                    .iter()
                    .flat_map(|p| [p[0], p[1], p[2]])
                    .collect::<Vec<_>>(),
            );
            encoder.set_trns(self.palette.iter().map(|p| p[3]).collect::<Vec<_>>());
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer
                .write_image_data(&self.indices)
                .map_err(|e| e.to_string())?;
        }
        Ok(bytes)
    }
    pub fn replace_color(&mut self, index: usize, color: [u8; 4]) -> Result<(), String> {
        let entry = self
            .palette
            .get_mut(index)
            .ok_or("Indice de palette invalide")?;
        *entry = color;
        Ok(())
    }
    /// Les pixels inchangés conservent leur indice, même si deux entrées ont la même couleur.
    pub fn map_edit(&self, before: &RgbaImage, after: &RgbaImage) -> Result<Self, String> {
        if before.dimensions() != after.dimensions()
            || before.as_raw().len() / 4 != self.indices.len()
        {
            return Err("Dimensions indexées incohérentes".into());
        }
        if !self.palette.iter().any(|p| p[3] < 128)
            && before
                .pixels()
                .zip(after.pixels())
                .any(|(a, b)| b[3] < 128 && b[3] < a[3])
        {
            return Err("La palette ne contient pas de transparence. Définissez une couleur transparente ou passez en RVB.".into());
        }
        let mut mapper = Mapper::new(&self.palette);
        let mut indices = Vec::with_capacity(self.indices.len());
        for (i, (a, b)) in before.pixels().zip(after.pixels()).enumerate() {
            indices.push(if a == b {
                self.indices[i]
            } else {
                mapper.color(b.0)
            });
        }
        Self::new(self.palette.clone(), indices, after.width(), after.height())
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<Option<(u32, u32, IndexedImage)>, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: 128 * 1024 * 1024,
    });
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    if reader.info().color_type != png::ColorType::Indexed
        || reader.info().animation_control.is_some()
    {
        return Ok(None);
    }
    let info = reader.info();
    crate::io::dimensions(info.width, info.height)?;
    let rgb = info.palette.as_ref().ok_or("PNG indexé sans palette")?;
    if rgb.len() % 3 != 0 || rgb.is_empty() || rgb.len() > 768 {
        return Err("Palette PNG invalide".into());
    }
    let alpha = info.trns.as_deref().unwrap_or(&[]);
    let palette = rgb
        .chunks_exact(3)
        .enumerate()
        .map(|(i, p)| [p[0], p[1], p[2], alpha.get(i).copied().unwrap_or(255)])
        .collect::<Vec<_>>();
    let mut raw = vec![0; reader.output_buffer_size().ok_or("PNG trop grand")?];
    let output = reader.next_frame(&mut raw).map_err(|e| e.to_string())?;
    let bits = output.bit_depth as usize;
    if ![1, 2, 4, 8].contains(&bits) {
        return Err("Profondeur de palette PNG invalide".into());
    }
    let mask = (1u16 << bits) - 1;
    let mut indices = Vec::with_capacity(output.width as usize * output.height as usize);
    for y in 0..output.height as usize {
        let row = &raw[y * output.line_size..(y + 1) * output.line_size];
        for x in 0..output.width as usize {
            let shift = 8 - bits - (x * bits % 8);
            indices.push(((row[x * bits / 8] as u16 >> shift) & mask) as u8);
        }
    }
    Ok(Some((
        output.width,
        output.height,
        IndexedImage::new(palette, indices, output.width, output.height)?,
    )))
}

pub fn validate_document(doc: &crate::document::Document) -> Result<(), String> {
    let Some(indexed) = &doc.indexed else {
        return Ok(());
    };
    if doc.animation.is_some() || doc.layers.len() != 1 {
        return Err("Un document indexé doit contenir un seul bitmap".into());
    }
    let layer = &doc.layers[0];
    if layer.x != 0
        || layer.y != 0
        || layer.opacity != 1.0
        || !layer.visible
        || !matches!(layer.blend, crate::document::BlendMode::Normal)
        || !layer.effects.is_empty()
    {
        return Err("Propriétés de calque incompatibles avec le mode indexé".into());
    }
    let img = layer.raster().ok_or("Bitmap indexé manquant")?;
    if img.dimensions() != (doc.width, doc.height)
        || indexed.indices.len() != doc.width as usize * doc.height as usize
        || indexed.palette.is_empty()
        || indexed.palette.len() > 256
        || indexed
            .indices
            .iter()
            .any(|&i| i as usize >= indexed.palette.len())
    {
        return Err("Données indexées incohérentes".into());
    }
    if !img
        .pixels()
        .zip(indexed.indices.iter())
        .all(|(p, &i)| p.0 == indexed.palette[i as usize])
    {
        return Err("Le bitmap ne correspond pas à sa palette".into());
    }
    Ok(())
}

#[derive(Clone)]
struct Bin {
    color: [u8; 4],
    weight: u64,
}
#[derive(Clone, Default)]
struct Acc {
    sum: [u64; 4],
    weight: u64,
}
fn key(p: [u8; 4]) -> usize {
    ((p[0] as usize >> 3) << 13)
        | ((p[1] as usize >> 3) << 8)
        | ((p[2] as usize >> 3) << 3)
        | (p[3] as usize >> 5)
}
fn vector(p: [u8; 4]) -> [f32; 4] {
    let a = p[3] as f32 / 255.0;
    [
        p[0] as f32 * a,
        p[1] as f32 * a,
        p[2] as f32 * a,
        p[3] as f32,
    ]
}
fn nearest(p: [f32; 4], palette: &[[f32; 4]]) -> u8 {
    palette
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let distance = |v: &[f32; 4]| {
                (p[0] - v[0]).powi(2)
                    + (p[1] - v[1]).powi(2)
                    + (p[2] - v[2]).powi(2)
                    + 2.0 * (p[3] - v[3]).powi(2)
            };
            distance(a).total_cmp(&distance(b))
        })
        .map_or(0, |(i, _)| i as u8)
}
struct Mapper<'a> {
    palette: &'a [[u8; 4]],
    vectors: Vec<[f32; 4]>,
    exact: HashMap<[u8; 4], u8>,
    cache: Vec<u16>,
}
impl<'a> Mapper<'a> {
    fn new(palette: &'a [[u8; 4]]) -> Self {
        let mut exact = HashMap::new();
        for (i, &p) in palette.iter().enumerate() {
            exact.entry(p).or_insert(i as u8);
        }
        Self {
            palette,
            vectors: palette.iter().copied().map(vector).collect(),
            exact,
            cache: vec![u16::MAX; 1 << 18],
        }
    }
    fn color(&mut self, p: [u8; 4]) -> u8 {
        if let Some(&i) = self.exact.get(&p) {
            return i;
        }
        let k = key(p);
        if self.cache[k] != u16::MAX {
            return self.cache[k] as u8;
        }
        let i = nearest(vector(p), &self.vectors);
        self.cache[k] = i as u16;
        i
    }
}
fn palette(img: &RgbaImage, count: usize) -> Vec<[u8; 4]> {
    let mut exact = HashMap::<[u8; 4], u64>::new();
    for p in img.pixels() {
        let p = if p[3] == 0 { [0; 4] } else { p.0 };
        *exact.entry(p).or_default() += 1;
        if exact.len() > count {
            break;
        }
    }
    if exact.len() <= count {
        let mut colors = exact.keys().copied().collect::<Vec<_>>();
        colors.sort();
        return colors;
    }
    let transparent = img.pixels().any(|p| p[3] == 0);
    let mut bins = vec![Acc::default(); 1 << 18];
    for p in img.pixels().filter(|p| p[3] > 0) {
        let a = &mut bins[key(p.0)];
        a.weight += 1;
        for c in 0..4 {
            a.sum[c] += p[c] as u64;
        }
    }
    let colors = bins
        .into_iter()
        .filter(|a| a.weight > 0)
        .map(|a| Bin {
            color: a.sum.map(|v| ((v + a.weight / 2) / a.weight) as u8),
            weight: a.weight,
        })
        .collect::<Vec<_>>();
    let mut boxes = vec![colors];
    let target = count - transparent as usize;
    let range = |b: &[Bin]| {
        let mut min = [255u8; 4];
        let mut max = [0u8; 4];
        for p in b {
            for c in 0..4 {
                min[c] = min[c].min(p.color[c]);
                max[c] = max[c].max(p.color[c]);
            }
        }
        let axis = (0..4).max_by_key(|&c| max[c] - min[c]).unwrap();
        (
            axis,
            (max[axis] - min[axis]) as u64 * b.iter().map(|p| p.weight).sum::<u64>().isqrt().max(1),
        )
    };
    while boxes.len() < target {
        let Some(index) = (0..boxes.len())
            .filter(|&i| boxes[i].len() > 1)
            .max_by_key(|&i| range(&boxes[i]).1)
        else {
            break;
        };
        let mut colors = boxes.swap_remove(index);
        let axis = range(&colors).0;
        colors.sort_by_key(|p| p.color[axis]);
        let total = colors.iter().map(|p| p.weight).sum::<u64>();
        let mut accumulated = 0;
        let mut split = colors.len() - 1;
        for (i, p) in colors.iter().enumerate().take(colors.len() - 1) {
            accumulated += p.weight;
            if accumulated * 2 >= total {
                split = i + 1;
                break;
            }
        }
        let right = colors.split_off(split);
        boxes.push(colors);
        boxes.push(right);
    }
    let mut result = Vec::new();
    if transparent {
        result.push([0; 4]);
    }
    for colors in boxes {
        let total = colors.iter().map(|p| p.weight).sum::<u64>().max(1);
        let color = std::array::from_fn(|c| {
            ((colors
                .iter()
                .map(|p| p.color[c] as u64 * p.weight)
                .sum::<u64>()
                + total / 2)
                / total) as u8
        });
        if !result.contains(&color) {
            result.push(color);
        }
    }
    result
}

pub fn quantize(img: &RgbaImage, colors: usize, dither: bool) -> Result<IndexedImage, String> {
    if !(2..=256).contains(&colors) {
        return Err("Une palette contient 2 à 256 couleurs maximum".into());
    }
    let palette = palette(img, colors);
    let mut mapper = Mapper::new(&palette);
    let (w, h) = img.dimensions();
    let mut indices = vec![0; w as usize * h as usize];
    let mut row_error = vec![[0f32; 4]; w as usize + 2];
    let mut next_error = row_error.clone();
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let mut sample = vector(p);
            if dither && p[3] > 0 {
                for c in 0..4 {
                    sample[c] = (sample[c] + row_error[x as usize + 1][c]).clamp(0.0, 255.0);
                }
            }
            let index = if dither && p[3] > 0 {
                nearest(sample, &mapper.vectors)
            } else {
                mapper.color(if p[3] == 0 { [0; 4] } else { p })
            };
            indices[(y * w + x) as usize] = index;
            if dither && p[3] > 0 {
                let mapped = vector(mapper.palette[index as usize]);
                for c in 0..4 {
                    let error = sample[c] - mapped[c];
                    row_error[x as usize + 2][c] += error * 7.0 / 16.0;
                    next_error[x as usize][c] += error * 3.0 / 16.0;
                    next_error[x as usize + 1][c] += error * 5.0 / 16.0;
                    next_error[x as usize + 2][c] += error / 16.0;
                }
            }
        }
        std::mem::swap(&mut row_error, &mut next_error);
        next_error.fill([0.0; 4]);
    }
    IndexedImage::new(palette, indices, w, h)
}

pub fn transform(
    state: &IndexedImage,
    w: u32,
    h: u32,
    selection: Option<&crate::document::Selection>,
    op: &crate::operations::Operation,
) -> Result<Option<(IndexedImage, u32, u32, &'static str)>, String> {
    use crate::operations::Operation;
    use image::{imageops, GrayImage, Luma};
    let source =
        GrayImage::from_raw(w, h, (*state.indices).clone()).ok_or("Indices incohérents")?;
    let background = state.palette.iter().position(|p| p[3] == 0).unwrap_or(0) as u8;
    let canvas = |nw: u32, nh: u32, ox: i32, oy: i32| -> Result<GrayImage, String> {
        crate::io::dimensions(nw, nh)?;
        Ok(GrayImage::from_fn(nw, nh, |x, y| {
            let (sx, sy) = (x as i64 - ox as i64, y as i64 - oy as i64);
            if sx >= 0 && sy >= 0 && sx < w as i64 && sy < h as i64 {
                *source.get_pixel(sx as u32, sy as u32)
            } else {
                Luma([background])
            }
        }))
    };
    let (out, label) = match op {
        Operation::Resize { width, height } => {
            crate::io::dimensions(*width, *height)?;
            (
                imageops::resize(&source, *width, *height, imageops::FilterType::Nearest),
                "Redimensionner l'image indexée",
            )
        }
        Operation::Canvas {
            width,
            height,
            offset_x,
            offset_y,
        } => (
            canvas(*width, *height, *offset_x, *offset_y)?,
            "Zone de travail indexée",
        ),
        Operation::Crop => {
            let (x, y, nw, nh) = selection
                .and_then(|s| s.bounds(w, h))
                .ok_or("Créez une sélection non vide avant de recadrer")?;
            (
                canvas(nw, nh, -(x as i32), -(y as i32))?,
                "Recadrer l'image indexée",
            )
        }
        Operation::Flip { horizontal } => (
            if *horizontal {
                imageops::flip_horizontal(&source)
            } else {
                imageops::flip_vertical(&source)
            },
            "Miroir indexé",
        ),
        Operation::Rotate { degrees } => {
            if !degrees.is_finite() {
                return Err("Angle invalide".into());
            }
            let deg = degrees.rem_euclid(360.0);
            let result = if deg == 0.0 {
                source.clone()
            } else if deg == 90.0 {
                imageops::rotate90(&source)
            } else if deg == 180.0 {
                imageops::rotate180(&source)
            } else if deg == 270.0 {
                imageops::rotate270(&source)
            } else {
                let (sn, cs) = deg.to_radians().sin_cos();
                let (nw, nh) = (
                    (w as f32 * cs.abs() + h as f32 * sn.abs()).ceil() as u32,
                    (w as f32 * sn.abs() + h as f32 * cs.abs()).ceil() as u32,
                );
                crate::io::dimensions(nw, nh)?;
                GrayImage::from_fn(nw, nh, |x, y| {
                    let (dx, dy) = (
                        x as f32 + 0.5 - nw as f32 / 2.0,
                        y as f32 + 0.5 - nh as f32 / 2.0,
                    );
                    let (sx, sy) = (
                        (cs * dx + sn * dy + w as f32 / 2.0).floor() as i32,
                        (-sn * dx + cs * dy + h as f32 / 2.0).floor() as i32,
                    );
                    if sx >= 0 && sy >= 0 && sx < w as i32 && sy < h as i32 {
                        *source.get_pixel(sx as u32, sy as u32)
                    } else {
                        Luma([background])
                    }
                })
            };
            (result, "Rotation indexée")
        }
        _ => return Ok(None),
    };
    let (nw, nh) = out.dimensions();
    Ok(Some((
        IndexedImage::new(state.palette.clone(), out.into_raw(), nw, nh)?,
        nw,
        nh,
        label,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_one_bit_png_indices_are_unpacked_by_row() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 3, 2);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::One);
            encoder.set_palette(vec![0, 0, 0, 255, 255, 255]);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0b01000000, 0b10100000]).unwrap();
        }
        let (_, _, image) = decode_png(&bytes).unwrap().unwrap();
        assert_eq!(*image.indices, vec![0, 1, 0, 1, 0, 1]);
    }
    #[test]
    fn exact_colors_alpha_and_duplicate_palette_indices_survive_png() {
        let indexed = IndexedImage::new(
            vec![[255, 0, 0, 255], [255, 0, 0, 255], [0, 0, 255, 100]],
            vec![0, 1, 2, 1],
            4,
            1,
        )
        .unwrap();
        let (_, _, mut restored) = decode_png(&indexed.png(4, 1).unwrap()).unwrap().unwrap();
        assert_eq!(*restored.indices, vec![0, 1, 2, 1]);
        restored.replace_color(1, [0, 255, 0, 255]).unwrap();
        assert_eq!(restored.expand(4, 1).get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(restored.expand(4, 1).get_pixel(1, 0).0, [0, 255, 0, 255]);
    }
    #[test]
    fn quantization_limits_real_colors_and_dithering_changes_gradient_distribution() {
        let img = RgbaImage::from_fn(128, 16, |x, _| Rgba([(x * 2) as u8; 4]));
        let a = quantize(&img, 8, false).unwrap();
        let b = quantize(&img, 8, true).unwrap();
        assert!(a.palette.len() <= 8);
        assert!(b.palette.len() <= 8);
        assert_ne!(a.indices, b.indices);
        assert_eq!(a.counts.iter().sum::<u64>(), 2048);
        let exact = RgbaImage::from_fn(4, 1, |x, _| {
            Rgba(
                [
                    [100, 0, 0, 255],
                    [101, 0, 0, 255],
                    [0, 255, 0, 128],
                    [20, 40, 60, 0],
                ][x as usize],
            )
        });
        let q = quantize(&exact, 8, false).unwrap();
        for (before, after) in exact.pixels().zip(q.expand(4, 1).pixels()) {
            if before[3] > 0 {
                assert_eq!(before, after);
            } else {
                assert_eq!(after[3], 0);
            }
        }
    }
    #[test]
    fn unchanged_pixels_keep_duplicate_indices_after_an_edit() {
        let old = IndexedImage::new(
            vec![[255, 0, 0, 255], [255, 0, 0, 255], [0, 0, 255, 255]],
            vec![0, 1, 2],
            3,
            1,
        )
        .unwrap();
        let before = old.expand(3, 1);
        let mut after = before.clone();
        after.put_pixel(2, 0, Rgba([250, 0, 0, 255]));
        let edited = old.map_edit(&before, &after).unwrap();
        assert_eq!(edited.indices[1], 1);
        assert_eq!(edited.indices[2], 0);
    }
}
