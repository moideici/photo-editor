//! Codecs supplémentaires et aperçu des fichiers photo bruts.
use crate::{io, metadata::Metadata};
use image::{DynamicImage, RgbaImage};
use std::{io::Cursor, sync::Arc};

pub fn avif(bytes: &[u8]) -> Result<(RgbaImage, Metadata), String> {
    let parsed = avif_parse::read_avif(&mut Cursor::new(bytes))
        .map_err(|e| format!("AVIF invalide : {e}"))?;
    let header = parsed.primary_item_metadata().map_err(|e| e.to_string())?;
    io::dimensions(header.max_frame_width.get(), header.max_frame_height.get())?;
    if let Some(alpha) = parsed.alpha_item_metadata().map_err(|e| e.to_string())? {
        io::dimensions(alpha.max_frame_width.get(), alpha.max_frame_height.get())?;
    }
    if parsed.premultiplied_alpha {
        return Err("Cet AVIF utilise un alpha prémultiplié non pris en charge par ce décodeur. Convertissez-le en alpha indépendant.".into());
    }
    let decoded = avif_decode::Decoder::from_avif(bytes)
        .and_then(|d| d.to_image())
        .map_err(|e| format!("Décodage AVIF : {e}"))?;
    use avif_decode::Image;
    let (w, h, data) = match decoded {
        Image::Rgb8(im) => (
            im.width(),
            im.height(),
            im.pixels().flat_map(|p| [p.r, p.g, p.b, 255]).collect(),
        ),
        Image::Rgba8(im) => (
            im.width(),
            im.height(),
            im.pixels().flat_map(|p| [p.r, p.g, p.b, p.a]).collect(),
        ),
        Image::Rgb16(im) => (
            im.width(),
            im.height(),
            im.pixels()
                .flat_map(|p| [(p.r >> 8) as u8, (p.g >> 8) as u8, (p.b >> 8) as u8, 255])
                .collect(),
        ),
        Image::Rgba16(im) => (
            im.width(),
            im.height(),
            im.pixels()
                .flat_map(|p| {
                    [
                        (p.r >> 8) as u8,
                        (p.g >> 8) as u8,
                        (p.b >> 8) as u8,
                        (p.a >> 8) as u8,
                    ]
                })
                .collect(),
        ),
        Image::Gray8(im) => (
            im.width(),
            im.height(),
            im.pixels()
                .flat_map(|p| [p.value(), p.value(), p.value(), 255])
                .collect(),
        ),
        Image::Gray16(im) => (
            im.width(),
            im.height(),
            im.pixels()
                .flat_map(|p| {
                    [
                        (p.value() >> 8) as u8,
                        (p.value() >> 8) as u8,
                        (p.value() >> 8) as u8,
                        255,
                    ]
                })
                .collect(),
        ),
    };
    io::dimensions(w as u32, h as u32)?;
    let image = RgbaImage::from_raw(w as u32, h as u32, data).ok_or("Tampon AVIF incohérent")?;
    let mut meta = Metadata {
        exif: Arc::new(iso_exif(bytes).unwrap_or_default()),
        ..Default::default()
    };
    let image = orient_iso(image, bytes, &mut meta);
    Ok((image, meta))
}

