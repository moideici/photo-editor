//! Lecture PSD RGB 8 bits, d'après la spécification publique Adobe.
//! Les caches de pixels deviennent des calques bitmap ; aucun code Adobe n'est utilisé.
use crate::{
    document::{BlendMode, Content, Document, Layer},
    io,
    metadata::Metadata,
};
use image::{Rgba, RgbaImage};
use std::{io::Read, sync::Arc};
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
    fn left(&self) -> usize {
        self.data.len() - self.at
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("Taille PSD invalide")?;
        let data = self.data.get(self.at..end).ok_or("PSD tronqué")?;
        self.at = end;
        Ok(data)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i16(&mut self) -> Result<i16, String> {
        Ok(self.u16()? as i16)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }
    fn section(&mut self) -> Result<Reader<'a>, String> {
        let n = self.u32()? as usize;
        Ok(Reader::new(self.take(n)?))
    }
    fn rect(&mut self) -> Result<Rect, String> {
        let r = Rect {
            top: self.i32()?,
            left: self.i32()?,
            bottom: self.i32()?,
            right: self.i32()?,
        };
        r.size()?;
        Ok(r)
    }
}
#[derive(Clone, Copy)]
struct Rect {
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
}
impl Rect {
    fn size(&self) -> Result<(u32, u32), String> {
        let w = i64::from(self.right) - i64::from(self.left);
        let h = i64::from(self.bottom) - i64::from(self.top);
        if w < 0
            || h < 0
            || self.left.unsigned_abs() > 1_000_000
            || self.top.unsigned_abs() > 1_000_000
        {
            return Err("Rectangle PSD invalide".into());
        }
        let (w, h) = (
            u32::try_from(w).map_err(|_| "Largeur PSD invalide")?,
            u32::try_from(h).map_err(|_| "Hauteur PSD invalide")?,
        );
        if w > 0 && h > 0 {
            io::dimensions(w, h)?;
        }
        Ok((w, h))
    }
}
struct Mask {
    rect: Rect,
    default: u8,
    flags: u8,
    density: u8,
}
struct Record {
    rect: Rect,
    name: String,
    visible: bool,
    opacity: f32,
    blend: BlendMode,
    channels: Vec<(i16, usize)>,
    mask: Option<Mask>,
    group: u32,
    clipping: bool,
    adjustment: bool,
}
fn note(meta: &mut Metadata, message: &str) {
    if !meta.notices.iter().any(|s| s == message) {
        meta.notices.push(message.into());
    }
}
fn blend(key: &[u8], meta: &mut Metadata) -> BlendMode {
    match key {
        b"norm" | b"pass" => BlendMode::Normal,
        b"mul " => BlendMode::Multiply,
        b"scrn" => BlendMode::Screen,
        b"over" => BlendMode::Overlay,
        b"dark" => BlendMode::Darken,
        b"lite" => BlendMode::Lighten,
        b"diff" => BlendMode::Difference,
        b"lddg" => BlendMode::Add,
        _ => {
            note(
                meta,
                "Certains modes de fusion PSD sont remplacés par Normal.",
            );
            BlendMode::Normal
        }
    }
}
fn record(r: &mut Reader, meta: &mut Metadata) -> Result<Record, String> {
    let rect = r.rect()?;
    let n = r.u16()? as usize;
    if n > 56 {
        return Err("Trop de canaux PSD".into());
    }
    let mut channels = vec![];
    for _ in 0..n {
        channels.push((r.i16()?, r.u32()? as usize));
    }
    if r.take(4)? != b"8BIM" {
        return Err("Signature de calque PSD invalide".into());
    }
    let blend = blend(r.take(4)?, meta);
    let opacity = r.byte()? as f32 / 255.0;
    let clipping = r.byte()? != 0;
    let flags = r.byte()?;
    r.byte()?;
    let mut extra = r.section()?;
    let mut m = extra.section()?;
    let mask = if m.left() >= 18 {
        let rect = m.rect()?;
        let default = m.byte()?;
        let flags = m.byte()?;
        let mut density = 255;
        if flags & 16 != 0 && m.left() > 0 {
            let params = m.byte()?;
            if params & 1 != 0 {
                density = m.byte()?;
            }
            if params & 2 != 0 {
                m.take(8)?;
                note(
                    meta,
                    "La progressivité des masques PSD n'est pas reproduite.",
                );
            }
        }
        Some(Mask {
            rect,
            default,
            flags,
            density,
        })
    } else {
        None
    };
    let ranges = extra.section()?;
    if ranges
        .data
        .chunks_exact(8)
        .any(|b| b != [0, 0, 255, 255, 0, 0, 255, 255])
    {
        note(
            meta,
            "Les plages de fusion conditionnelles PSD ne sont pas reproduites.",
        );
    }
    let len = extra.byte()? as usize;
    let mut name = String::from_utf8_lossy(extra.take(len)?).into_owned();
    extra.take((4 - (len + 1) % 4) % 4)?;
    let mut group = 0;
    let mut adjustment = false;
    while extra.left() >= 12 {
        let signature = extra.take(4)?;
        if signature != b"8BIM" && signature != b"8B64" {
            break;
        }
        let key = extra.take(4)?;
        let mut value = extra.section()?;
        if value.data.len() % 2 == 1 {
            extra.take(1)?;
        }
        match key {
   b"luni"=>{let n=value.u32()? as usize;if n>100_000{return Err("Nom de calque PSD trop long".into());}let chars=value.take(n.checked_mul(2).ok_or("Nom PSD invalide")?)?.chunks_exact(2).map(|c|u16::from_be_bytes([c[0],c[1]])).collect::<Vec<_>>();name=String::from_utf16_lossy(&chars).trim_end_matches('\0').into();},
   b"lsct"|b"lsdk"=>{group=value.u32()?;},
   b"lrFX"|b"lfx2"=>note(meta,"Les effets Photoshop ne sont pas reproduits ; les caches de pixels des calques sont conservés."),
   b"TySh"|b"tySh"|b"SoLd"|b"SoLE"=>note(meta,"Les textes et objets dynamiques PSD sont importés comme pixels."),
   b"vmsk"|b"vsms"=>note(meta,"Les masques vectoriels PSD ne sont pas interprétés."),
   b"brit"|b"levl"|b"curv"|b"expA"|b"vibA"|b"hue "|b"hue2"|b"blnc"|b"blwh"|b"phfl"|b"mixr"|b"clrL"|b"nvrt"|b"post"|b"thrs"|b"grdm"|b"selc"=>adjustment=true,
   _=>(),
  }
    }
    Ok(Record {
        rect,
        name,
        visible: flags & 2 == 0,
        opacity,
        blend,
        channels,
        mask,
        group,
        clipping,
        adjustment,
    })
}
fn plane(data: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let expected = (w as usize)
        .checked_mul(h as usize)
        .ok_or("Canal PSD trop grand")?;
    if expected > 128 * 1024 * 1024 {
        return Err("Canal PSD trop grand".into());
    }
    let mut r = Reader::new(data);
    let compression = r.u16()?;
    let mut out = match compression {
        0 => r.take(expected)?.to_vec(),
        1 => {
            let mut lengths = Vec::with_capacity(h as usize);
            for _ in 0..h {
                lengths.push(r.u16()? as usize);
            }
            let mut out = Vec::with_capacity(expected);
            for len in lengths {
                let mut row = Reader::new(r.take(len)?);
                let start = out.len();
                while row.left() > 0 {
                    let control = row.byte()? as i8;
                    if control >= 0 {
                        let n = control as usize + 1;
                        if out.len() - start + n > w as usize {
                            return Err("Ligne RLE PSD trop longue".into());
                        }
                        out.extend(row.take(n)?);
                    } else if control != -128 {
                        let n = (1i16 - control as i16) as usize;
                        if out.len() - start + n > w as usize {
                            return Err("Ligne RLE PSD trop longue".into());
                        }
                        let value = row.byte()?;
                        out.resize(out.len() + n, value);
                    }
                }
                if out.len() - start != w as usize {
                    return Err("Ligne RLE PSD incomplète".into());
                }
            }
            out
        }
        2 | 3 => {
            let mut out = Vec::with_capacity(expected);
            flate2::read::ZlibDecoder::new(&r.data[r.at..])
                .take(expected as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| e.to_string())?;
            out
        }
        _ => return Err("Compression PSD inconnue".into()),
    };
    if out.len() != expected {
        return Err("Taille de canal PSD incohérente".into());
    }
    if compression == 3 && w > 0 {
        for row in out.chunks_mut(w as usize) {
            for i in 1..row.len() {
                row[i] = row[i].wrapping_add(row[i - 1]);
            }
        }
    }
    Ok(out)
}
pub fn read(bytes: &[u8], name: &str) -> Result<Document, String> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != b"8BPS" || r.u16()? != 1 {
        return Err("PSD version 1 requis ; PSB n'est pas encore pris en charge".into());
    }
    r.take(6)?;
    let channels = r.u16()?;
    if !(3..=56).contains(&channels) {
        return Err("Nombre de canaux PSD invalide".into());
    }
    let (h, w) = (r.u32()?, r.u32()?);
    io::dimensions(w, h)?;
    if r.u16()? != 8 || r.u16()? != 3 {
        return Err("L'import des calques PSD requiert le mode RVB 8 bits".into());
    }
    r.section()?;
    let mut resources = r.section()?;
    let mut meta = Metadata::default();
    while resources.left() >= 12 {
        if resources.take(4)? != b"8BIM" {
            return Err("Ressource PSD invalide".into());
        }
        let id = resources.u16()?;
        let n = resources.byte()? as usize;
        resources.take(n)?;
        if (n + 1) % 2 == 1 {
            resources.take(1)?;
        }
        let data = resources.section()?;
        if matches!(id, 1058 | 1059) && data.data.len() <= crate::metadata::MAX_EXIF {
            meta.exif = Arc::new(data.data.to_vec());
        }
        if data.data.len() % 2 == 1 {
            resources.take(1)?;
        }
    }
    let mut section = r.section()?;
    let mut info = if section.left() >= 4 {
        section.section()?
    } else {
        Reader::new(&[])
    };
    let mut records = vec![];
    if info.left() >= 2 {
        let n = info.i16()?.unsigned_abs() as usize;
        if n > 512 {
            return Err("PSD : plus de 512 enregistrements de calques".into());
        }
        for _ in 0..n {
            records.push(record(&mut info, &mut meta)?);
        }
    }
    let mut layers = vec![];
    let mut clips = vec![];
    let mut groups: Vec<(bool, f32)> = vec![];
    let mut total = 0usize;
    let mut decoded = vec![];
    for record in records {
        let (lw, lh) = record.rect.size()?;
        total = total.saturating_add(lw as usize * lh as usize * 4);
        if total > 512 * 1024 * 1024 {
            return Err("Pixels des calques PSD supérieurs à 512 Mio".into());
        }
        let mut image = RgbaImage::from_pixel(lw, lh, Rgba([0, 0, 0, 255]));
        let mut has_rgb = false;
        let mut mask_pixels = None;
        for &(channel, len) in &record.channels {
            let data = info.take(len)?;
            if record.group != 0 || lw == 0 || lh == 0 {
                continue;
            }
            if matches!(channel, 0..=2 | -1) {
                let values = plane(data, lw, lh)?;
                let component = if channel == -1 {
                    3
                } else {
                    has_rgb = true;
                    channel as usize
                };
                for (pixel, value) in image.pixels_mut().zip(values) {
                    pixel[component] = value;
                }
            } else if channel == -2 {
                if let Some(mask) = &record.mask {
                    let (mw, mh) = mask.rect.size()?;
                    if mw > 0 && mh > 0 {
                        total = total.saturating_add(mw as usize * mh as usize);
                        if total > 512 * 1024 * 1024 {
                            return Err("Pixels et masques PSD supérieurs à 512 Mio".into());
                        }
                        mask_pixels = Some((plane(data, mw, mh)?, mw, mh));
                    }
                }
            } else if channel == -3 {
                note(
                    &mut meta,
                    "Le masque utilisateur combiné PSD (-3) n'est pas interprété.",
                );
            }
        }
        decoded.push((record, image, mask_pixels, has_rgb));
    }
    // Les enregistrements PSD vont du bas vers le haut. On parcourt les dossiers
    // en sens inverse pour propager leurs propriétés aux enfants.
    for (record, mut image, mask_pixels, has_rgb) in decoded.into_iter().rev() {
        match record.group {
            1 | 2 => {
                if record.opacity < 1.0 || record.blend != BlendMode::Normal {
                    note(&mut meta,"Les groupes PSD sont dépliés ; leur opacité est appliquée à chaque calque et leur fusion isolée n'est pas reproduite.");
                }
                groups.push((record.visible, record.opacity));
                continue;
            }
            3 => {
                groups.pop();
                continue;
            }
            _ => (),
        }
        if record.adjustment || (!has_rgb && image.width() > 0 && image.height() > 0) {
            note(
                &mut meta,
                "Les calques de réglage et les calques sans cache RVB sont ignorés.",
            );
            continue;
        }
        if image.width() == 0 || image.height() == 0 {
            image = RgbaImage::new(1, 1);
        }
        if let Some(mask) = record.mask.filter(|m| m.flags & 2 == 0) {
            let x = mask.rect.left as i64
                + if mask.flags & 1 != 0 {
                    record.rect.left as i64
                } else {
                    0
                };
            let y = mask.rect.top as i64
                + if mask.flags & 1 != 0 {
                    record.rect.top as i64
                } else {
                    0
                };
            for (px, py, pixel) in image.enumerate_pixels_mut() {
                let (mx, my) = (
                    px as i64 + record.rect.left as i64 - x,
                    py as i64 + record.rect.top as i64 - y,
                );
                let mut a = mask_pixels
                    .as_ref()
                    .filter(|(_, mw, mh)| mx >= 0 && my >= 0 && mx < *mw as i64 && my < *mh as i64)
                    .map_or(mask.default, |(data, mw, _)| {
                        data[my as usize * *mw as usize + mx as usize]
                    });
                if mask.flags & 4 != 0 {
                    a = 255 - a;
                }
                a = 255 - ((255 - a) as u32 * mask.density as u32 / 255) as u8;
                pixel[3] = ((pixel[3] as u32 * a as u32 + 127) / 255) as u8;
            }
        }
        if layers.len() >= 256 {
            return Err("Plus de 256 calques PSD".into());
        }
        let id = layers.len() as u64 + 1;
        layers.push(Layer {
            locks: Default::default(),
            id,
            name: record.name,
            visible: record.visible && groups.iter().all(|g| g.0),
            opacity: record.opacity * groups.iter().map(|g| g.1).product::<f32>(),
            blend: record.blend,
            x: record.rect.left,
            y: record.rect.top,
            version: id,
            content: Content::Raster(Arc::new(image)),
            effects: Default::default(),
        });
        clips.push(record.clipping);
    }
    layers.reverse();
    clips.reverse();
    let mut base = None;
    for i in 0..layers.len() {
        if !clips[i] {
            base = Some(i);
            continue;
        }
        let Some(base) = base else {
            note(
                &mut meta,
                "Un calque écrêté PSD sans base est conservé sans écrêtage.",
            );
            continue;
        };
        let source = layers[base].raster().unwrap().clone();
        let (bx, by) = (layers[base].x, layers[base].y);
        let (x, y) = (layers[i].x, layers[i].y);
        let mut image = (**layers[i].raster().unwrap()).clone();
        for (px, py, pixel) in image.enumerate_pixels_mut() {
            let (sx, sy) = (
                px as i64 + x as i64 - bx as i64,
                py as i64 + y as i64 - by as i64,
            );
            let alpha = if sx >= 0
                && sy >= 0
                && sx < source.width() as i64
                && sy < source.height() as i64
            {
                source.get_pixel(sx as u32, sy as u32)[3]
            } else {
                0
            };
            pixel[3] = ((pixel[3] as u16 * alpha as u16 + 127) / 255) as u8;
        }
        layers[i].content = Content::Raster(Arc::new(image));
        note(&mut meta,"L'écrêtage PSD a été appliqué aux pixels ; son lien avec le calque de base n'est plus dynamique.");
    }
    if layers.is_empty() {
        let size = w as usize * h as usize;
        if size * channels as usize > 128 * 1024 * 1024 {
            return Err("Composite PSD trop grand".into());
        }
        let data = plane(&r.data[r.at..], w, h * channels as u32)?;
        let image = RgbaImage::from_fn(w, h, |x, y| {
            let at = (y * w + x) as usize;
            Rgba([data[at], data[size + at], data[2 * size + at], 255])
        });
        layers.push(Layer {
            locks: Default::default(),
            id: 1,
            name: "Image fusionnée".into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            x: 0,
            y: 0,
            version: 1,
            content: Content::Raster(Arc::new(image)),
            effects: Default::default(),
        });
    }
    meta.exif = Arc::new(meta.exported_exif(w, h));
    Ok(Document {
        text_styles: Vec::new(),
        width: w,
        height: h,
        name: name.into(),
        layers,
        selection: None,
        selections: Default::default(),
        indexed: None,
        animation: None,
        metadata: meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn section(data: Vec<u8>) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend(data);
        out
    }
    fn channel(values: &[u8], w: usize, compression: u16) -> Vec<u8> {
        let mut out = compression.to_be_bytes().to_vec();
        let mut values = values.to_vec();
        if compression == 3 {
            for row in values.chunks_mut(w) {
                for i in (1..row.len()).rev() {
                    row[i] = row[i].wrapping_sub(row[i - 1]);
                }
            }
        }
        match compression {
            0 => out.extend(values),
            1 => {
                for row in values.chunks(w) {
                    out.extend(((row.len() + 1) as u16).to_be_bytes());
                }
                for row in values.chunks(w) {
                    out.push(row.len() as u8 - 1);
                    out.extend(row);
                }
            }
            2 | 3 => {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&values).unwrap();
                out.extend(encoder.finish().unwrap());
            }
            _ => unreachable!(),
        }
        out
    }
    pub fn fixture(compression: u16) -> Vec<u8> {
        let mut records = vec![];
        let mut images = vec![];
        for (top, left, bottom, right, name, color, mask) in [
            (0i32, 0i32, 3i32, 4i32, "Fond", [255, 0, 0], false),
            (1, 1, 3, 3, "Bleu é", [0, 0, 255], true),
        ] {
            let (w, h) = ((right - left) as usize, (bottom - top) as usize);
            let mut r = vec![];
            for v in [top, left, bottom, right] {
                r.extend(v.to_be_bytes());
            }
            r.extend((if mask { 5u16 } else { 4 }).to_be_bytes());
            for c in 0i16..4 {
                let values = vec![if c == 3 { 255 } else { color[c as usize] }; w * h];
                let data = channel(&values, w, compression);
                r.extend((if c == 3 { -1i16 } else { c }).to_be_bytes());
                r.extend((data.len() as u32).to_be_bytes());
                images.extend(data);
            }
            if mask {
                let data = channel(&[255, 0, 255, 0], w, compression);
                r.extend((-2i16).to_be_bytes());
                r.extend((data.len() as u32).to_be_bytes());
                images.extend(data);
            }
            r.extend(b"8BIMnorm");
            r.extend([255, 0, 0, 0]);
            let mut extra = vec![];
            let mut m = vec![];
            if mask {
                for v in [top, left, bottom, right] {
                    m.extend(v.to_be_bytes());
                }
                m.extend([0, 0, 0, 0]);
            }
            extra.extend(section(m));
            extra.extend(0u32.to_be_bytes());
            extra.extend([0; 4]);
            let mut unicode = (name.encode_utf16().count() as u32).to_be_bytes().to_vec();
            for v in name.encode_utf16() {
                unicode.extend(v.to_be_bytes());
            }
            extra.extend(b"8BIMluni");
            extra.extend(section(unicode));
            r.extend(section(extra));
            records.extend(r);
        }
        let mut info = (-2i16).to_be_bytes().to_vec();
        info.extend(records);
        info.extend(images);
        if info.len() % 2 == 1 {
            info.push(0);
        }
        let layer = section(section(info));
        let mut file = b"8BPS\0\x01\0\0\0\0\0\0\0\x04".to_vec();
        file.extend(3u32.to_be_bytes());
        file.extend(4u32.to_be_bytes());
        file.extend(8u16.to_be_bytes());
        file.extend(3u16.to_be_bytes());
        file.extend(0u32.to_be_bytes());
        file.extend(0u32.to_be_bytes());
        file.extend(layer);
        file.extend([0u8; 2]);
        file.extend([0u8; 48]);
        file
    }
    #[test]
    fn layers_masks_unicode_and_all_four_compressions() {
        for compression in 0..4 {
            let bytes = fixture(compression);
            let d = read(&bytes, "Exemple.psd").unwrap();
            assert_eq!(d.layers.len(), 2);
            assert_eq!(d.layers[1].name, "Bleu é");
            assert_eq!((d.layers[1].x, d.layers[1].y), (1, 1));
            let image = crate::vector::flatten(&d).unwrap();
            assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
            assert_eq!(image.get_pixel(1, 1).0, [0, 0, 255, 255]);
            assert_eq!(image.get_pixel(2, 1).0, [255, 0, 0, 255]);
            let restored = io::read_project(&io::project_bytes(&d).unwrap()).unwrap();
            assert_eq!(crate::vector::flatten(&restored).unwrap(), image);
        }
    }
    #[test]
    fn truncated_psd_does_not_panic() {
        let bytes = fixture(1);
        for end in 0..bytes.len() {
            assert!(std::panic::catch_unwind(|| read(&bytes[..end], "x")).is_ok());
        }
    }
    #[test]
    #[ignore = "Fichiers de référence indépendants dans work/format-fixtures"]
    fn external_reference_files() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work/format-fixtures");
        let mut report = vec![];
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "psd") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy();
            let doc = read(&std::fs::read(&path).unwrap(), &name)
                .unwrap_or_else(|e| panic!("{name} : {e}"));
            let rendered = crate::vector::flatten(&doc).unwrap();
            std::fs::write(
                path.with_extension("prisme.png"),
                io::png_bytes(&rendered).unwrap(),
            )
            .unwrap();
            report.push(serde_json::json!({"file":name,"layers":doc.layers.iter().map(|l|serde_json::json!({"name":l.name,"x":l.x,"y":l.y,"width":l.raster().unwrap().width(),"height":l.raster().unwrap().height(),"opacity":l.opacity,"visible":l.visible})).collect::<Vec<_>>(),"notices":doc.metadata.notices}));
        }
        std::fs::write(
            dir.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        assert!(!report.is_empty());
    }
}
