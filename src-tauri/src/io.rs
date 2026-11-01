//! Projets ZIP (JSON + PNG), import/export et écritures atomiques.
pub const PROJECT_VERSION:u32=18;
use crate::document::{BlendMode, Content, Document, Layer, SelShape, Selection, Shape, ShapeKind};
use image::{DynamicImage, ImageEncoder, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::Path,
    sync::Arc,
};

pub const MAX_PIXELS: u64 = 16_777_216;
pub fn dimensions(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 || w > 16384 || h > 16384 || w as u64 * h as u64 > MAX_PIXELS {
        return Err(
            "Dimensions invalides : 1 à 16 384 pixels par côté, 16 mégapixels maximum".into(),
        );
    }
    Ok(())
}

pub fn png_bytes(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

pub fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(MAX_PIXELS * 8);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|e| format!("Image illisible : {e}"))?;
    dimensions(img.width(), img.height())?;
    Ok(img.into_rgba8())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    #[serde(default)]
    text_styles: Vec<crate::text_styles::TextStyle>,
    version: u32,
    width: u32,
    height: u32,
    name: String,
    selection: Option<SavedSelection>,
    #[serde(default)]
    selections: std::collections::BTreeMap<String, SavedSelection>,
    layers: Vec<SavedLayer>,
    #[serde(default)]
    indexed: Option<String>,
    #[serde(default)]
    animation: Option<SavedAnimation>,
    #[serde(default)]
    exif: Option<String>,
    #[serde(default)]
    notices: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedAnimation {
    frames: Vec<SavedFrame>,
    current: usize,
    loops: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedFrame {
    image: String,
    delay_ms: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SavedSelection {
    Inline(Selection),
    Mask { mask: String, inverted: bool },
}

fn save_selection(
    zip: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    selection: &Selection,
    index: usize,
) -> Result<SavedSelection, String> {
    if let SelShape::Mask {
        width,
        height,
        data,
    } = &selection.shape
    {
        let img = image::GrayImage::from_raw(*width, *height, (**data).clone())
            .ok_or("Masque invalide")?;
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageLuma8(img)
            .write_to(&mut png, ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let mask = format!("selections/{index}.png");
        zip.start_file(
            &mask,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(|e| e.to_string())?;
        zip.write_all(&png.into_inner())
            .map_err(|e| e.to_string())?;
        Ok(SavedSelection::Mask {
            mask,
            inverted: selection.inverted,
        })
    } else {
        Ok(SavedSelection::Inline(selection.clone()))
    }
}

fn load_selection<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    saved: SavedSelection,
    w: u32,
    h: u32,
    total: &mut u64,
) -> Result<Selection, String> {
    let selection = match saved {
        SavedSelection::Inline(selection) => selection,
        SavedSelection::Mask { mask, inverted } => {
            let img = DynamicImage::ImageRgba8(decode(&read_entry(zip, &mask, MAX_PIXELS * 2)?)?)
                .into_luma8();
            let mut selection =
                crate::selection::from_mask(img.width(), img.height(), img.into_raw());
            selection.inverted = inverted;
            selection
        }
    };
    crate::selection::validate(&selection, w, h)?;
    if let SelShape::Mask { data, .. } = &selection.shape {
        *total += data.len() as u64;
    }
    if *total > 512 * 1024 * 1024 {
        return Err("Les pixels et masques du projet dépassent 512 Mio".into());
    }
    Ok(selection)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedLayer {
    #[serde(default)]
    locks: crate::layer_tools::Locks,
    id: u64,
    name: String,
    visible: bool,
    opacity: f32,
    blend: BlendMode,
    #[serde(default)]
    effects: crate::effects::LayerEffects,
    x: i32,
    y: i32,
    image: Option<String>,
    shapes: Option<Vec<Shape>>,
}

pub fn project_bytes(doc: &Document) -> Result<Vec<u8>, String> {
    project_budget(doc, 512 * 1024 * 1024)?;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut layers = Vec::new();
    for (i, l) in doc.layers.iter().enumerate() {
        let (image, shapes) = match &l.content {
            Content::Raster(img) => {
                let name = format!("layers/{i}.png");
                zip.start_file(&name, opts).map_err(|e| e.to_string())?;
                let bytes = if let Some(indexed) = &doc.indexed {
                    indexed.png(doc.width, doc.height)?
                } else {
                    png_bytes(img)?
                };
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
                (Some(name), None)
            }
            Content::Vector(shapes) => {
                crate::groups::validate(shapes)?;
                (None, Some(shapes.clone()))
            }
        };
        layers.push(SavedLayer {
            locks: l.locks,
            id: l.id,
            name: l.name.clone(),
            visible: l.visible,
            opacity: l.opacity,
            blend: l.blend,
            effects: l.effects.clone(),
            x: l.x,
            y: l.y,
            image,
            shapes,
        });
    }
    let selection = doc
        .selection
        .as_ref()
        .map(|s| save_selection(&mut zip, s, 0))
        .transpose()?;
    let mut selections = std::collections::BTreeMap::new();
    for (i, (name, selection)) in doc.selections.iter().enumerate() {
        selections.insert(name.clone(), save_selection(&mut zip, selection, i + 1)?);
    }
    let mut animation = None;
    if let Some(a) = &doc.animation {
        let mut frames = Vec::new();
        for (i, frame) in a.frames.iter().enumerate() {
            let image = format!("animation/{i}.png");
            zip.start_file(&image, opts).map_err(|e| e.to_string())?;
            zip.write_all(&png_bytes(&frame.image)?)
                .map_err(|e| e.to_string())?;
            frames.push(SavedFrame {
                image,
                delay_ms: frame.delay_ms,
            });
        }
        animation = Some(SavedAnimation {
            frames,
            current: a.current,
            loops: a.loops,
        });
    }
    let exif = if !doc.metadata.exif.is_empty() {
        zip.start_file("metadata/exif.tiff", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(&doc.metadata.exported_exif(doc.width, doc.height))
            .map_err(|e| e.to_string())?;
        Some("metadata/exif.tiff".into())
    } else {
        None
    };
    let project = Project {
        text_styles: doc.text_styles.clone(),
        version: PROJECT_VERSION,
        width: doc.width,
        height: doc.height,
        name: doc.name.clone(),
        selection,
        selections,
        layers,
        indexed: doc.indexed.as_ref().map(|_| "layers/0.png".into()),
        animation,
        exif,
        notices: doc.metadata.notices.clone(),
    };
    zip.start_file("document.json", opts)
        .map_err(|e| e.to_string())?;
    let metadata = serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err("Les métadonnées vectorielles dépassent 16 Mio ; simplifiez le projet avant de l'enregistrer".into());
    }
    zip.write_all(&metadata).map_err(|e| e.to_string())?;
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

fn project_budget(doc: &Document, limit: u64) -> Result<(), String> {
    crate::text_styles::validate(&doc.text_styles)?;
    dimensions(doc.width, doc.height)?;
    if doc.metadata.exif.len() > crate::metadata::MAX_EXIF {
        return Err("EXIF supérieur à 1 Mio".into());
    }
    if doc.layers.is_empty() || doc.layers.len() > 256 || doc.selections.len() > 32 {
        return Err("Nombre de calques ou sélections invalide".into());
    }
    let mut total = 0u64;
    crate::indexed::validate_document(doc)?;
    if let Some(indexed) = &doc.indexed {
        total += indexed.indices.len() as u64;
    }
    if let Some(animation) = &doc.animation {
        animation.validate(doc.width, doc.height)?;
        for frame in animation.frames.iter() {
            total += frame.image.as_raw().len() as u64;
        }
    }
    for layer in &doc.layers {
        crate::vector::style_dimensions(&layer.effects, doc.width, doc.height)?;
        if let Content::Raster(img) = &layer.content {
            total = total.saturating_add(img.as_raw().len() as u64);
        }
    }
    for selection in doc.selection.iter().chain(doc.selections.values()) {
        crate::selection::validate(selection, doc.width, doc.height)?;
        if let SelShape::Mask { data, .. } = &selection.shape {
            total = total.saturating_add(data.len() as u64);
        }
    }
    if total > limit {
        return Err("Les pixels et masques dépassent la limite de chargement (512 Mio) ; réduisez le projet avant de l'enregistrer".into());
    }
    Ok(())
}

fn read_entry<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    let file = zip.by_name(name).map_err(|e| e.to_string())?;
    if file.size() > limit {
        return Err(format!("Entrée trop volumineuse : {name}"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Projet trop volumineux".into());
    }
    Ok(bytes)
}

pub fn read_project(bytes: &[u8]) -> Result<Document, String> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Projet invalide : {e}"))?;
    let p: Project =
        serde_json::from_slice(&read_entry(&mut zip, "document.json", 16 * 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if !(1..=PROJECT_VERSION).contains(&p.version) {
        return Err(format!(
            "Version de projet non prise en charge : {}",
            p.version
        ));
    }
    dimensions(p.width, p.height)?;
    if p.selections.len() > 32 {
        return Err("Trop de sélections mémorisées".into());
    }
    let mut total = 0;
    let selection = p
        .selection
        .map(|s| load_selection(&mut zip, s, p.width, p.height, &mut total))
        .transpose()?;
    let mut selections = std::collections::BTreeMap::new();
    for (name, saved) in p.selections {
        selections.insert(
            name,
            load_selection(&mut zip, saved, p.width, p.height, &mut total)?,
        );
    }
    if p.layers.is_empty() || p.layers.len() > 256 {
        return Err("Le projet doit contenir 1 à 256 calques".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut layers = Vec::new();
    for l in p.layers {
        crate::vector::style_dimensions(&l.effects, p.width, p.height)?;
        if !ids.insert(l.id) {
            return Err("Identifiants de calques dupliqués".into());
        }
        let content = match (l.image, l.shapes) {
            (Some(name), None) => {
                let img = decode(&read_entry(&mut zip, &name, MAX_PIXELS * 5)?)?;
                total += img.as_raw().len() as u64;
                Content::Raster(Arc::new(img))
            }
            (None, Some(mut shapes)) => {
                if p.version < 7 {
                    crate::groups::repair_legacy_ids(&mut shapes)?;
                }
                crate::groups::validate(&shapes)?;
                crate::text_path::refresh(&mut shapes)?;
                for s in &shapes {
                    validate_shape(s)?;
                }
                Content::Vector(shapes)
            }
            _ => return Err("Contenu de calque invalide".into()),
        };
        if total > 512 * 1024 * 1024 {
            return Err("Les pixels du projet dépassent 512 Mio".into());
        }
        layers.push(Layer {
            locks: l.locks,
            id: l.id,
            name: l.name,
            visible: l.visible,
            opacity: l.opacity.clamp(0.0, 1.0),
            blend: l.blend,
            effects: l.effects,
            x: l.x,
            y: l.y,
            version: l.id,
            content,
        });
    }
    let indexed = if let Some(name) = p.indexed {
        let (w, h, indexed) =
            crate::indexed::decode_png(&read_entry(&mut zip, &name, MAX_PIXELS * 5)?)?
                .ok_or("Le projet déclare une palette mais le PNG n'est pas indexé")?;
        if (w, h) != (p.width, p.height) {
            return Err("Dimensions indexées incohérentes".into());
        }
        total += indexed.indices.len() as u64;
        Some(indexed)
    } else {
        None
    };
    let animation = if let Some(a) = p.animation {
        if !(2..=1000).contains(&a.frames.len()) {
            return Err("Nombre d'images d'animation invalide".into());
        }
        let mut frames = Vec::new();
        for f in a.frames {
            let img = decode(&read_entry(&mut zip, &f.image, MAX_PIXELS * 5)?)?;
            total += img.as_raw().len() as u64;
            if total > 512 * 1024 * 1024 {
                return Err("Projet trop grand (512 Mio maximum)".into());
            }
            frames.push(crate::animation::AnimationFrame {
                image: Arc::new(img),
                delay_ms: f.delay_ms,
            });
        }
        let animation = crate::animation::Animation {
            id: 0,
            frames: Arc::new(frames),
            current: a.current,
            loops: if p.version < 5 && a.loops > 0 {
                a.loops.saturating_add(1)
            } else {
                a.loops
            },
        };
        animation.validate(p.width, p.height)?;
        if layers.len() != 1
            || layers[0].x != 0
            || layers[0].y != 0
            || !layers[0].effects.is_empty()
        {
            return Err("Calque d'animation invalide".into());
        }
        let current = animation.frames[animation.current].image.clone();
        if layers[0]
            .raster()
            .is_none_or(|img| img.as_raw() != current.as_raw())
        {
            return Err("L'image courante ne correspond pas à l'animation".into());
        }
        layers[0].content = Content::Raster(current);
        Some(animation)
    } else {
        None
    };
    if total > 512 * 1024 * 1024 {
        return Err("Projet trop grand (512 Mio maximum)".into());
    }
    let doc = Document {
        text_styles: p.text_styles,
        width: p.width,
        height: p.height,
        name: p.name,
        selection,
        selections,
        layers,
        indexed,
        animation,
        metadata: crate::metadata::Metadata {
            exif: Arc::new(
                p.exif
                    .map(|name| read_entry(&mut zip, &name, crate::metadata::MAX_EXIF as u64))
                    .transpose()?
                    .unwrap_or_default(),
            ),
            notices: p.notices,
        },
    };
    crate::indexed::validate_document(&doc)?;
    crate::text_styles::validate(&doc.text_styles)?;
    Ok(doc)
}

pub fn validate_shape(s: &Shape) -> Result<(), String> {
    validate_shape_fields(s)
}
pub fn validate_shape_fields(s: &Shape) -> Result<(), String> {
    crate::fonts::validate(s)?;
    if let Some(p) = &s.text_path {
        p.validate(s)?;
    }
    if s.svg_unclipped && !s.svg_viewport {
        return Err("Le débordement SVG est réservé à un cadre SVG".into());
    }
    if s.svg_viewport && !matches!(s.kind, ShapeKind::Group { .. }) {
        return Err("Le cadre SVG est réservé aux groupes vectoriels".into());
    }
    if let Some(filters) = &s.filters {
        if !matches!(s.kind, ShapeKind::Group { .. }) {
            return Err("Les filtres s’appliquent à un groupe vectoriel".into());
        }
        filters.validate()?;
    }
    if let Some(mask) = &s.mask {
        if !matches!(s.kind, ShapeKind::Group { .. }) {
            return Err("Le masque s’applique à un groupe vectoriel".into());
        }
        mask.validate()?;
    }
    if let Some(clip) = &s.clip {
        if !matches!(s.kind, ShapeKind::Group { .. }) {
            return Err("La découpe s’applique à un groupe".into());
        }
        clip.validate()?;
    }
    if let Some(style) = &s.group_style {
        style.validate()?;
        if !matches!(s.kind, ShapeKind::Group { .. }) {
            return Err("Les styles de groupe sont réservés aux groupes vectoriels".into());
        }
    }
    if (s.fill_pattern.is_some() && s.fill_gradient.is_some())
        || (s.stroke_pattern.is_some() && s.stroke_gradient.is_some())
    {
        return Err("Une cible ne peut pas recevoir un motif et un dégradé simultanément".into());
    }
    for pattern in [s.fill_pattern.as_ref(), s.stroke_pattern.as_ref()]
        .into_iter()
        .flatten()
    {
        pattern.validate()?;
    }
    for gradient in [s.fill_gradient.as_ref(), s.stroke_gradient.as_ref()]
        .into_iter()
        .flatten()
    {
        gradient.validate()?;
    }
    if (s.fill_gradient.is_some()
        || s.stroke_gradient.is_some()
        || s.fill_pattern.is_some()
        || s.stroke_pattern.is_some())
        && matches!(s.kind, ShapeKind::Text { .. })
        && (s.w.abs() < 1e-6 || s.h.abs() < 1e-6)
    {
        return Err("Dimensions du texte trop petites pour le dégradé".into());
    }
    if matches!(
        s.kind,
        ShapeKind::Group { .. } | ShapeKind::Svg { .. } | ShapeKind::Image { .. }
    ) && (s.fill_gradient.is_some()
        || s.stroke_gradient.is_some()
        || s.fill_pattern.is_some()
        || s.stroke_pattern.is_some())
    {
        return Err("Appliquez le dégradé à une forme, pas à un groupe ou bloc SVG".into());
    }
    if let Some(m) = s.transform {
        if m.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
            || (m[0] * m[3] - m[1] * m[2]).abs() < 1e-10
        {
            return Err("Transformation vectorielle invalide ou non inversible".into());
        }
    }
    if [s.x, s.y, s.w, s.h, s.rotation, s.stroke_width, s.opacity]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err("Coordonnées vectorielles invalides".into());
    }
    if s.stroke_width < 0.0 || !(0.0..=1.0).contains(&s.opacity) {
        return Err("Style vectoriel invalide".into());
    }
    match &s.kind {
        ShapeKind::Image { asset } => {
            asset.validate()?;
            if s.w <= 0.
                || s.h <= 0.
                || s.fill.is_some()
                || s.stroke.is_some()
                || s.stroke_width != 0.
            {
                return Err("Dimensions ou style d’image intégrée invalides".into());
            }
        }
        ShapeKind::Path { contours, .. } => crate::paths::validate(contours)?,
        ShapeKind::Svg { source, .. } => {
            crate::vector::parse(source)?;
        }
        ShapeKind::Text {
            font_size, text, ..
        } if !font_size.is_finite()
            || *font_size <= 0.0
            || *font_size > 4096.0
            || text.len() > 100_000 =>
        {
            return Err("Texte invalide".into())
        }
        _ => (),
    }
    Ok(())
}

pub fn import(path: &str) -> Result<Document, String> {
    let path = Path::new(path);
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 512 * 1024 * 1024 {
        return Err("Fichier supérieur à 512 Mio".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "prisme" {
        return read_project(&bytes);
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if ext == "psd" {
        return crate::psd::read(&bytes, &name);
    }
    if ext == "pfv" {
        return crate::pfv::document(&bytes, &name);
    }
    let mut indexed = None;
    let mut animation = None;
    let mut metadata = crate::metadata::Metadata::default();
    let (w, h, content) = if ext == "svg" {
        let source = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        metadata
            .notices
            .extend(crate::embedded::diagnostics(&source)?);
        let tree = crate::vector::parse(&source)?;
        let size = tree.size();
        let (w, h) = (size.width().ceil() as u32, size.height().ceil() as u32);
        dimensions(w, h)?;
        (
            w,
            h,
            Content::Vector(vec![Shape {
                font_options: None,
                text_path: None,
                svg_unclipped: false,
                svg_viewport: false,
                filters: None,
                mask: None,
                clip: None,
                group_style: None,
                fill_pattern: None,
                stroke_pattern: None,
                fill_gradient: None,
                stroke_gradient: None,
                transform: None,
                id: 2,
                kind: ShapeKind::Svg {
                    source,
                    intrinsic_w: size.width(),
                    intrinsic_h: size.height(),
                },
                x: 0.0,
                y: 0.0,
                w: w as f32,
                h: h as f32,
                rotation: 0.0,
                fill: None,
                stroke: None,
                stroke_width: 0.0,
                opacity: 1.0,
            }]),
        )
    } else if ext == "avif" {
        let (img, meta) = crate::formats::avif(&bytes)?;
        metadata = meta;
        (img.width(), img.height(), Content::Raster(Arc::new(img)))
    } else if ext == "heic" || ext == "heif" {
        let (img, mut meta) = crate::formats::wic(path.to_str().ok_or("Chemin non UTF-8")?, false)?;
        meta.exif = Arc::new(crate::formats::iso_exif(&bytes).unwrap_or_default());
        let img = crate::formats::orient_iso(img, &bytes, &mut meta);
        metadata = meta;
        (img.width(), img.height(), Content::Raster(Arc::new(img)))
    } else if crate::formats::is_raw(&ext) {
        let (img, meta) =
            crate::formats::raw_preview(&bytes, path.to_str().ok_or("Chemin non UTF-8")?)?;
        metadata = meta;
        (img.width(), img.height(), Content::Raster(Arc::new(img)))
    } else if ext == "gif" {
        let (first, a) = crate::animation::decode_gif(&bytes)?;
        animation = a;
        (first.width(), first.height(), Content::Raster(first))
    } else if ext == "png" {
        if let Some((w, h, state)) = crate::indexed::decode_png(&bytes)? {
            let exif = crate::metadata::read_exif(&bytes);
            let orientation =
                image::metadata::Orientation::from_exif_chunk(&exif).map_or(1, |o| o.to_exif());
            let (mut state, mut w, mut h) = (state, w, h);
            let ops = match orientation {
                2 => vec![crate::operations::Operation::Flip { horizontal: true }],
                3 => vec![crate::operations::Operation::Rotate { degrees: 180.0 }],
                4 => vec![crate::operations::Operation::Flip { horizontal: false }],
                5 => vec![
                    crate::operations::Operation::Rotate { degrees: 90.0 },
                    crate::operations::Operation::Flip { horizontal: true },
                ],
                6 => vec![crate::operations::Operation::Rotate { degrees: 90.0 }],
                7 => vec![
                    crate::operations::Operation::Rotate { degrees: 270.0 },
                    crate::operations::Operation::Flip { horizontal: true },
                ],
                8 => vec![crate::operations::Operation::Rotate { degrees: 270.0 }],
                _ => vec![],
            };
            for op in ops {
                let next = crate::indexed::transform(&state, w, h, None, &op)?
                    .ok_or("Orientation indexée invalide")?;
                state = next.0;
                w = next.1;
                h = next.2;
            }
            let img = state.expand(w, h);
            indexed = Some(state);
            metadata.exif = Arc::new(exif);
            metadata.exif = Arc::new(metadata.exported_exif(w, h));
            (w, h, Content::Raster(Arc::new(img)))
        } else {
            let (img, meta) = crate::metadata::decode(&bytes)?;
            metadata = meta;
            (img.width(), img.height(), Content::Raster(Arc::new(img)))
        }
    } else {
        let (img, meta) = crate::metadata::decode(&bytes)?;
        metadata = meta;
        (img.width(), img.height(), Content::Raster(Arc::new(img)))
    };
    Ok(Document {
        text_styles: Vec::new(),
        width: w,
        height: h,
        name: name.clone(),
        selection: None,
        selections: Default::default(),
        metadata,
        indexed,
        animation,
        layers: vec![Layer {
            locks: Default::default(),
            id: 1,
            name,
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            effects: Default::default(),
            x: 0,
            y: 0,
            version: 3,
            content,
        }],
    })
}

/// Le fichier précédent reste intact si l'encodage ou l'écriture échoue.
pub fn atomic_write(path: &str, bytes: &[u8]) -> Result<(), String> {
    let path = Path::new(path);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = parent.join(format!(
        ".prisme-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn export(doc: &Document, path: &str, quality: u8) -> Result<(), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    atomic_write(path, &export_bytes(doc, &ext, quality)?)
}

pub fn export_bytes(doc: &Document, ext: &str, quality: u8) -> Result<Vec<u8>, String> {
    if ext == "gif" {
        return crate::animation::encode_gif(doc);
    }
    let exif = doc.metadata.exported_exif(doc.width, doc.height);
    if ext == "pdf" {
        return crate::vector_export::pdf(doc);
    }
    if ext == "svg" {
        return Ok(crate::vector::export_svg(doc)?.into_bytes());
    }
    if ext == "png" {
        if let Some(indexed) = &doc.indexed {
            crate::indexed::validate_document(doc)?;
            return indexed.png_metadata(doc.width, doc.height, &exif);
        }
    }
    let img = crate::vector::flatten(doc)?;
    let mut cursor = Cursor::new(Vec::new());
    match ext {
        "jpg" | "jpeg" => {
            let mut white = RgbaImage::from_pixel(doc.width, doc.height, image::Rgba([255; 4]));
            crate::raster::composite(&mut white, &img, 0, 0, 1.0, BlendMode::Normal);
            let rgb = DynamicImage::ImageRgba8(white).into_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut cursor,
                quality.clamp(1, 100),
            );
            if !exif.is_empty() {
                encoder.set_exif_metadata(exif).map_err(|e| e.to_string())?;
            }
            encoder.encode_image(&rgb).map_err(|e| e.to_string())?;
        }
        "png" => {
            let mut encoder = image::codecs::png::PngEncoder::new(&mut cursor);
            if !exif.is_empty() {
                encoder.set_exif_metadata(exif).map_err(|e| e.to_string())?;
            }
            encoder
                .write_image(
                    img.as_raw(),
                    doc.width,
                    doc.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?;
        }
        "webp" => {
            let mut encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut cursor);
            if !exif.is_empty() {
                encoder.set_exif_metadata(exif).map_err(|e| e.to_string())?;
            }
            encoder
                .write_image(
                    img.as_raw(),
                    doc.width,
                    doc.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?;
        }
        "avif" => {
            let mut encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
                &mut cursor,
                8,
                quality.clamp(1, 100),
            );
            if !exif.is_empty() {
                encoder.set_exif_metadata(exif).map_err(|e| e.to_string())?;
            }
            encoder
                .write_image(
                    img.as_raw(),
                    doc.width,
                    doc.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?;
        }
        "bmp" | "tif" | "tiff" => {
            let format = ImageFormat::from_extension(&ext).ok_or("Format d'export inconnu")?;
            DynamicImage::ImageRgba8(img)
                .write_to(&mut cursor, format)
                .map_err(|e| e.to_string())?;
        }
        _ => return Err("Choisir PNG, JPEG, WebP, BMP, TIFF, AVIF ou SVG".into()),
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(e: &mut crate::document::Editor, value: serde_json::Value) -> Result<(), String> {
        crate::operations::execute(e, serde_json::from_value(value).unwrap())
    }
    #[test]
    fn indexed_palette_edits_transforms_transparency_and_png_are_persistent() {
        use crate::operations::{self, Operation};
        use serde_json::json;
        let mut e = crate::document::Editor::default();
        command(
            &mut e,
            json!({"type":"new","width":6,"height":4,"background":"#ffffff"}),
        )
        .unwrap();
        let id = e.doc().unwrap().layers[0].id;
        command(&mut e,json!({"type":"selection","selection":{"shape":{"type":"rect","x":0,"y":0,"w":3,"h":4},"inverted":false}})).unwrap();
        command(
            &mut e,
            json!({"type":"fill","id":id,"x":0,"y":0,"color":"#000000","tolerance":0}),
        )
        .unwrap();
        command(&mut e, json!({"type":"selection","selection":null})).unwrap();
        command(
            &mut e,
            json!({"type":"convertIndexed","colors":2,"dither":false}),
        )
        .unwrap();
        let left = e.doc().unwrap().indexed.as_ref().unwrap().indices[0] as usize;
        let right = e.doc().unwrap().indexed.as_ref().unwrap().indices[5] as usize;
        assert_ne!(left, right);
        command(
            &mut e,
            json!({"type":"paletteColor","index":left,"color":"#ff0000"}),
        )
        .unwrap();
        command(
            &mut e,
            json!({"type":"paletteColor","index":right,"color":"#ff0000"}),
        )
        .unwrap();
        command(
            &mut e,
            json!({"type":"paletteColor","index":right,"color":"#00ff00"}),
        )
        .unwrap();
        let doc = e.doc().unwrap();
        assert_eq!(
            doc.layers[0].raster().unwrap().get_pixel(0, 0).0,
            [255, 0, 0, 255]
        );
        assert_eq!(
            doc.layers[0].raster().unwrap().get_pixel(5, 0).0,
            [0, 255, 0, 255]
        );
        let restored = read_project(&project_bytes(doc).unwrap()).unwrap();
        assert_eq!(
            restored.indexed.as_ref().unwrap().indices,
            doc.indexed.as_ref().unwrap().indices
        );
        assert_eq!(
            restored.indexed.as_ref().unwrap().palette,
            doc.indexed.as_ref().unwrap().palette
        );
        assert!(operations::execute(
            &mut e,
            Operation::AddLayer {
                kind: "raster".into()
            }
        )
        .is_err());
        command(&mut e, json!({"type":"flip","horizontal":true})).unwrap();
        assert_eq!(
            e.doc().unwrap().indexed.as_ref().unwrap().indices[0] as usize,
            right
        );
        let id = e.doc().unwrap().layers[0].id;
        command(
            &mut e,
            json!({"type":"transparency","id":id,"color":"#ff0000","tolerance":0}),
        )
        .unwrap();
        assert_eq!(
            e.doc().unwrap().layers[0].raster().unwrap().get_pixel(5, 0)[3],
            0
        );
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../work/fixture-indexed.png");
        export(e.doc().unwrap(), path.to_str().unwrap(), 90).unwrap();
        let imported = import(path.to_str().unwrap()).unwrap();
        assert!(imported.indexed.is_some());
        assert_eq!(
            imported.indexed.as_ref().unwrap().indices,
            e.doc().unwrap().indexed.as_ref().unwrap().indices
        );
        command(&mut e, json!({"type":"convertRgb"})).unwrap();
        assert!(e.doc().unwrap().indexed.is_none());
        e.undo().unwrap();
        assert!(e.doc().unwrap().indexed.is_some());
    }
    #[test]
    fn gif_frames_survive_project_and_read_only_mode_until_extraction() {
        use crate::operations::Operation;
        let work = Path::new(env!("CARGO_MANIFEST_DIR")).join("../work");
        fs::create_dir_all(&work).unwrap();
        let path = work.join("fixture-animation-project.gif");
        fs::write(&path, crate::animation::tests::fixture()).unwrap();
        let doc = import(path.to_str().unwrap()).unwrap();
        let mut e = crate::document::Editor::default();
        e.load(doc, None, "Ouvrir");
        crate::operations::execute(&mut e, Operation::AnimationFrame { index: 1 }).unwrap();
        assert!(!e.dirty);
        assert_eq!(
            e.doc().unwrap().layers[0]
                .raster()
                .unwrap()
                .get_pixel(1, 1)
                .0,
            [0, 255, 0, 255]
        );
        let id = e.doc().unwrap().layers[0].id;
        assert!(command(
            &mut e,
            serde_json::json!({"type":"filter","id":id,"name":"invert","params":{}})
        )
        .is_err());
        let restored = read_project(&project_bytes(e.doc().unwrap()).unwrap()).unwrap();
        let a = restored.animation.as_ref().unwrap();
        assert_eq!(a.current, 1);
        assert_eq!(a.frames.len(), 3);
        assert_eq!(a.loops, 3);
        assert_eq!(a.info().delays, vec![50, 80, 120]);
        crate::operations::execute(&mut e, Operation::ExtractFrame { index: 2 }).unwrap();
        assert!(e.doc().unwrap().animation.is_none());
        assert_eq!(
            e.doc().unwrap().layers[0]
                .raster()
                .unwrap()
                .get_pixel(2, 1)
                .0,
            [0, 0, 255, 255]
        );
        e.undo().unwrap();
        assert!(e.doc().unwrap().animation.is_some());
    }
    #[test]
    fn saving_checks_the_same_logical_pixel_budget_as_reopening() {
        let mut editor = crate::document::Editor::default();
        crate::operations::execute(
            &mut editor,
            serde_json::from_value(
                serde_json::json!({"type":"new","width":3,"height":2,"background":null}),
            )
            .unwrap(),
        )
        .unwrap();
        let doc = editor.doc.as_mut().unwrap();
        assert!(project_budget(doc, 24).is_ok());
        doc.selections.insert(
            "Masque".into(),
            crate::selection::from_mask(3, 2, vec![255; 6]),
        );
        assert!(project_budget(doc, 24).is_err());
        assert!(project_budget(doc, 30).is_ok());
        assert!(read_project(&project_bytes(doc).unwrap()).is_ok());
    }
    #[test]
    fn masks_and_named_selections_are_png_entries_and_round_trip_exactly() {
        let mut editor = crate::document::Editor::default();
        crate::operations::execute(
            &mut editor,
            serde_json::from_value(
                serde_json::json!({"type":"new","width":3,"height":2,"background":null}),
            )
            .unwrap(),
        )
        .unwrap();
        let mask = crate::selection::from_mask(3, 2, vec![0, 23, 128, 200, 254, 255]);
        let doc = editor.doc.as_mut().unwrap();
        doc.selection = Some(mask.clone());
        doc.selections.insert("Détail".into(), mask.clone());
        let bytes = project_bytes(doc).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let json =
            String::from_utf8(read_entry(&mut zip, "document.json", 10000).unwrap()).unwrap();
        assert!(json.contains("selections/0.png"));
        assert!(!json.contains("\"data\""));
        let restored = read_project(&bytes).unwrap();
        assert_eq!(restored.selection.unwrap().mask(3, 2), mask.mask(3, 2));
        assert_eq!(restored.selections["Détail"].mask(3, 2), mask.mask(3, 2));
    }
    #[test]
    fn project_round_trip_keeps_pixels_and_metadata() {
        let d = Document {
            text_styles: Vec::new(),
            width: 3,
            height: 2,
            name: "Été".into(),
            selection: None,
            selections: Default::default(),
            metadata: Default::default(),
            indexed: None,
            animation: None,
            layers: vec![Layer {
                locks: Default::default(),
                id: 7,
                name: "Fond".into(),
                visible: true,
                opacity: 0.6,
                blend: BlendMode::Multiply,
                effects: Default::default(),
                x: -1,
                y: 2,
                version: 13,
                content: Content::Raster(Arc::new(RgbaImage::from_pixel(
                    3,
                    2,
                    image::Rgba([30, 40, 50, 128]),
                ))),
            }],
        };
        let restored = read_project(&project_bytes(&d).unwrap()).unwrap();
        assert_eq!(restored.name, d.name);
        assert_eq!(restored.layers[0].x, -1);
        assert_eq!(restored.layers[0].opacity, 0.6);
        assert_eq!(
            restored.layers[0].raster().unwrap(),
            d.layers[0].raster().unwrap()
        );
    }
    #[test]
    fn rejects_invalid_dimensions_and_archives() {
        assert!(dimensions(0, 1).is_err());
        assert!(dimensions(16000, 16000).is_err());
        assert!(read_project(b"not a project").is_err());
    }
    #[test]
    fn exports_all_phase_one_formats_and_reopens_raster() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../work/export-tests");
        fs::create_dir_all(&folder).unwrap();
        let d = Document {
            text_styles: Vec::new(),
            width: 4,
            height: 3,
            name: "Export".into(),
            selection: None,
            selections: Default::default(),
            metadata: Default::default(),
            indexed: None,
            animation: None,
            layers: vec![Layer {
                locks: Default::default(),
                id: 1,
                name: "Fond".into(),
                visible: true,
                opacity: 1.0,
                blend: BlendMode::Normal,
                effects: Default::default(),
                x: 0,
                y: 0,
                version: 2,
                content: Content::Raster(Arc::new(RgbaImage::from_pixel(
                    4,
                    3,
                    image::Rgba([255, 0, 0, 128]),
                ))),
            }],
        };
        for ext in ["png", "jpg", "webp", "bmp", "tiff", "svg"] {
            let path = folder.join(format!("round-trip.{ext}"));
            let path = path.to_str().unwrap();
            export(&d, path, 92).unwrap();
            let reopened = import(path).unwrap();
            assert_eq!((reopened.width, reopened.height), (4, 3));
            if ext == "png" {
                assert_eq!(
                    reopened.layers[0].raster().unwrap().get_pixel(0, 0).0,
                    [255, 0, 0, 128]
                );
            }
            if ext == "jpg" {
                let px = reopened.layers[0].raster().unwrap().get_pixel(0, 0);
                assert!(px[0] > 245 && px[1] > 115 && px[1] < 140);
                assert_eq!(px[3], 255);
            }
            // Vérifie aussi le remplacement atomique d'un export existant sous Windows.
            export(&d, path, 80).unwrap();
            fs::remove_file(path).unwrap();
        }
    }
}