#[derive(Clone, Copy)]
struct BoxRef {
    kind: [u8; 4],
    payload: usize,
    end: usize,
}
fn boxes(bytes: &[u8], mut at: usize, end: usize) -> Option<Vec<BoxRef>> {
    let mut result = vec![];
    if end > bytes.len() {
        return None;
    }
    while at < end {
        if result.len() >= 4096 || end - at < 8 {
            return None;
        }
        let mut len = u32::from_be_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
        let kind = bytes[at + 4..at + 8].try_into().ok()?;
        let mut header = 8;
        if len == 1 {
            len = usize::try_from(u64::from_be_bytes(
                bytes.get(at + 8..at + 16)?.try_into().ok()?,
            ))
            .ok()?;
            header = 16;
        } else if len == 0 {
            len = end - at;
        }
        if len < header || at.checked_add(len)? > end {
            return None;
        }
        result.push(BoxRef {
            kind,
            payload: at + header,
            end: at + len,
        });
        at += len;
    }
    Some(result)
}
fn number(b: &[u8], at: &mut usize, n: usize) -> Option<u64> {
    if n > 8 {
        return None;
    }
    let end = at.checked_add(n)?;
    let mut v = 0u64;
    for &byte in b.get(*at..end)? {
        v = (v << 8) | u64::from(byte);
    }
    *at = end;
    Some(v)
}
fn meta_boxes(bytes: &[u8]) -> Option<Vec<BoxRef>> {
    let root = boxes(bytes, 0, bytes.len())?;
    let meta = root.iter().find(|b| &b.kind == b"meta")?;
    boxes(bytes, meta.payload + 4, meta.end)
}
/// Lit les extents de l'item Exif du conteneur HEIF/AVIF, sans scanner les pixels.
pub fn iso_exif(bytes: &[u8]) -> Option<Vec<u8>> {
    let children = meta_boxes(bytes)?;
    let iinf = children.iter().find(|b| &b.kind == b"iinf")?;
    let v = *bytes.get(iinf.payload)?;
    let entries = boxes(
        bytes,
        iinf.payload + 4 + if v == 0 { 2 } else { 4 },
        iinf.end,
    )?;
    let mut exif_id = None;
    for entry in entries.into_iter().filter(|b| &b.kind == b"infe") {
        let v = bytes[entry.payload];
        let mut p = entry.payload + 4;
        if v != 2 && v != 3 {
            continue;
        }
        let id = number(bytes, &mut p, if v == 2 { 2 } else { 4 })?;
        p += 2;
        if bytes.get(p..p + 4)? == b"Exif" {
            exif_id = Some(id);
            break;
        }
    }
    let exif_id = exif_id?;
    let iloc = children.iter().find(|b| &b.kind == b"iloc")?;
    let data = bytes.get(iloc.payload..iloc.end)?;
    let version = data[0];
    if version > 2 {
        return None;
    }
    let mut at = 4;
    let size = data.get(at)?;
    at += 1;
    let offset_n = (size >> 4) as usize;
    let length_n = (size & 15) as usize;
    let size = data.get(at)?;
    at += 1;
    let base_n = (size >> 4) as usize;
    let index_n = if version > 0 { (size & 15) as usize } else { 0 };
    let count = number(data, &mut at, if version < 2 { 2 } else { 4 })?;
    if count > 65536 {
        return None;
    }
    for _ in 0..count {
        let id = number(data, &mut at, if version < 2 { 2 } else { 4 })?;
        let method = if version > 0 {
            number(data, &mut at, 2)? & 15
        } else {
            0
        };
        let reference = number(data, &mut at, 2)?;
        let base = number(data, &mut at, base_n)?;
        let extents = number(data, &mut at, 2)?;
        let mut output = vec![];
        for _ in 0..extents {
            let _ = number(data, &mut at, index_n)?;
            let offset = number(data, &mut at, offset_n)?;
            let len = number(data, &mut at, length_n)?;
            if id != exif_id {
                continue;
            }
            if reference != 0 || method > 1 {
                return None;
            }
            let origin = if method == 1 {
                children.iter().find(|b| &b.kind == b"idat")?.payload as u64
            } else {
                0
            };
            let start = usize::try_from(origin.checked_add(base)?.checked_add(offset)?).ok()?;
            let len = usize::try_from(len).ok()?;
            if output.len().checked_add(len)? > crate::metadata::MAX_EXIF + 4 {
                return None;
            }
            output.extend_from_slice(bytes.get(start..start.checked_add(len)?)?);
        }
        if id == exif_id {
            let skip = u32::from_be_bytes(output.get(..4)?.try_into().ok()?) as usize;
            let tiff = output.get(4usize.checked_add(skip)?..)?.to_vec();
            return if matches!(tiff.get(..4), Some(b"II*\0" | b"MM\0*")) {
                Some(tiff)
            } else {
                None
            };
        }
    }
    None
}
fn iso_transforms(bytes: &[u8]) -> Option<Vec<BoxRef>> {
    let children = meta_boxes(bytes)?;
    let pitm = children.iter().find(|b| &b.kind == b"pitm")?;
    let mut at = pitm.payload + 4;
    let id = number(bytes, &mut at, if bytes[pitm.payload] == 0 { 2 } else { 4 })?;
    let iprp = children.iter().find(|b| &b.kind == b"iprp")?;
    let props = boxes(bytes, iprp.payload, iprp.end)?;
    let ipco = props.iter().find(|b| &b.kind == b"ipco")?;
    let list = boxes(bytes, ipco.payload, ipco.end)?;
    let mut result = vec![];
    for ipma in props.iter().filter(|b| &b.kind == b"ipma") {
        let data = bytes.get(ipma.payload..ipma.end)?;
        let mut at = 0;
        let vf = number(data, &mut at, 4)?;
        let version = vf >> 24;
        let count = number(data, &mut at, 4)?;
        if count > 65536 {
            return None;
        }
        for _ in 0..count {
            let item = number(data, &mut at, if version < 1 { 2 } else { 4 })?;
            let n = number(data, &mut at, 1)?;
            for _ in 0..n {
                let association = number(data, &mut at, if vf & 1 != 0 { 2 } else { 1 })?;
                let index = (association & if vf & 1 != 0 { 0x7fff } else { 0x7f }) as usize;
                if item == id && index > 0 {
                    result.push(*list.get(index - 1)?);
                }
            }
        }
    }
    Some(result)
}
pub fn orient_iso(image: RgbaImage, bytes: &[u8], meta: &mut Metadata) -> RgbaImage {
    let mut image = DynamicImage::ImageRgba8(image);
    let mut transformed = false;
    for property in iso_transforms(bytes).unwrap_or_default() {
        if property.payload >= property.end {
            continue;
        }
        match &property.kind {
            b"irot" => {
                image = match bytes[property.payload] & 3 {
                    1 => image.rotate270(),
                    2 => image.rotate180(),
                    3 => image.rotate90(),
                    _ => image,
                };
                transformed = true;
            }
            b"imir" => {
                image = if bytes[property.payload] & 1 == 0 {
                    image.fliph()
                } else {
                    image.flipv()
                };
                transformed = true;
            }
            b"clap" => meta
                .notices
                .push("Le recadrage de conteneur AVIF/HEIF (clap) n'est pas appliqué.".into()),
            _ => (),
        }
    }
    if !transformed {
        if let Some(o) = image::metadata::Orientation::from_exif_chunk(&meta.exif) {
            image.apply_orientation(o);
        }
    }
    meta.exif = Arc::new(meta.exported_exif(image.width(), image.height()));
    image.into_rgba8()
}

