//! Commandes transactionnelles partagées par l'interface et les tests.
use crate::{
    document::{BlendMode, Content, Document, Editor, Selection, Shape, ShapeKind},
    io, raster,
    selection::{self, Combine},
    vector,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Operation {
    LayerVisibility {
        states: std::collections::BTreeMap<String, bool>,
    },
    LayerBatch {
        ids: Vec<u64>,
        kind: String,
        value: Option<String>,
        locks: Option<crate::layer_tools::Locks>,
    },
    ImportPfv {
        path: String,
    },
    Vectorize {
        id: u64,
        revision: u64,
        settings: crate::vectorize::Settings,
        hide_source: bool,
    },
    TextStyleSave {
        id: u64,
        shape_id: u64,
        name: String,
    },
    TextStyleDelete {
        name: String,
    },
    TextStyleApply {
        id: u64,
        shape_ids: Vec<u64>,
        name: String,
    },
    TextStylesImport {
        path: String,
    },
    TextStylesExport {
        path: String,
    },
    New {
        width: u32,
        height: u32,
        background: Option<String>,
    },
    Open {
        path: String,
    },
    OpenSvgEditable {
        path: String,
    },
    Import {
        path: String,
    },
    Paste {
        png: String,
    },
    PasteClipboard {
        #[serde(default)]
        as_new: bool,
    },
    CopyClipboard,
    ImportSelection {
        path: String,
    },
    Save {
        path: String,
    },
    Export {
        path: String,
        quality: u8,
        #[serde(default)]
        optimized: bool,
        #[serde(default = "default_precision")]
        precision: u8,
    },
    Close,
    Undo,
    Redo,
    Jump {
        index: usize,
    },
    AddLayer {
        kind: String,
    },
    DeleteLayer {
        id: u64,
    },
    DuplicateLayer {
        id: u64,
    },
    Layer {
        id: u64,
        name: Option<String>,
        visible: Option<bool>,
        opacity: Option<f32>,
        blend: Option<BlendMode>,
        x: Option<i32>,
        y: Option<i32>,
    },
    Reorder {
        id: u64,
        index: usize,
    },
    MergeDown {
        id: u64,
    },
    MergePrefix {
        id: u64,
    },
    Flatten,
    Rasterize {
        id: u64,
    },
    Effects {
        id: u64,
        effects: crate::effects::LayerEffects,
    },
    ConvertIndexed {
        colors: usize,
        dither: bool,
    },
    ConvertRgb,
    PaletteColor {
        index: usize,
        color: String,
    },
    Transparency {
        id: u64,
        color: String,
        tolerance: u8,
    },
    AnimationFrame {
        index: usize,
    },
    ExtractFrame {
        index: usize,
    },
    Shape {
        id: u64,
        shape: Shape,
    },
    DeleteShape {
        id: u64,
        shape_id: u64,
    },
    GroupShapes {
        id: u64,
        shape_ids: Vec<u64>,
    },
    UngroupShapes {
        id: u64,
        shape_id: u64,
    },
    ToPath {
        id: u64,
        shape_id: u64,
    },
    DecomposeSvg {
        id: u64,
        shape_id: u64,
    },
    Boolean {
        id: u64,
        first: u64,
        second: u64,
        kind: String,
    },
    Stroke {
        id: u64,
        points: Vec<[f32; 2]>,
        size: f32,
        color: String,
        opacity: f32,
        erase: bool,
    },
    Retouch {
        id: u64,
        tool: String,
        points: Vec<[f32; 2]>,
        size: f32,
        strength: f32,
        source_offset: Option<[f32; 2]>,
        color: String,
        #[serde(default)]
        nozzle: Option<String>,
        nozzle_path: Option<String>,
    },
    Asset {
        id: u64,
        #[serde(flatten)]
        settings: crate::assets::Settings,
    },
    Fill {
        id: u64,
        x: i32,
        y: i32,
        color: String,
        tolerance: u8,
    },
    Gradient {
        id: u64,
        start: [f32; 2],
        end: [f32; 2],
        color: String,
        secondary: String,
    },
    Filter {
        id: u64,
        name: String,
        params: Value,
    },
    Plugin {
        id: u64,
        manifest: String,
        params: Value,
    },
    EightBf {
        id: u64,
        path: String,
        key: String,
        #[serde(default)]
        settings: crate::eightbf::Settings,
    },
    Pfl {
        #[serde(default)]
        id: Option<u64>,
        path: String,
        #[serde(default = "crate::pfl::default_timeout")]
        timeout_ms: u64,
        #[serde(skip)]
        parent_window: isize,
    },
    Selection {
        selection: Option<Selection>,
        #[serde(default)]
        mode: Combine,
    },
    SelectColor {
        x: i32,
        y: i32,
        tolerance: u8,
        contiguous: bool,
        #[serde(default)]
        mode: Combine,
    },
    ModifySelection {
        kind: String,
        radius: f32,
    },
    RememberSelection {
        name: String,
    },
    RestoreSelection {
        name: String,
    },
    ForgetSelection {
        name: String,
    },
    InvertSelection,
    Resize {
        width: u32,
        height: u32,
    },
    Canvas {
        width: u32,
        height: u32,
        offset_x: i32,
        offset_y: i32,
    },
    Crop,
    Rotate {
        degrees: f32,
    },
    Flip {
        horizontal: bool,
    },
}

fn default_precision() -> u8 {
    8
}
fn color(s: &str) -> Result<[u8; 4], String> {
    raster::parse_color(s).ok_or("Couleur hexadécimale invalide".into())
}
fn raster_layer(e: &Editor, id: u64) -> Result<(usize, RgbaImage, i32, i32), String> {
    let idx = e.layer_index(id)?;
    let layer = &e.doc()?.layers[idx];
    let img = layer.raster().ok_or(
        "Cet outil nécessite un calque bitmap. Rastérisez le calque ou ajoutez un calque bitmap.",
    )?;
    Ok((idx, (**img).clone(), layer.x, layer.y))
}
fn set_pixels(
    e: &mut Editor,
    idx: usize,
    mut img: RgbaImage,
    label: &str,
    masked: bool,
) -> Result<(), String> {
    if masked {
        let doc = e.doc()?;
        let layer = &doc.layers[idx];
        if let Some(selection) = &doc.selection {
            raster::apply_mask(
                layer.raster().ok_or("Calque bitmap requis")?,
                &mut img,
                &selection.mask(doc.width, doc.height),
                doc.width,
                doc.height,
                layer.x,
                layer.y,
            );
        }
    }
    if let Some(indexed) = &e.doc()?.indexed {
        let layer = &e.doc()?.layers[idx];
        let before = layer.raster().ok_or("Bitmap indexé requis")?;
        let indexed = indexed.map_edit(before, &img)?;
        img = indexed.expand(img.width(), img.height());
        e.doc_mut()?.indexed = Some(indexed);
    }
    if e.doc()?.layers[idx].locks.alpha {
        let before = e.doc()?.layers[idx]
            .raster()
            .ok_or("Verrou alpha réservé au bitmap")?;
        if before.dimensions() != img.dimensions() {
            return Err("Déverrouillez l’alpha avant de changer les dimensions".into());
        }
        for (a, b) in before.pixels().zip(img.pixels_mut()) {
            if a[3] == 0 || b[3] == 0 {
                b[0] = a[0];
                b[1] = a[1];
                b[2] = a[2];
            }
            b[3] = a[3];
        }
    }
    e.doc_mut()?.layers[idx].content = Content::Raster(Arc::new(img));
    e.touch(idx);
    e.commit(label);
    Ok(())
}

pub fn execute(e: &mut Editor, op: Operation) -> Result<(), String> {
    execute_captured(e, op).map(|_| ())
}

/// Renvoie les paramètres réellement produits par un dialogue de filtre.
pub fn execute_captured(e: &mut Editor, op: Operation) -> Result<Option<Operation>, String> {
    // A linked guide can invalidate a command after geometry changes. Stage
    // history and document together so a refusal never leaves a committed step.
    let mut staged = e.clone();
    let mut captured = None;
    execute_inner(&mut staged, op, &mut captured)?;
    staged.refresh_text_paths()?;
    *e = staged;
    Ok(captured)
}

fn execute_inner(
    e: &mut Editor,
    op: Operation,
    captured: &mut Option<Operation>,
) -> Result<(), String> {
    if e.doc.is_some() {
        crate::layer_tools::guard(e, &op)?;
    }
    if e.doc.as_ref().is_some_and(|d| d.animation.is_some())
        && !matches!(
            &op,
            Operation::New { .. }
                | Operation::Open { .. }
                | Operation::OpenSvgEditable { .. }
                | Operation::Close
                | Operation::Save { .. }
                | Operation::Export { .. }
                | Operation::Undo
                | Operation::Redo
                | Operation::Jump { .. }
                | Operation::Selection { .. }
                | Operation::SelectColor { .. }
                | Operation::ModifySelection { .. }
                | Operation::RememberSelection { .. }
                | Operation::RestoreSelection { .. }
                | Operation::ForgetSelection { .. }
                | Operation::InvertSelection
                | Operation::AnimationFrame { .. }
                | Operation::ExtractFrame { .. }
                | Operation::CopyClipboard
                | Operation::PasteClipboard { as_new: true }
        )
    {
        return Err("Ce GIF est en lecture. Choisissez « Éditer cette image » pour retoucher une image fixe.".into());
    }
    if let Some(indexed) = e.doc.as_ref().and_then(|d| d.indexed.as_ref()) {
        let doc = e.doc()?;
        if let Some((indexed, w, h, label)) =
            crate::indexed::transform(indexed, doc.width, doc.height, doc.selection.as_ref(), &op)?
        {
            let pixels = indexed.expand(w, h);
            let doc = e.doc_mut()?;
            doc.width = w;
            doc.height = h;
            doc.selection = None;
            doc.layers[0].content = Content::Raster(Arc::new(pixels));
            doc.indexed = Some(indexed);
            e.touch(0);
            e.commit(label);
            return Ok(());
        }
        let incompatible = match &op {
            Operation::Vectorize { .. }
            | Operation::AddLayer { .. }
            | Operation::DeleteLayer { .. }
            | Operation::DuplicateLayer { .. }
            | Operation::Reorder { .. }
            | Operation::MergeDown { .. }
            | Operation::MergePrefix { .. }
            | Operation::Flatten
            | Operation::Import { .. }
            | Operation::Paste { .. }
            | Operation::Shape { .. }
            | Operation::DeleteShape { .. } => true,
            Operation::PasteClipboard { as_new } => !as_new,
            Operation::Effects { effects, .. } => !effects.is_empty(),
            Operation::Layer {
                visible,
                opacity,
                blend,
                x,
                y,
                ..
            } => {
                visible.is_some_and(|v| !v)
                    || opacity.is_some_and(|v| v != 1.0)
                    || blend.is_some_and(|v| v != BlendMode::Normal)
                    || x.is_some_and(|v| v != 0)
                    || y.is_some_and(|v| v != 0)
            }
            _ => false,
        };
        if incompatible {
            return Err(
                "Cette opération nécessite le mode RVB. Utilisez Image > Passer en RVB.".into(),
            );
        }
    }
    match op {
        Operation::LayerVisibility { states } => {
            if states.len() != e.doc()?.layers.len()
                || e.doc()?
                    .layers
                    .iter()
                    .any(|l| !states.contains_key(&l.id.to_string()))
            {
                return Err("La liste des calques a changé ; restaurez avec Annuler".into());
            }
            for l in &mut e.doc_mut()?.layers {
                l.visible = states[&l.id.to_string()];
            }
            e.commit("Isoler ou restaurer les calques");
        }
        Operation::LayerBatch {
            ids,
            kind,
            value,
            locks,
        } => crate::layer_tools::batch(e, ids, kind, value, locks)?,
        Operation::ImportPfv { path } => crate::pfv::import(e, &path)?,
        Operation::Vectorize {
            id,
            revision,
            settings,
            hide_source,
        } => {
            crate::vectorize::apply(e, id, revision, settings, hide_source)?;
        }
        Operation::ConvertIndexed { colors, dither } => {
            let doc = e.doc()?;
            let image = vector::flatten(doc)?;
            let indexed = crate::indexed::quantize(&image, colors, dither)?;
            let pixels = indexed.expand(doc.width, doc.height);
            let layer = e.new_raster_layer("Image indexée", pixels, 0, 0);
            e.active_hint = Some(layer.id);
            let doc = e.doc_mut()?;
            doc.layers = vec![layer];
            doc.indexed = Some(indexed);
            e.commit("Convertir en couleurs indexées");
        }
        Operation::ConvertRgb => {
            if e.doc_mut()?.indexed.take().is_some() {
                e.commit("Passer en RVB");
            }
        }
        Operation::PaletteColor { index, color: ink } => {
            let rgba = color(&ink)?;
            let doc = e.doc_mut()?;
            let indexed = doc.indexed.as_mut().ok_or("Le document n'est pas indexé")?;
            indexed.replace_color(index, rgba)?;
            doc.layers[0].content =
                Content::Raster(Arc::new(indexed.expand(doc.width, doc.height)));
            e.touch(0);
            e.commit("Modifier la palette");
        }
        Operation::Transparency {
            id,
            color: ink,
            tolerance,
        } => {
            let rgba = color(&ink)?;
            if let Some(indexed) = e.doc_mut()?.indexed.as_mut() {
                for color in &mut indexed.palette {
                    if (0..3).all(|c| color[c].abs_diff(rgba[c]) <= tolerance) {
                        color[3] = 0;
                    }
                }
                let doc = e.doc_mut()?;
                doc.layers[0].content = Content::Raster(Arc::new(
                    doc.indexed.as_ref().unwrap().expand(doc.width, doc.height),
                ));
                e.touch(0);
                e.commit("Transparence par couleur");
            } else {
                let (idx, mut img, _, _) = raster_layer(e, id)?;
                for pixel in img.pixels_mut() {
                    if (0..3).all(|c| pixel[c].abs_diff(rgba[c]) <= tolerance) {
                        pixel[3] = 0;
                    }
                }
                set_pixels(e, idx, img, "Transparence par couleur", true)?;
            }
        }
        Operation::AnimationFrame { index } => {
            select_animation_frame(e, index)?;
        }
        Operation::ExtractFrame { index } => {
            select_animation_frame(e, index)?;
            e.doc_mut()?.animation = None;
            e.commit("Éditer une image du GIF");
        }
        Operation::TextStyleSave { id, shape_id, name } => {
            crate::text_styles::save(e.doc_mut()?, id, shape_id, &name)?;
            e.commit("Enregistrer un style de texte");
        }
        Operation::TextStyleDelete { name } => {
            let styles = &mut e.doc_mut()?.text_styles;
            let before = styles.len();
            styles.retain(|s| s.name != name);
            if before == styles.len() {
                return Err("Style de texte introuvable".into());
            }
            e.commit("Supprimer un style de texte");
        }
        Operation::TextStyleApply {
            id,
            shape_ids,
            name,
        } => {
            if shape_ids.is_empty()
                || shape_ids.len() > 20_000
                || shape_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != shape_ids.len()
            {
                return Err("Choisissez des textes distincts pour appliquer le style".into());
            }
            let style = e
                .doc()?
                .text_styles
                .iter()
                .find(|s| s.name == name)
                .ok_or("Style de texte introuvable")?
                .clone();
            let idx = e.layer_index(id)?;
            let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            for sid in shape_ids {
                style.apply(crate::groups::find_mut(shapes, sid).ok_or("Texte introuvable")?)?;
            }
            e.touch(idx);
            e.commit("Appliquer un style de texte");
        }
        Operation::TextStylesImport { path } => {
            let styles = crate::text_styles::read(&path)?;
            crate::text_styles::merge(&mut e.doc_mut()?.text_styles, styles)?;
            e.commit("Importer des styles de texte");
        }
        Operation::TextStylesExport { path } => {
            crate::text_styles::write(&path, &e.doc()?.text_styles)?;
        }
        Operation::New {
            width,
            height,
            background,
        } => {
            io::dimensions(width, height)?;
            let fill = background
                .as_deref()
                .map(color)
                .transpose()?
                .unwrap_or([0; 4]);
            let layer = e.new_raster_layer(
                "Arrière-plan",
                RgbaImage::from_pixel(width, height, Rgba(fill)),
                0,
                0,
            );
            e.load(
                Document {
                    text_styles: Vec::new(),
                    width,
                    height,
                    layers: vec![layer],
                    selection: None,
                    selections: Default::default(),
                    metadata: Default::default(),
                    indexed: None,
                    animation: None,
                    name: "Sans titre".into(),
                },
                None,
                "Nouvelle image",
            );
        }
        Operation::OpenSvgEditable { path } => {
            let mut doc = io::import(&path)?;
            let Content::Vector(shapes) = &doc.layers[0].content else {
                return Err("Choisissez un fichier SVG".into());
            };
            let mut shapes = crate::svg_edit::decompose(shapes.first().ok_or("SVG vide")?)?;
            let mut next = 1;
            crate::groups::assign_ids(&mut shapes, &mut || {
                next += 1;
                next
            });
            crate::groups::validate(&shapes)?;
            doc.layers[0].content = Content::Vector(shapes);
            doc.metadata.notices.push("SVG décomposé en tracés éditables ; texte et traits convertis en surfaces. Les groupes neutres sont dépliés ; opacité, fusion et isolation des autres groupes sont conservées.".into());
            e.load(doc, None, "Ouvrir un SVG éditable");
        }
        Operation::Open { path } => {
            let doc = io::import(&path)?;
            let project = path.to_lowercase().ends_with(".prisme");
            e.load(doc, if project { Some(path) } else { None }, "Ouvrir");
        }
        Operation::Import { path } => {
            let imported = io::import(&path)?;
            if e.doc.is_none() {
                e.load(imported, None, "Importer");
                return Ok(());
            }
            if e.doc()?.layers.len() + imported.layers.len() > 256 {
                return Err("256 calques maximum".into());
            }
            crate::text_styles::merge(&mut e.doc_mut()?.text_styles, imported.text_styles)?;
            for mut layer in imported.layers {
                layer.id = e.next_id();
                layer.version = e.next_id();
                if let Content::Vector(shapes) = &mut layer.content {
                    crate::groups::assign_ids(shapes, &mut || e.next_id());
                }
                e.active_hint = Some(layer.id);
                e.doc_mut()?.layers.push(layer);
            }
            e.commit("Importer des calques");
        }
        Operation::CopyClipboard => {
            let doc = e.doc()?;
            let mut image = vector::flatten(doc)?;
            if let Some(selection) = &doc.selection {
                let (x, y, w, h) = selection
                    .bounds(doc.width, doc.height)
                    .ok_or("Sélection vide")?;
                let mask = selection.mask(doc.width, doc.height);
                for (p, m) in image.pixels_mut().zip(mask.iter()) {
                    p[3] = ((p[3] as u16 * *m as u16 + 127) / 255) as u8;
                }
                image = imageops::crop_imm(&image, x, y, w, h).to_image();
            }
            let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            clipboard
                .set_image(arboard::ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: std::borrow::Cow::Owned(image.into_raw()),
                })
                .map_err(|e| e.to_string())?;
        }
        Operation::PasteClipboard { as_new } => {
            let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            let data = clipboard
                .get_image()
                .map_err(|_| "Le presse-papiers ne contient pas d'image compatible")?;
            io::dimensions(data.width as u32, data.height as u32)?;
            let img = RgbaImage::from_raw(
                data.width as u32,
                data.height as u32,
                data.bytes.into_owned(),
            )
            .ok_or("Image du presse-papiers invalide")?;
            let (w, h) = img.dimensions();
            let layer = e.new_raster_layer("Presse-papiers", img, 0, 0);
            e.active_hint = Some(layer.id);
            if e.doc.is_none() || as_new {
                e.load(
                    Document {
                        text_styles: Vec::new(),
                        width: w,
                        height: h,
                        layers: vec![layer],
                        selection: None,
                        selections: Default::default(),
                        metadata: Default::default(),
                        indexed: None,
                        animation: None,
                        name: "Presse-papiers".into(),
                    },
                    None,
                    "Coller",
                );
            } else {
                if e.doc()?.layers.len() >= 256 {
                    return Err("256 calques maximum".into());
                }
                e.doc_mut()?.layers.push(layer);
                e.commit("Coller");
            }
        }
        Operation::Paste { png } => {
            if png.len() > 128 * 1024 * 1024 {
                return Err("Image du presse-papiers trop grande".into());
            }
            let bytes = STANDARD
                .decode(png.split(',').next_back().unwrap_or(&png))
                .map_err(|e| e.to_string())?;
            let img = io::decode(&bytes)?;
            let (w, h) = img.dimensions();
            let layer = e.new_raster_layer("Presse-papiers", img, 0, 0);
            e.active_hint = Some(layer.id);
            if e.doc.is_none() {
                e.load(
                    Document {
                        text_styles: Vec::new(),
                        width: w,
                        height: h,
                        layers: vec![layer],
                        selection: None,
                        selections: Default::default(),
                        metadata: Default::default(),
                        indexed: None,
                        animation: None,
                        name: "Presse-papiers".into(),
                    },
                    None,
                    "Coller",
                );
            } else {
                if e.doc()?.layers.len() >= 256 {
                    return Err("256 calques maximum".into());
                }
                e.doc_mut()?.layers.push(layer);
                e.commit("Coller");
            }
        }
        Operation::ImportSelection { path } => {
            let doc = e.doc()?;
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            let target = doc
                .selection
                .as_ref()
                .and_then(|s| s.bounds(doc.width, doc.height));
            let selection = crate::interop::parse_pfs(&bytes, doc.width, doc.height, target)?;
            e.doc_mut()?.selection = Some(selection);
            e.commit("Importer la sélection PFS");
        }
        Operation::Save { path } => {
            if !path.to_lowercase().ends_with(".prisme") {
                return Err("Les projets utilisent l'extension .prisme".into());
            }
            let bytes = io::project_bytes(e.doc()?)?;
            io::atomic_write(&path, &bytes)?;
            e.mark_saved(path);
        }
        Operation::Export {
            path,
            quality,
            optimized,
            precision,
        } => {
            if optimized && path.to_lowercase().ends_with(".svg") {
                io::atomic_write(
                    &path,
                    crate::vector_export::optimized_svg(e.doc()?, precision)?.as_bytes(),
                )?;
            } else {
                io::export(e.doc()?, &path, quality)?;
            }
        }
        Operation::Close => e.close(),
        Operation::Undo => e.undo()?,
        Operation::Redo => e.redo()?,
        Operation::Jump { index } => e.jump(index)?,
        Operation::AddLayer { kind } => {
            let doc = e.doc()?;
            let (w, h) = (doc.width, doc.height);
            if doc.layers.len() >= 256 {
                return Err("256 calques maximum".into());
            }
            let layer = match kind.as_str() {
                "raster" => e.new_raster_layer("Calque bitmap", RgbaImage::new(w, h), 0, 0),
                "vector" => e.new_vector_layer("Calque vectoriel", vec![]),
                _ => return Err("Type de calque inconnu".into()),
            };
            e.active_hint = Some(layer.id);
            e.doc_mut()?.layers.push(layer);
            e.commit("Ajouter un calque");
        }
        Operation::DeleteLayer { id } => {
            let idx = e.layer_index(id)?;
            if e.doc()?.layers.len() == 1 {
                return Err("Conservez au moins un calque".into());
            }
            e.doc_mut()?.layers.remove(idx);
            e.active_hint = Some(e.doc()?.layers[idx.saturating_sub(1)].id);
            e.commit("Supprimer le calque");
        }
        Operation::DuplicateLayer { id } => {
            let idx = e.layer_index(id)?;
            if e.doc()?.layers.len() >= 256 {
                return Err("256 calques maximum".into());
            }
            let mut layer = e.doc()?.layers[idx].clone();
            layer.id = e.next_id();
            layer.version = e.next_id();
            layer.name.push_str(" — copie");
            if let Content::Vector(shapes) = &mut layer.content {
                crate::groups::assign_ids(shapes, &mut || e.next_id());
            }
            e.active_hint = Some(layer.id);
            e.doc_mut()?.layers.insert(idx + 1, layer);
            e.commit("Dupliquer le calque");
        }
        Operation::Layer {
            id,
            name,
            visible,
            opacity,
            blend,
            x,
            y,
        } => {
            let idx = e.layer_index(id)?;
            let l = &mut e.doc_mut()?.layers[idx];
            if let Some(v) = name {
                l.name = v;
            }
            if let Some(v) = visible {
                l.visible = v;
            }
            if let Some(v) = opacity {
                l.opacity = v.clamp(0.0, 1.0);
            }
            if let Some(v) = blend {
                l.blend = v;
            }
            if let Some(v) = x {
                l.x = v;
            }
            if let Some(v) = y {
                l.y = v;
            }
            e.touch(idx);
            e.commit("Propriétés du calque");
        }
        Operation::Reorder { id, index } => {
            let idx = e.layer_index(id)?;
            if index >= e.doc()?.layers.len() {
                return Err("Position de calque invalide".into());
            }
            let layer = e.doc_mut()?.layers.remove(idx);
            e.doc_mut()?.layers.insert(index, layer);
            e.commit("Réordonner les calques");
        }
        Operation::Effects { id, effects } => {
            let idx = e.layer_index(id)?;
            let doc = e.doc()?;
            vector::style_dimensions(&effects, doc.width, doc.height)?;
            e.doc_mut()?.layers[idx].effects = effects;
            e.touch(idx);
            e.commit("Effets du calque");
        }
        Operation::MergeDown { id } => {
            let idx = e.layer_index(id)?;
            if idx == 0 {
                return Err("Aucun calque en dessous".into());
            }
            let doc = e.doc()?;
            let below = &doc.layers[idx - 1];
            let above = &doc.layers[idx];
            if !below.visible || !above.visible {
                return Err("Rendez les deux calques visibles avant de les fusionner".into());
            }
            if idx > 1 && (below.blend != BlendMode::Normal || above.blend != BlendMode::Normal) {
                return Err("Ces modes dépendent des calques du dessous. Utilisez « Fusionner jusqu’à ce calque » pour conserver exactement le rendu et les calques supérieurs.".into());
            }
            let mut img = RgbaImage::new(doc.width, doc.height);
            for l in [below, above] {
                let (pixels, x, y) = vector::layer_surface(l, doc.width, doc.height)?;
                raster::composite(&mut img, &pixels, x, y, l.opacity, l.blend);
            }
            let mut merged = below.clone();
            merged.content = Content::Raster(Arc::new(img));
            merged.x = 0;
            merged.y = 0;
            merged.opacity = 1.0;
            merged.blend = BlendMode::Normal;
            merged.effects = Default::default();
            e.active_hint = Some(merged.id);
            e.doc_mut()?.layers[idx - 1] = merged;
            e.doc_mut()?.layers.remove(idx);
            e.touch(idx - 1);
            e.commit("Fusionner vers le bas");
        }
        Operation::MergePrefix { id } => {
            let idx = e.layer_index(id)?;
            if idx == 0 {
                return Err("Aucun calque en dessous".into());
            }
            let mut prefix = e.doc()?.clone();
            prefix.layers.truncate(idx + 1);
            let image = vector::flatten(&prefix)?;
            let layer = e.new_raster_layer("Calques fusionnés", image, 0, 0);
            e.active_hint = Some(layer.id);
            e.doc_mut()?.layers.splice(0..=idx, [layer]);
            e.commit("Fusionner jusqu’au calque");
        }
        Operation::Flatten => {
            let img = vector::flatten(e.doc()?)?;
            let layer = e.new_raster_layer("Image aplatie", img, 0, 0);
            e.active_hint = Some(layer.id);
            e.doc_mut()?.layers = vec![layer];
            e.commit("Aplatir l'image");
        }
        Operation::Rasterize { id } => {
            let idx = e.layer_index(id)?;
            let doc = e.doc()?;
            let (img, x, y) = vector::content_surface(&doc.layers[idx], doc.width, doc.height)?;
            e.doc_mut()?.layers[idx].x = x;
            e.doc_mut()?.layers[idx].y = y;
            set_pixels(e, idx, img, "Rastériser le calque", false)?;
        }
        Operation::Shape { id, mut shape } => {
            io::validate_shape(&shape)?;
            let idx = e.layer_index(id)?;
            if shape.id == 0 {
                crate::groups::assign_ids(std::slice::from_mut(&mut shape), &mut || e.next_id());
            }
            match &mut e.doc_mut()?.layers[idx].content {
                Content::Vector(shapes) => {
                    if let Some(existing) = crate::groups::find_mut(shapes, shape.id) {
                        *existing = shape;
                    } else {
                        shapes.push(shape);
                    }
                    crate::groups::validate(shapes)?;
                }
                _ => return Err("Sélectionnez un calque vectoriel".into()),
            }
            e.touch(idx);
            e.commit("Éditer une forme");
        }
        Operation::GroupShapes { id, shape_ids } => {
            let idx = e.layer_index(id)?;
            let group_id = e.next_id();
            let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            crate::groups::group(shapes, &shape_ids, group_id)?;
            crate::groups::validate(shapes)?;
            e.touch(idx);
            e.commit("Grouper les formes");
        }
        Operation::UngroupShapes { id, shape_id } => {
            let idx = e.layer_index(id)?;
            let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            crate::groups::ungroup(shapes, shape_id)?;
            crate::groups::validate(shapes)?;
            e.touch(idx);
            e.commit("Dissocier le groupe");
        }
        Operation::DecomposeSvg { id, shape_id } => {
            let idx = e.layer_index(id)?;
            let Content::Vector(shapes) = &e.doc()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            let siblings = crate::groups::siblings(shapes, shape_id).ok_or("Forme introuvable")?;
            let at = siblings
                .iter()
                .position(|s| s.id == shape_id)
                .ok_or("Forme introuvable")?;
            let mut decomposed = crate::svg_edit::decompose(
                crate::groups::find(shapes, shape_id).ok_or("Forme introuvable")?,
            )?;
            crate::groups::assign_ids(&mut decomposed, &mut || e.next_id());
            if let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content {
                crate::groups::siblings_mut(shapes, shape_id)
                    .ok_or("Forme introuvable")?
                    .splice(at..=at, decomposed);
            }
            e.touch(idx);
            e.commit("Décomposer le SVG en tracés");
        }
        Operation::ToPath { id, shape_id } => {
            let idx = e.layer_index(id)?;
            let next = if let Content::Vector(shapes) = &e.doc()?.layers[idx].content {
                let original = crate::groups::find(shapes, shape_id).ok_or("Forme introuvable")?;
                if matches!(original.kind, ShapeKind::Text { .. }) {
                    Some(crate::text_path::outlines(original)?)
                } else {
                    None
                }
            } else {
                None
            };
            let next = next.map(|mut s| {
                if let ShapeKind::Group { children, .. } = &mut s.kind {
                    crate::groups::assign_ids(children, &mut || e.next_id());
                }
                s
            });
            let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            let shape = crate::groups::find_mut(shapes, shape_id).ok_or("Forme introuvable")?;
            *shape = if let Some(s) = next {
                s
            } else {
                crate::paths::convert(shape)?
            };
            e.touch(idx);
            e.commit("Convertir en tracé");
        }
        Operation::Boolean {
            id,
            first,
            second,
            kind,
        } => {
            if first == second {
                return Err("Choisissez deux formes distinctes".into());
            }
            let idx = e.layer_index(id)?;
            let Content::Vector(shapes) = &e.doc()?.layers[idx].content else {
                return Err("Calque vectoriel requis".into());
            };
            let siblings =
                crate::groups::siblings(shapes, first).ok_or("Première forme introuvable")?;
            let a = siblings
                .iter()
                .find(|s| s.id == first)
                .ok_or("Première forme introuvable")?;
            let b = siblings
                .iter()
                .find(|s| s.id == second)
                .ok_or("Seconde forme introuvable")?;
            let mut result = crate::paths::boolean(a, b, &kind)?;
            result.id = e.next_id();
            if let Content::Vector(shapes) = &mut e.doc_mut()?.layers[idx].content {
                let shapes =
                    crate::groups::siblings_mut(shapes, first).ok_or("Forme introuvable")?;
                let at = shapes
                    .iter()
                    .position(|s| s.id == first || s.id == second)
                    .unwrap();
                shapes.retain(|s| s.id != first && s.id != second);
                shapes.insert(at, result);
            }
            e.touch(idx);
            e.commit("Opération booléenne");
        }
        Operation::DeleteShape { id, shape_id } => {
            let idx = e.layer_index(id)?;
            match &mut e.doc_mut()?.layers[idx].content {
                Content::Vector(shapes) => {
                    if !crate::groups::remove(shapes, shape_id) {
                        return Err("Forme introuvable".into());
                    }
                }
                _ => return Err("Calque vectoriel requis".into()),
            };
            e.touch(idx);
            e.commit("Supprimer la forme");
        }
        Operation::Stroke {
            id,
            points,
            size,
            color: ink,
            opacity,
            erase,
        } => {
            if points.len() > 100_000
                || points
                    .iter()
                    .flatten()
                    .any(|p| !p.is_finite() || p.abs() > 1e6)
            {
                return Err("Tracé invalide".into());
            }
            let (idx, mut img, x, y) = raster_layer(e, id)?;
            let points = points
                .iter()
                .map(|p| [p[0] - x as f32, p[1] - y as f32])
                .collect::<Vec<_>>();
            raster::stroke(&mut img, &points, size / 2.0, color(&ink)?, opacity, erase);
            set_pixels(e, idx, img, if erase { "Gomme" } else { "Pinceau" }, true)?;
        }
        Operation::Asset { id, settings } => {
            let (idx, img, _, _) = raster_layer(e, id)?;
            let changed = crate::assets::apply_file(&img, &settings)?;
            let label = match settings.kind {
                crate::assets::Kind::Mask => "Appliquer un masque",
                crate::assets::Kind::Frame => "Appliquer un cadre",
                crate::assets::Kind::Pattern => "Remplir avec un motif",
                crate::assets::Kind::Texture => "Appliquer une texture",
            };
            set_pixels(e, idx, changed, label, true)?;
        }
        Operation::Retouch {
            id,
            tool,
            points,
            size,
            strength,
            source_offset,
            color: ink,
            nozzle,
            nozzle_path,
        } => {
            let (idx, img, x, y) = raster_layer(e, id)?;
            let points = points
                .iter()
                .map(|p| [p[0] - x as f32, p[1] - y as f32])
                .collect::<Vec<_>>();
            let stamp = nozzle_path
                .map(|path| io::import(&path).and_then(|doc| vector::flatten(&doc)))
                .transpose()?;
            let seed = e.next_id() as u32;
            let changed = crate::retouch::apply(
                &img,
                &crate::retouch::Settings {
                    tool: &tool,
                    points: &points,
                    size,
                    strength,
                    source_offset,
                    color: color(&ink)?,
                    seed,
                    nozzle: nozzle.as_deref().unwrap_or("star"),
                    stamp: stamp.as_ref(),
                },
            )?;
            let label = match tool.as_str() {
                "clone" => "Tampon de clonage",
                "heal" => "Correcteur",
                "smudge" => "Doigt",
                "dodge" => "Éclaircir",
                "burn" => "Assombrir",
                "airbrush" => "Aérographe",
                "nozzle" => "Tube",
                _ => "Retouche",
            };
            set_pixels(e, idx, changed, label, true)?;
        }
        Operation::Fill {
            id,
            x,
            y,
            color: ink,
            tolerance,
        } => {
            let (idx, mut img, lx, ly) = raster_layer(e, id)?;
            let doc = e.doc()?;
            let allowed = doc.selection.as_ref().map(|s| {
                let mask = s.mask(doc.width, doc.height);
                let mut local = vec![0; (img.width() * img.height()) as usize];
                for py in 0..img.height() {
                    for px in 0..img.width() {
                        let (dx, dy) = (px as i64 + lx as i64, py as i64 + ly as i64);
                        if dx >= 0 && dy >= 0 && dx < doc.width as i64 && dy < doc.height as i64 {
                            local[(py * img.width() + px) as usize] =
                                mask[(dy * doc.width as i64 + dx) as usize];
                        }
                    }
                }
                local
            });
            raster::flood_fill(
                &mut img,
                x.saturating_sub(lx),
                y.saturating_sub(ly),
                color(&ink)?,
                tolerance,
                allowed.as_deref(),
            );
            set_pixels(e, idx, img, "Pot de peinture", true)?;
        }
        Operation::Gradient {
            id,
            start,
            end,
            color: a,
            secondary: b,
        } => {
            let (idx, mut img, x, y) = raster_layer(e, id)?;
            raster::gradient(
                &mut img,
                [start[0] - x as f32, start[1] - y as f32],
                [end[0] - x as f32, end[1] - y as f32],
                color(&a)?,
                color(&b)?,
            );
            set_pixels(e, idx, img, "Dégradé", true)?;
        }
        Operation::Pfl {
            id,
            path,
            timeout_ms,
            parent_window,
        } => {
            let applied = crate::pfl::apply(
                e,
                id,
                std::path::Path::new(&path),
                timeout_ms,
                parent_window,
            )?;
            if applied.outcome.changed {
                let mut document = applied
                    .document
                    .ok_or("Le PFL a annoncé une modification sans document")?;
                if applied.outcome.new_document {
                    if e.doc.is_some() {
                        return Err(
                            "Une nouvelle image PFL ne peut pas remplacer le document ouvert"
                                .into(),
                        );
                    }
                    for layer in &mut document.layers {
                        layer.id = e.next_id();
                        layer.version = e.next_id();
                    }
                    e.load(document, None, "Image du plugin PFL");
                    e.commit("Importer par plugin PFL");
                } else {
                    let current = e.doc()?;
                    crate::pfl::reuse_snapshot(&mut document, current, &applied.outcome.touched)?;
                    let mut staged = e.clone();
                    staged.doc = Some(document);
                    for id in &applied.outcome.touched {
                        let index = staged.layer_index(*id)?;
                        staged.touch(index);
                    }
                    e.adopt(staged, "Plugin PhotoFiltre PFL");
                }
            }
            crate::pfl::remember(applied.outcome);
        }
        Operation::EightBf {
            id,
            path,
            key,
            settings,
        } => {
            let (idx, img, x, y) = raster_layer(e, id)?;
            let doc = e.doc()?;
            let mask = doc.selection.as_ref().map(|s| {
                let full = s.mask(doc.width, doc.height);
                let mut local = vec![0; img.width() as usize * img.height() as usize];
                for ly in 0..img.height() as i64 {
                    for lx in 0..img.width() as i64 {
                        let (dx, dy) = (lx + x as i64, ly + y as i64);
                        if dx >= 0 && dy >= 0 && dx < doc.width as i64 && dy < doc.height as i64 {
                            local[ly as usize * img.width() as usize + lx as usize] =
                                full[dy as usize * doc.width as usize + dx as usize];
                        }
                    }
                }
                local
            });
            let result = crate::eightbf::apply(
                &img,
                std::path::Path::new(&path),
                &key,
                &settings,
                mask.as_deref(),
                &doc.name,
                &doc.layers[idx].name,
            )?;
            // Le cœur 8BF a déjà composé le masque ; ne pas appliquer deux fois le bord progressif.
            set_pixels(e, idx, result.image, "Filtre Photoshop 8BF", false)?;
            *captured = Some(Operation::EightBf {
                id,
                path,
                key,
                settings: result.settings,
            });
        }
        Operation::Plugin {
            id,
            manifest,
            params,
        } => {
            let (idx, img, _, _) = raster_layer(e, id)?;
            let changed = crate::plugins::apply(&img, std::path::Path::new(&manifest), &params)?;
            set_pixels(e, idx, changed, "Plugin", true)?;
        }
        Operation::Filter { id, name, params } => {
            let (idx, img, _, _) = raster_layer(e, id)?;
            let changed = raster::filter(&img, &name, &params)?;
            set_pixels(e, idx, changed, &format!("Filtre · {name}"), true)?;
        }
        Operation::Selection {
            selection: incoming,
            mode,
        } => {
            let doc = e.doc_mut()?;
            doc.selection = if let Some(incoming) = incoming {
                selection::validate(&incoming, doc.width, doc.height)?;
                Some(selection::combine(
                    doc.selection.as_ref(),
                    incoming,
                    doc.width,
                    doc.height,
                    mode,
                ))
            } else {
                None
            };
            e.commit("Sélection");
        }
        Operation::SelectColor {
            x,
            y,
            tolerance,
            contiguous,
            mode,
        } => {
            let doc = e.doc()?;
            let incoming =
                selection::by_color(&vector::flatten(doc)?, x, y, tolerance, contiguous)?;
            let next = selection::combine(
                doc.selection.as_ref(),
                incoming,
                doc.width,
                doc.height,
                mode,
            );
            e.doc_mut()?.selection = Some(next);
            e.commit(if contiguous {
                "Baguette magique"
            } else {
                "Sélection par couleur"
            });
        }
        Operation::ModifySelection { kind, radius } => {
            let doc = e.doc()?;
            let current = doc.selection.as_ref().ok_or("Créez une sélection")?;
            let next = selection::modify(current, doc.width, doc.height, &kind, radius)?;
            e.doc_mut()?.selection = Some(next);
            e.commit("Modifier la sélection");
        }
        Operation::RememberSelection { name } => {
            let name = name.trim();
            if name.is_empty() || name.len() > 128 {
                return Err("Nom de sélection : 1 à 128 caractères".into());
            }
            let doc = e.doc_mut()?;
            if doc.selections.len() >= 32 && !doc.selections.contains_key(name) {
                return Err("32 sélections mémorisées maximum".into());
            }
            let current = doc.selection.clone().ok_or("Créez une sélection")?;
            doc.selections.insert(name.into(), current);
            e.commit("Mémoriser la sélection");
        }
        Operation::RestoreSelection { name } => {
            let current = e
                .doc()?
                .selections
                .get(&name)
                .cloned()
                .ok_or("Sélection mémorisée introuvable")?;
            e.doc_mut()?.selection = Some(current);
            e.commit("Rappeler la sélection");
        }
        Operation::ForgetSelection { name } => {
            if e.doc_mut()?.selections.remove(&name).is_none() {
                return Err("Sélection mémorisée introuvable".into());
            }
            e.commit("Oublier la sélection");
        }
        Operation::InvertSelection => {
            let doc = e.doc_mut()?;
            if let Some(sel) = &mut doc.selection {
                sel.inverted = !sel.inverted;
            } else {
                doc.selection = Some(Selection {
                    shape: crate::document::SelShape::Rect {
                        x: 0.0,
                        y: 0.0,
                        w: doc.width as f32,
                        h: doc.height as f32,
                    },
                    inverted: false,
                });
            }
            e.commit("Inverser la sélection");
        }
        Operation::Resize { width, height } => {
            io::dimensions(width, height)?;
            let doc = e.doc_mut()?;
            let (sx, sy) = (
                width as f32 / doc.width as f32,
                height as f32 / doc.height as f32,
            );
            for layer in &mut doc.layers {
                layer.x = (layer.x as f32 * sx).round() as i32;
                layer.y = (layer.y as f32 * sy).round() as i32;
                match &mut layer.content {
                    Content::Raster(img) => {
                        let (w, h) = (
                            (img.width() as f32 * sx).round().max(1.0) as u32,
                            (img.height() as f32 * sy).round().max(1.0) as u32,
                        );
                        io::dimensions(w, h)?;
                        let premul = image::ImageBuffer::<Rgba<f32>, Vec<f32>>::from_fn(
                            img.width(),
                            img.height(),
                            |x, y| {
                                let p = img.get_pixel(x, y);
                                let a = p[3] as f32 / 255.0;
                                Rgba([
                                    p[0] as f32 / 255.0 * a,
                                    p[1] as f32 / 255.0 * a,
                                    p[2] as f32 / 255.0 * a,
                                    a,
                                ])
                            },
                        );
                        let resized =
                            imageops::resize(&premul, w, h, imageops::FilterType::Lanczos3);
                        *img = Arc::new(RgbaImage::from_fn(w, h, |x, y| {
                            let p = resized.get_pixel(x, y);
                            let a = p[3].clamp(0.0, 1.0);
                            if a < 0.0001 {
                                Rgba([0; 4])
                            } else {
                                Rgba([
                                    (p[0] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                                    (p[1] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                                    (p[2] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                                    (a * 255.0).round() as u8,
                                ])
                            }
                        }));
                    }
                    Content::Vector(shapes) => {
                        for s in shapes {
                            if s.transform.is_some()
                                || s.fill_pattern.is_some()
                                || s.stroke_pattern.is_some()
                                || s.fill_gradient.is_some()
                                || s.stroke_gradient.is_some()
                                || matches!(
                                    s.kind,
                                    ShapeKind::Group { .. } | ShapeKind::Image { .. }
                                )
                            {
                                crate::groups::apply(s, [sx, 0., 0., sy, 0., 0.]);
                                continue;
                            }
                            s.x *= sx;
                            s.y *= sy;
                            s.w *= sx;
                            s.h *= sy;
                            s.stroke_width *= (sx + sy) / 2.0;
                            if let ShapeKind::Text { font_size, .. } = &mut s.kind {
                                *font_size *= sy;
                            }
                        }
                    }
                }
            }
            doc.width = width;
            doc.height = height;
            doc.selection = None;
            validate_vectors(e)?;
            touch_all(e);
            e.commit("Redimensionner l'image");
        }
        Operation::Canvas {
            width,
            height,
            offset_x,
            offset_y,
        } => canvas(
            e,
            width,
            height,
            offset_x,
            offset_y,
            "Taille de la zone de travail",
        )?,
        Operation::Crop => {
            let doc = e.doc()?;
            let sel = doc
                .selection
                .as_ref()
                .ok_or("Créez une sélection avant de recadrer")?;
            let (x, y, w, h) = sel.bounds(doc.width, doc.height).ok_or("Sélection vide")?;
            canvas(e, w, h, -(x as i32), -(y as i32), "Recadrer")?;
        }
        Operation::Rotate { degrees } => {
            if !degrees.is_finite() {
                return Err("Angle invalide".into());
            }
            let deg = degrees.rem_euclid(360.0);
            if deg % 90.0 == 0.0 {
                for _ in 0..(deg / 90.0) as usize {
                    rotate90(e)?;
                }
            } else {
                rotate_free(e, deg)?;
            }
            validate_vectors(e)?;
            touch_all(e);
            e.commit("Rotation de l'image");
        }
        Operation::Flip { horizontal } => {
            let replacement_ids = (0..e.doc()?.layers.len())
                .map(|_| e.next_id())
                .collect::<Vec<_>>();
            let doc = e.doc_mut()?;
            for (index, l) in doc.layers.iter_mut().enumerate() {
                l.effects.mirror(horizontal);
                match &mut l.content {
                    Content::Raster(img) => {
                        if horizontal {
                            l.x = doc.width as i32 - l.x - img.width() as i32;
                            *img = Arc::new(imageops::flip_horizontal(&**img));
                        } else {
                            l.y = doc.height as i32 - l.y - img.height() as i32;
                            *img = Arc::new(imageops::flip_vertical(&**img));
                        }
                    }
                    Content::Vector(shapes) => {
                        if shapes.iter().any(|s| {
                            s.transform.is_some()
                                || s.fill_pattern.is_some()
                                || s.stroke_pattern.is_some()
                                || s.fill_gradient.is_some()
                                || s.stroke_gradient.is_some()
                                || matches!(
                                    s.kind,
                                    ShapeKind::Group { .. } | ShapeKind::Image { .. }
                                )
                        }) {
                            let m = if horizontal {
                                [-1., 0., 0., 1., doc.width as f32, 0.]
                            } else {
                                [1., 0., 0., -1., 0., doc.height as f32]
                            };
                            let m = crate::groups::multiply(
                                m,
                                crate::groups::translation(l.x as f32, l.y as f32),
                            );
                            for s in shapes {
                                crate::groups::apply(s, m);
                            }
                            l.x = 0;
                            l.y = 0;
                            continue;
                        }
                        let source = vector::svg_document(
                            doc.width,
                            doc.height,
                            &format!(
                                "<g transform=\"{}\">{}</g>",
                                if horizontal {
                                    format!("translate({} 0) scale(-1 1)", doc.width)
                                } else {
                                    format!("translate(0 {}) scale(1 -1)", doc.height)
                                },
                                shapes.iter().map(vector::shape_svg).collect::<String>()
                            ),
                        );
                        *shapes = vec![Shape {
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
                            id: replacement_ids[index],
                            kind: ShapeKind::Svg {
                                source,
                                intrinsic_w: doc.width as f32,
                                intrinsic_h: doc.height as f32,
                            },
                            x: 0.0,
                            y: 0.0,
                            w: doc.width as f32,
                            h: doc.height as f32,
                            rotation: 0.0,
                            fill: None,
                            stroke: None,
                            stroke_width: 0.0,
                            opacity: 1.0,
                        }];
                        if horizontal {
                            l.x = -l.x;
                        } else {
                            l.y = -l.y;
                        }
                    }
                }
            }
            doc.selection = None;
            validate_vectors(e)?;
            touch_all(e);
            e.commit("Miroir");
        }
    }
    Ok(())
}
fn validate_vectors(e: &Editor) -> Result<(), String> {
    for layer in &e.doc()?.layers {
        if let Content::Vector(shapes) = &layer.content {
            crate::groups::validate(shapes)?;
        }
    }
    Ok(())
}
fn select_animation_frame(e: &mut Editor, index: usize) -> Result<(), String> {
    let animation = e
        .doc()?
        .animation
        .as_ref()
        .ok_or("Aucune animation ouverte")?;
    let image = animation
        .frames
        .get(index)
        .ok_or("Image d'animation introuvable")?
        .image
        .clone();
    let doc = e.doc_mut()?;
    doc.layers[0].content = Content::Raster(image);
    doc.animation.as_mut().unwrap().current = index;
    e.touch(0);
    Ok(())
}
fn touch_all(e: &mut Editor) {
    let n = e.doc.as_ref().map_or(0, |d| d.layers.len());
    for idx in 0..n {
        e.touch(idx);
    }
}
fn canvas(e: &mut Editor, w: u32, h: u32, x: i32, y: i32, label: &str) -> Result<(), String> {
    io::dimensions(w, h)?;
    let doc = e.doc_mut()?;
    for layer in &mut doc.layers {
        layer.x = layer.x.saturating_add(x);
        layer.y = layer.y.saturating_add(y);
    }
    doc.width = w;
    doc.height = h;
    doc.selection = None;
    touch_all(e);
    e.commit(label);
    Ok(())
}
fn rotate90(e: &mut Editor) -> Result<(), String> {
    let doc = e.doc_mut()?;
    let h = doc.height as i32;
    for l in &mut doc.layers {
        l.effects.rotate(90.0);
        match &mut l.content {
            Content::Raster(img) => {
                let x = h - l.y - img.height() as i32;
                l.y = l.x;
                l.x = x;
                *img = Arc::new(imageops::rotate90(&**img));
            }
            Content::Vector(shapes) => {
                for s in shapes {
                    if s.transform.is_some()
                        || s.fill_pattern.is_some()
                        || s.stroke_pattern.is_some()
                        || s.fill_gradient.is_some()
                        || s.stroke_gradient.is_some()
                        || matches!(s.kind, ShapeKind::Group { .. } | ShapeKind::Image { .. })
                    {
                        crate::groups::apply(
                            s,
                            crate::groups::multiply(
                                [0., 1., -1., 0., h as f32, 0.],
                                crate::groups::translation(l.x as f32, l.y as f32),
                            ),
                        );
                        continue;
                    }
                    let cx = s.x + l.x as f32 + s.w / 2.0;
                    let cy = s.y + l.y as f32 + s.h / 2.0;
                    s.x = h as f32 - cy - s.w / 2.0;
                    s.y = cx - s.h / 2.0;
                    s.rotation += 90.0;
                }
                l.x = 0;
                l.y = 0;
            }
        }
    }
    std::mem::swap(&mut doc.width, &mut doc.height);
    doc.selection = None;
    Ok(())
}
fn rotate_free(e: &mut Editor, deg: f32) -> Result<(), String> {
    let doc = e.doc_mut()?;
    let (sn, cs) = deg.to_radians().sin_cos();
    let (ow, oh) = (doc.width as f32, doc.height as f32);
    let (w, h) = (
        (ow * cs.abs() + oh * sn.abs()).ceil() as u32,
        (ow * sn.abs() + oh * cs.abs()).ceil() as u32,
    );
    io::dimensions(w, h)?;
    let center = |x: f32, y: f32| {
        let (x, y) = (x - ow / 2.0, y - oh / 2.0);
        (
            cs * x - sn * y + w as f32 / 2.0,
            sn * x + cs * y + h as f32 / 2.0,
        )
    };
    for l in &mut doc.layers {
        l.effects.rotate(deg);
        match &mut l.content {
            Content::Raster(img) => {
                let (cx, cy) = center(
                    l.x as f32 + img.width() as f32 / 2.0,
                    l.y as f32 + img.height() as f32 / 2.0,
                );
                let rotated = raster::rotate(img, deg)?;
                l.x = (cx - rotated.width() as f32 / 2.0).round() as i32;
                l.y = (cy - rotated.height() as f32 / 2.0).round() as i32;
                *img = Arc::new(rotated);
            }
            Content::Vector(shapes) => {
                for s in shapes {
                    if s.transform.is_some()
                        || s.fill_pattern.is_some()
                        || s.stroke_pattern.is_some()
                        || s.fill_gradient.is_some()
                        || s.stroke_gradient.is_some()
                        || matches!(s.kind, ShapeKind::Group { .. } | ShapeKind::Image { .. })
                    {
                        let (tx, ty) = center(0., 0.);
                        crate::groups::apply(
                            s,
                            crate::groups::multiply(
                                [cs, sn, -sn, cs, tx, ty],
                                crate::groups::translation(l.x as f32, l.y as f32),
                            ),
                        );
                        continue;
                    }
                    let (cx, cy) =
                        center(s.x + l.x as f32 + s.w / 2.0, s.y + l.y as f32 + s.h / 2.0);
                    s.x = cx - s.w / 2.0;
                    s.y = cy - s.h / 2.0;
                    s.rotation += deg;
                }
                l.x = 0;
                l.y = 0;
            }
        }
    }
    doc.width = w;
    doc.height = h;
    doc.selection = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_preserves_normalized_rgb_and_alpha_without_darkening() {
        for ink in ["#ffffff", "#ffff00", "#a0c0e080"] {
            let mut e = Editor::default();
            run(
                &mut e,
                serde_json::json!({"type":"new","width":8,"height":6,"background":ink}),
            )
            .unwrap();
            run(
                &mut e,
                serde_json::json!({"type":"resize","width":4,"height":3}),
            )
            .unwrap();
            let expected = color(ink).unwrap();
            assert!(e.doc().unwrap().layers[0]
                .raster()
                .unwrap()
                .pixels()
                .all(|p| (0..4).all(|c| p[c].abs_diff(expected[c]) <= 1)));
        }
    }
    #[test]
    fn mirrored_vector_remains_addressable_without_duplicating_the_shape() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":100,"height":80,"background":null}),
        )
        .unwrap();
        run(
            &mut e,
            serde_json::json!({"type":"addLayer","kind":"vector"}),
        )
        .unwrap();
        let id = e.active_hint.unwrap();
        run(&mut e,serde_json::json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":0,"x":10,"y":30,"w":20,"h":20,"fill":"#ff0000","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}})).unwrap();
        run(&mut e, serde_json::json!({"type":"flip","horizontal":true})).unwrap();
        let Content::Vector(shapes) = &e.doc().unwrap().layers[1].content else {
            panic!()
        };
        assert_ne!(shapes[0].id, 0);
        let mut shape = shapes[0].clone();
        shape.x += 5.0;
        assert_eq!(
            vector::flatten(e.doc().unwrap()).unwrap().get_pixel(70, 35)[3],
            255
        );
        execute(&mut e, Operation::Shape { id, shape }).unwrap();
        assert!(
            matches!(&e.doc().unwrap().layers[1].content,Content::Vector(shapes) if shapes.len()==1)
        );
        let rendered = vector::flatten(e.doc().unwrap()).unwrap();
        assert_eq!(rendered.get_pixel(70, 35)[3], 0);
        assert_eq!(rendered.get_pixel(80, 35).0, [255, 0, 0, 255]);
    }
    fn run(e: &mut Editor, v: Value) -> Result<(), String> {
        execute(e, serde_json::from_value(v).unwrap())
    }
    #[test]
    fn edit_selection_and_undo_pipeline() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":4,"height":3,"background":"#ffffff"}),
        )
        .unwrap();
        let id = e.doc().unwrap().layers[0].id;
        run(&mut e,serde_json::json!({"type":"selection","selection":{"shape":{"type":"rect","x":0,"y":0,"w":2,"h":3},"inverted":false}})).unwrap();
        run(
            &mut e,
            serde_json::json!({"type":"filter","id":id,"name":"invert","params":{}}),
        )
        .unwrap();
        let img = e.doc().unwrap().layers[0].raster().unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(img.get_pixel(3, 0).0, [255; 4]);
        e.undo().unwrap();
        assert_eq!(
            e.doc().unwrap().layers[0]
                .raster()
                .unwrap()
                .get_pixel(0, 0)
                .0,
            [255; 4]
        );
    }
    #[test]
    fn failed_resize_rolls_back_document() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":4,"height":3,"background":null}),
        )
        .unwrap();
        assert!(run(
            &mut e,
            serde_json::json!({"type":"resize","width":0,"height":3})
        )
        .is_err());
        assert_eq!(e.doc().unwrap().width, 4);
        assert_eq!(e.info().history.len(), 1);
    }
    #[test]
    fn rotation_preserves_pixels() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":4,"height":3,"background":"#ff0000"}),
        )
        .unwrap();
        run(&mut e, serde_json::json!({"type":"rotate","degrees":90})).unwrap();
        let d = e.doc().unwrap();
        assert_eq!((d.width, d.height), (3, 4));
        assert_eq!(
            vector::flatten(d).unwrap().get_pixel(0, 0).0,
            [255, 0, 0, 255]
        );
    }
    #[test]
    fn free_rotation_keeps_vector_editable_and_transparency_colored() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":8,"height":6,"background":"#ff000080"}),
        )
        .unwrap();
        run(
            &mut e,
            serde_json::json!({"type":"addLayer","kind":"vector"}),
        )
        .unwrap();
        let id = e.active_hint.unwrap();
        run(&mut e,serde_json::json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":0,"x":1,"y":1,"w":3,"h":2,"fill":"#00ff00","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}})).unwrap();
        run(&mut e, serde_json::json!({"type":"rotate","degrees":33.5})).unwrap();
        assert!(
            matches!(&e.doc().unwrap().layers[1].content,Content::Vector(shapes) if shapes[0].rotation==33.5)
        );
        for p in e.doc().unwrap().layers[0]
            .raster()
            .unwrap()
            .pixels()
            .filter(|p| p[3] > 0)
        {
            assert_eq!(p[0], 255);
            assert_eq!(p[1], 0);
        }
    }
    #[test]
    fn cropped_vector_is_translated_before_clipping_and_rasterizes_identically() {
        let mut e = Editor::default();
        run(
            &mut e,
            serde_json::json!({"type":"new","width":100,"height":100,"background":null}),
        )
        .unwrap();
        run(
            &mut e,
            serde_json::json!({"type":"addLayer","kind":"vector"}),
        )
        .unwrap();
        let id = e.active_hint.unwrap();
        run(&mut e,serde_json::json!({"type":"shape","id":id,"shape":{"id":0,"type":"rect","radius":0,"x":60,"y":60,"w":20,"h":20,"fill":"#00ff00","stroke":null,"strokeWidth":0,"opacity":1,"rotation":0}})).unwrap();
        run(&mut e,serde_json::json!({"type":"selection","selection":{"shape":{"type":"rect","x":60,"y":60,"w":20,"h":20},"inverted":false}})).unwrap();
        run(&mut e, serde_json::json!({"type":"crop"})).unwrap();
        let before = vector::flatten(e.doc().unwrap()).unwrap();
        assert_eq!(before.get_pixel(5, 5).0, [0, 255, 0, 255]);
        run(&mut e, serde_json::json!({"type":"rasterize","id":id})).unwrap();
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
        e.undo().unwrap();
        run(&mut e, serde_json::json!({"type":"mergeDown","id":id})).unwrap();
        assert_eq!(vector::flatten(e.doc().unwrap()).unwrap(), before);
    }
}
