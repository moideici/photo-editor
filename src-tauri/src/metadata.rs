//! EXIF conservé comme bloc TIFF ; orientation appliquée une seule fois à l'import.
use image::{metadata::Orientation, DynamicImage, ImageDecoder};
use serde::Serialize;
use std::{io::Cursor, sync::Arc};
pub const MAX_EXIF: usize = 1024 * 1024;
#[derive(Clone, Default)]
pub struct Metadata {
    pub exif: Arc<Vec<u8>>,
    pub notices: Vec<String>,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataInfo {
    pub has_exif: bool,
    pub notices: Vec<String>,
}
impl Metadata {
    pub fn info(&self) -> MetadataInfo {
        MetadataInfo {
            has_exif: !self.exif.is_empty(),
            notices: self.notices.clone(),
        }
    }
    pub fn exported_exif(&self, w: u32, h: u32) -> Vec<u8> {
        let mut data = (*self.exif).clone();
        normalize(&mut data, w, h);
        data
    }
}
pub fn read_exif(bytes: &[u8]) -> Vec<u8> {
    let read = (|| {
        let reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let mut decoder = reader.into_decoder().ok()?;
        decoder.exif_metadata().ok().flatten()
    })();
    read.filter(|v| v.len() <= MAX_EXIF).unwrap_or_default()
}
pub fn decode(bytes: &[u8]) -> Result<(image::RgbaImage, Metadata), String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(crate::io::MAX_PIXELS * 8);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("Image illisible : {e}"))?;
    let (w, h) = decoder.dimensions();
    crate::io::dimensions(w, h)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let exif = decoder.exif_metadata().unwrap_or(None).unwrap_or_default();
    let mut meta = Metadata::default();
    if exif.len() <= MAX_EXIF {
        meta.exif = Arc::new(exif);
    } else {
        meta.notices
            .push("EXIF supérieur à 1 Mio : bloc non conservé.".into());
    }
    let mut image = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    meta.exif = Arc::new(meta.exported_exif(image.width(), image.height()));
    Ok((image.into_rgba8(), meta))
}
fn read16(b: &[u8], offset: usize, le: bool) -> Option<u16> {
    let a = b.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(if le {
        u16::from_le_bytes(a)
    } else {
        u16::from_be_bytes(a)
    })
}
fn read32(b: &[u8], offset: usize, le: bool) -> Option<u32> {
    let a = b.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(if le {
        u32::from_le_bytes(a)
    } else {
        u32::from_be_bytes(a)
    })
}
fn write(b: &mut [u8], offset: usize, value: u32, short: bool, le: bool) {
    if short {
        if let Some(dst) = b.get_mut(offset..offset + 2) {
            dst.copy_from_slice(&if le {
                (value.min(65535) as u16).to_le_bytes()
            } else {
                (value.min(65535) as u16).to_be_bytes()
            });
        }
    } else if let Some(dst) = b.get_mut(offset..offset + 4) {
        dst.copy_from_slice(&if le {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        });
    }
}
/// Ne déplace aucun octet : les offsets MakerNote restent valides. Retire la référence
/// à l'ancienne vignette et met à jour les champs de dimensions déjà présents.
pub fn normalize(data: &mut [u8], w: u32, h: u32) {
    let le = match data.get(..4) {
        Some(b"II*\0") => true,
        Some(b"MM\0*") => false,
        _ => return,
    };
    let _ = Orientation::remove_from_exif_chunk(data);
    let Some(root) = read32(data, 4, le) else {
        return;
    };
    let mut pending = vec![root as usize];
    let mut seen = std::collections::HashSet::new();
    while let Some(offset) = pending.pop() {
        if seen.len() > 16 || !seen.insert(offset) {
            continue;
        }
        let Some(count) = read16(data, offset, le).map(usize::from) else {
            continue;
        };
        if count > 4096
            || offset
                .checked_add(2 + count * 12 + 4)
                .is_none_or(|end| end > data.len())
        {
            continue;
        }
        for i in 0..count {
            let pos = offset + 2 + i * 12;
            let tag = read16(data, pos, le).unwrap();
            let kind = read16(data, pos + 2, le).unwrap();
            let len = read32(data, pos + 4, le).unwrap();
            if tag == 0x8769 && kind == 4 && len == 1 {
                pending.push(read32(data, pos + 8, le).unwrap() as usize);
            }
            if len == 1 && (kind == 3 || kind == 4) {
                match tag {
                    0x100 | 0xa002 => write(data, pos + 8, w, kind == 3, le),
                    0x101 | 0xa003 => write(data, pos + 8, h, kind == 3, le),
                    _ => (),
                }
            }
        }
        if offset == root as usize {
            write(data, offset + 2 + count * 12, 0, false, le);
        }
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture() -> Vec<u8> {
        let mut b = b"II*\0\x08\0\0\0".to_vec();
        b.extend(4u16.to_le_bytes());
        for (tag, kind, count, value) in [
            (0x112u16, 3u16, 1u32, 6u32),
            (0x100, 4, 1, 2),
            (0x101, 4, 1, 3),
            (0x10f, 2, 6, 62),
        ] {
            b.extend(tag.to_le_bytes());
            b.extend(kind.to_le_bytes());
            b.extend(count.to_le_bytes());
            b.extend(value.to_le_bytes());
        }
        b.extend(0u32.to_le_bytes());
        b.extend(b"Prism\0");
        b
    }
    #[test]
    fn normalization_updates_dimensions_orientation_and_keeps_camera_bytes() {
        let mut b = fixture();
        normalize(&mut b, 100, 70);
        assert_eq!(
            Orientation::from_exif_chunk(&b),
            Some(Orientation::NoTransforms)
        );
        assert_eq!(read32(&b, 30, true), Some(100));
        assert_eq!(read32(&b, 42, true), Some(70));
        assert_eq!(&b[62..], b"Prism\0");
    }
}