pub fn is_raw(ext: &str) -> bool {
    matches!(
        ext,
        "raw"
            | "cr2"
            | "cr3"
            | "nef"
            | "nrw"
            | "arw"
            | "raf"
            | "rw2"
            | "orf"
            | "pef"
            | "srw"
            | "dng"
            | "3fr"
            | "rwl"
    )
}
/// Saute les segments JPEG avant de chercher EOI, y compris une vignette dans APP1.
fn jpeg_end(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"\xff\xd8") {
        return None;
    }
    let mut at = 2;
    while at < bytes.len() {
        if bytes[at] != 255 {
            at += 1;
            continue;
        }
        while bytes.get(at) == Some(&255) {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        match marker {
            0xd9 => return Some(at),
            0 | 0xd0..=0xd8 | 0x01 => (),
            _ => {
                let len = u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as usize;
                if len < 2 {
                    return None;
                }
                at = at.checked_add(len)?;
            }
        }
    }
    None
}
pub fn raw_preview(bytes: &[u8], path: &str) -> Result<(RgbaImage, Metadata), String> {
    let mut candidates = vec![];
    for (at, window) in bytes.windows(3).enumerate() {
        if window != b"\xff\xd8\xff" {
            continue;
        }
        if candidates.len() >= 256 {
            break;
        }
        if let Some(len) = jpeg_end(&bytes[at..]) {
            if let Ok(reader) =
                image::ImageReader::new(Cursor::new(&bytes[at..at + len])).with_guessed_format()
            {
                if let Ok((w, h)) = reader.into_dimensions() {
                    if io::dimensions(w, h).is_ok() {
                        candidates.push((w as u64 * h as u64, at, len));
                    }
                }
            }
        }
    }
    candidates.sort_unstable_by_key(|c| std::cmp::Reverse(c.0));
    for (_, at, len) in candidates {
        if let Ok((image, mut meta)) = crate::metadata::decode(&bytes[at..at + len]) {
            meta.notices.push(
                "Aperçu JPEG intégré au RAW ; les données du capteur ne sont pas développées."
                    .into(),
            );
            return Ok((image, meta));
        }
    }
    let (image, mut meta) = wic(path, true).map_err(|e| {
        format!("Aucun aperçu RAW exploitable dans la limite de 16 mégapixels. {e}")
    })?;
    meta.notices.push(
        "Aperçu fourni par le codec RAW Windows ; les données du capteur ne sont pas développées."
            .into(),
    );
    Ok((image, meta))
}
#[cfg(not(windows))]
pub fn wic(_path: &str, _preview: bool) -> Result<(RgbaImage, Metadata), String> {
    Err("Le codec Windows est indisponible sur ce système".into())
}
#[cfg(windows)]
pub fn wic(path: &str, preview: bool) -> Result<(RgbaImage, Metadata), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows::{
        core::{w, Interface, PCWSTR},
        Win32::{
            Foundation::GENERIC_READ,
            Graphics::Imaging::*,
            System::Com::{StructuredStorage::*, *},
        },
    };
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() }
            }
        }
    }
    // Les interfaces COM restent sur ce thread jusqu'à leur destruction, avant l'appartement.
    let result = (|| -> windows::core::Result<(RgbaImage, Metadata)> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() && hr.0 != 0x80010106u32 as i32 {
                return Err(hr.into());
            }
            let _apartment = Apartment(hr.is_ok());
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
            let decoder = factory.CreateDecoderFromFilename(
                PCWSTR(wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = decoder.GetFrame(0)?;
            let source: IWICBitmapSource = if preview {
                decoder
                    .GetPreview()
                    .or_else(|_| frame.GetThumbnail())
                    .or_else(|_| decoder.GetThumbnail())?
            } else {
                frame.cast()?
            };
            let (mut w, mut h) = (0, 0);
            source.GetSize(&mut w, &mut h)?;
            io::dimensions(w, h).map_err(|_| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80070057u32 as i32),
                    "Dimensions hors limites",
                )
            })?;
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(
                &source,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )?;
            let mut data = vec![0; w as usize * h as usize * 4];
            converter.CopyPixels(std::ptr::null(), w * 4, &mut data)?;
            let mut image = DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, data).unwrap());
            if let Some(reader) = if preview {
                frame.GetMetadataQueryReader().ok()
            } else {
                None
            } {
                for query in [w!("/ifd/{ushort=274}"), w!("/app1/ifd/{ushort=274}")] {
                    let mut value = PROPVARIANT::default();
                    let read = reader.GetMetadataByName(query, &mut value);
                    let orientation = if read.is_ok() {
                        PropVariantToUInt32(&value).ok()
                    } else {
                        None
                    };
                    let _ = PropVariantClear(&mut value);
                    if let Some(o) = orientation
                        .and_then(|v| u8::try_from(v).ok())
                        .and_then(image::metadata::Orientation::from_exif)
                    {
                        image.apply_orientation(o);
                        break;
                    }
                }
            }
            Ok((image.into_rgba8(), Metadata::default()))
        }
    })();
    result.map_err(|e|format!("Codec Windows indisponible ou image non reconnue ({e}). Pour HEIC, les extensions HEIF et HEVC doivent être installées."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgba};
    #[test]
    fn exif_orientation_and_camera_data_survive_projects_and_four_exports() {
        let exif = crate::metadata::tests::fixture();
        let image = RgbaImage::from_fn(12, 20, |x, _| {
            if x < 6 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let mut bytes = vec![];
        let mut png = image::codecs::png::PngEncoder::new(&mut bytes);
        png.set_exif_metadata(exif).unwrap();
        png.write_image(image.as_raw(), 12, 20, image::ExtendedColorType::Rgba8)
            .unwrap();
        let dir = std::env::temp_dir().join(format!("prisme-formats-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("source.png");
        std::fs::write(&input, &bytes).unwrap();
        let doc = io::import(input.to_str().unwrap()).unwrap();
        assert_eq!((doc.width, doc.height), (20, 12));
        assert!(doc.metadata.exif.ends_with(b"Prism\0"));
        let pixels = crate::vector::flatten(&doc).unwrap();
        assert_eq!(pixels.get_pixel(5, 2).0, [255, 0, 0, 255]);
        assert_eq!(pixels.get_pixel(5, 10).0, [0, 0, 255, 255]);
        let doc = io::read_project(&io::project_bytes(&doc).unwrap()).unwrap();
        assert_eq!(crate::vector::flatten(&doc).unwrap(), pixels);
        for ext in ["png", "jpg", "webp", "avif"] {
            let output = dir.join(format!("output.{ext}"));
            io::export(&doc, output.to_str().unwrap(), 95).unwrap();
            let restored = io::import(output.to_str().unwrap()).unwrap();
            assert_eq!((restored.width, restored.height), (20, 12), "{ext}");
            assert!(restored.metadata.exif.ends_with(b"Prism\0"), "{ext}");
            assert_eq!(
                image::metadata::Orientation::from_exif_chunk(&restored.metadata.exif),
                Some(image::metadata::Orientation::NoTransforms)
            );
            let rgba = crate::vector::flatten(&restored).unwrap();
            let red = rgba.get_pixel(5, 2);
            assert!(red[0] > 230 && red[2] < 25, "{ext}: {red:?}");
            std::fs::remove_file(output).unwrap();
        }
        std::fs::remove_file(input).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn raw_preview_chooses_largest_complete_jpeg_and_wic_decodes_rgba() {
        let mut raw = b"Synthetic RAW container\0".to_vec();
        for (w, h, ink) in [(4, 3, [0, 255, 0]), (24, 16, [255, 0, 0])] {
            let image = image::RgbImage::from_pixel(w, h, image::Rgb(ink));
            let mut jpeg = vec![];
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
                .encode_image(&image)
                .unwrap();
            raw.extend(jpeg);
        }
        let (image, meta) = raw_preview(&raw, "unused.raw").unwrap();
        assert_eq!(image.dimensions(), (24, 16));
        assert!(!meta.notices.is_empty());
        assert!(image.get_pixel(10, 10)[0] > 240);
        #[cfg(windows)]
        {
            let path = std::env::temp_dir().join(format!("prisme-wic-{}.png", std::process::id()));
            std::fs::write(&path, io::png_bytes(&image).unwrap()).unwrap();
            let (decoded, _) = wic(path.to_str().unwrap(), false).unwrap();
            assert_eq!(decoded, image);
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn avif_alpha_round_trips_without_system_codec() {
        let mut editor = crate::document::Editor::default();
        crate::operations::execute(
            &mut editor,
            crate::operations::Operation::New {
                width: 16,
                height: 16,
                background: Some("#2080e080".into()),
            },
        )
        .unwrap();
        let bytes = io::export_bytes(editor.doc().unwrap(), "avif", 100).unwrap();
        let (image, _) = avif(&bytes).unwrap();
        assert_eq!(image.dimensions(), (16, 16));
        let p = image.get_pixel(8, 8);
        for (a, b) in p.0.iter().zip([32u8, 128, 224, 128]) {
            assert!(a.abs_diff(b) < 4, "{p:?}");
        }
        assert!(avif(b"invalid").is_err());
    }
    #[test]
    #[ignore = "Fichiers HEIC et RAW indépendants dans work/format-fixtures"]
    fn external_camera_files() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../work/format-fixtures");
        let path = dir.join("canon-preview.cr2");
        let bytes = std::fs::read(&path).unwrap();
        let (image, meta) = raw_preview(&bytes, path.to_str().unwrap()).unwrap();
        assert!(image.width() > 1000);
        std::fs::write(dir.join("raw-preview.png"), io::png_bytes(&image).unwrap()).unwrap();
        let path = dir.join("example.heic");
        let result = io::import(path.to_str().unwrap());
        let heic = match result {
            Ok(doc) => {
                std::fs::write(
                    dir.join("heic-import.png"),
                    io::png_bytes(&crate::vector::flatten(&doc).unwrap()).unwrap(),
                )
                .unwrap();
                serde_json::json!({"decoded":true,"width":doc.width,"height":doc.height})
            }
            Err(error) => serde_json::json!({"decoded":false,"reason":error}),
        };
        std::fs::write(dir.join("camera-report.json"),serde_json::to_vec_pretty(&serde_json::json!({"raw":{"width":image.width(),"height":image.height(),"notices":meta.notices},"heic":heic})).unwrap()).unwrap();
    }
}
