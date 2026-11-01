//! Modèle de document : calques bitmap et vectoriels, sélection, historique.

use std::{collections::BTreeMap, sync::Arc};

use image::RgbaImage;
use resvg::tiny_skia;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    Difference,
    Add,
}

/// Forme vectorielle éditable. `x, y, w, h` est la boîte englobante en coordonnées
/// du document ; pour une ligne, `(x, y)` est le départ et `(x + w, y + h)` l'arrivée.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ShapeKind {
    Image {
        asset: crate::embedded::ImageAsset,
    },
    Group {
        children: Vec<Shape>,
        intrinsic_w: f32,
        intrinsic_h: f32,
    },
    Path {
        contours: Vec<crate::paths::Contour>,
        #[serde(default)]
        even_odd: bool,
    },
    Rect {
        radius: f32,
    },
    Ellipse,
    Line,
    Text {
        text: String,
        font_family: String,
        font_size: f32,
        bold: bool,
        italic: bool,
    },
    Svg {
        source: String,
        intrinsic_w: f32,
        intrinsic_h: f32,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_options: Option<crate::fonts::FontOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_path: Option<crate::text_path::TextPath>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub svg_unclipped: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub svg_viewport: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<crate::svg_filter::FilterChain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<crate::svg_mask::VectorMask>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<crate::clip::Clip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_style: Option<crate::group_style::GroupStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_pattern: Option<crate::paint::Pattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_pattern: Option<crate::paint::Pattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_gradient: Option<crate::paint::Gradient>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_gradient: Option<crate::paint::Gradient>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f32; 6]>,
    #[serde(default)]
    pub id: u64,
    #[serde(flatten)]
    pub kind: ShapeKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub rotation: f32,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default = "one")]
    pub opacity: f32,
}

fn one() -> f32 {
    1.0
}

#[derive(Clone)]
pub enum Content {
    Raster(Arc<RgbaImage>),
    Vector(Vec<Shape>),
}

#[derive(Clone)]
pub struct Layer {
    pub locks: crate::layer_tools::Locks,
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    /// Position du coin haut-gauche (calques bitmap seulement ; les formes ont leurs coordonnées).
    pub x: i32,
    pub y: i32,
    /// Change à chaque modification du contenu : l'interface ne recharge que ce qui a changé.
    pub version: u64,
    pub content: Content,
    pub effects: crate::effects::LayerEffects,
}

impl Layer {
    pub fn raster(&self) -> Option<&Arc<RgbaImage>> {
        match &self.content {
            Content::Raster(img) => Some(img),
            Content::Vector(_) => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SelShape {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Ellipse {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Polygon {
        points: Vec<[f32; 2]>,
    },
    Mask {
        width: u32,
        height: u32,
        #[serde(with = "crate::selection::mask_data")]
        data: Arc<Vec<u8>>,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub shape: SelShape,
    #[serde(default)]
    pub inverted: bool,
}

impl Selection {
    /// Boîte englobante entière, bornée au document. `None` si vide.
    pub fn bounds(&self, w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
        if self.inverted || matches!(self.shape, SelShape::Mask { .. }) {
            return crate::selection::bounds(&self.mask(w, h), w, h);
        }
        let (x0, y0, x1, y1) = match &self.shape {
            SelShape::Rect { x, y, w, h } | SelShape::Ellipse { x, y, w, h } => {
                (*x, *y, x + w, y + h)
            }
            SelShape::Polygon { points } => points.iter().fold(
                (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
                |(a, b, c, d), p| (a.min(p[0]), b.min(p[1]), c.max(p[0]), d.max(p[1])),
            ),
            SelShape::Mask { .. } => unreachable!(),
        };
        let x0 = x0.floor().clamp(0.0, w as f32) as u32;
        let y0 = y0.floor().clamp(0.0, h as f32) as u32;
        let x1 = x1.ceil().clamp(0.0, w as f32) as u32;
        let y1 = y1.ceil().clamp(0.0, h as f32) as u32;
        (x1 > x0 && y1 > y0).then(|| (x0, y0, x1 - x0, y1 - y0))
    }

    /// Masque 8 bits à la taille du document (255 = sélectionné), anticrénelé.
    pub fn mask(&self, w: u32, h: u32) -> Vec<u8> {
        let mut out = vec![0u8; (w * h) as usize];
        if let SelShape::Mask {
            width,
            height,
            data,
        } = &self.shape
        {
            for y in 0..h.min(*height) {
                let n = w.min(*width) as usize;
                let start = (y * w) as usize;
                let src = (y * width) as usize;
                if src + n <= data.len() {
                    out[start..start + n].copy_from_slice(&data[src..src + n]);
                }
            }
        } else if let Some(mut mask) = tiny_skia::Mask::new(w, h) {
            let path = match &self.shape {
                SelShape::Rect { x, y, w, h } => tiny_skia::Rect::from_xywh(*x, *y, *w, *h)
                    .map(tiny_skia::PathBuilder::from_rect),
                SelShape::Ellipse { x, y, w, h } => tiny_skia::Rect::from_xywh(*x, *y, *w, *h)
                    .and_then(tiny_skia::PathBuilder::from_oval),
                SelShape::Polygon { points } => {
                    let mut pb = tiny_skia::PathBuilder::new();
                    for (i, p) in points.iter().enumerate() {
                        if i == 0 {
                            pb.move_to(p[0], p[1]);
                        } else {
                            pb.line_to(p[0], p[1]);
                        }
                    }
                    pb.close();
                    pb.finish()
                }
                SelShape::Mask { .. } => unreachable!(),
            };
            if let Some(path) = path {
                mask.fill_path(
                    &path,
                    tiny_skia::FillRule::EvenOdd,
                    true,
                    tiny_skia::Transform::identity(),
                );
            }
            out.copy_from_slice(mask.data());
        }
        if self.inverted {
            out.iter_mut().for_each(|v| *v = 255 - *v);
        }
        out
    }
}

#[derive(Clone)]
pub struct Document {
    pub text_styles: Vec<crate::text_styles::TextStyle>,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub selection: Option<Selection>,
    pub selections: BTreeMap<String, Selection>,
    pub name: String,
    pub indexed: Option<crate::indexed::IndexedImage>,
    pub animation: Option<crate::animation::Animation>,
    pub metadata: crate::metadata::Metadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerInfo {
    pub locks: crate::layer_tools::Locks,
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub effects: crate::effects::LayerEffects,
    pub x: i32,
    pub y: i32,
    pub version: u64,
    pub kind: &'static str,
    pub width: u32,
    pub height: u32,
    pub shapes: Option<Vec<Shape>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocInfo {
    pub revision: u64,
    pub text_styles: Vec<crate::text_styles::TextStyle>,
    pub font_notices: Vec<String>,
    pub open: bool,
    pub width: u32,
    pub height: u32,
    pub name: String,
    pub path: Option<String>,
    pub dirty: bool,
    pub layers: Vec<LayerInfo>,
    pub selection: Option<Selection>,
    pub saved_selections: Vec<String>,
    pub indexed: Option<crate::indexed::IndexedInfo>,
    pub animation: Option<crate::animation::AnimationInfo>,
    pub metadata: crate::metadata::MetadataInfo,
    pub history: Vec<String>,
    pub history_index: usize,
    /// Calque à rendre actif après l'opération (création, duplication…).
    pub active_hint: Option<u64>,
}

const HISTORY_BYTES: usize = 256 * 1024 * 1024;
fn next_revision() -> u64 {
    static REV: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    REV.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// État de l'éditeur : document courant + historique d'instantanés.
/// Les pixels sont partagés par `Arc`, donc un instantané ne coûte que les calques modifiés.
#[derive(Clone, Default)]
pub struct Editor {
    pub doc: Option<Document>,
    pub path: Option<String>,
    pub dirty: bool,
    history: Vec<(String, Document, u64)>,
    index: usize,
    counter: u64,
    saved_revision: Option<u64>,
    pub active_hint: Option<u64>,
}

impl Editor {
    pub fn revision(&self) -> u64 {
        self.history.get(self.index).map_or(0, |entry| entry.2)
    }
    pub fn next_id(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    /// Adopte un travail isolé en une seule étape annulable.
    pub fn adopt(&mut self, staged: Editor, label: &str) {
        self.counter = self.counter.max(staged.counter);
        self.doc = staged.doc;
        self.active_hint = staged.active_hint;
        self.commit(label);
    }

    pub fn doc(&self) -> Result<&Document, String> {
        self.doc
            .as_ref()
            .ok_or_else(|| "Aucun document ouvert".to_string())
    }

    pub fn doc_mut(&mut self) -> Result<&mut Document, String> {
        self.doc
            .as_mut()
            .ok_or_else(|| "Aucun document ouvert".to_string())
    }

    /// Remplace le document (nouveau / ouvrir) et réinitialise l'historique.
    pub fn load(&mut self, mut doc: Document, path: Option<String>, label: &str) {
        if let Some(animation) = doc.animation.as_mut() {
            animation.id = self.next_id();
        }
        for layer in &doc.layers {
            self.counter = self.counter.max(layer.id).max(layer.version);
            if let Content::Vector(shapes) = &layer.content {
                self.counter = self.counter.max(crate::groups::maximum_id(shapes));
            }
        }
        let revision = next_revision();
        self.saved_revision = Some(revision);
        self.active_hint = doc.layers.last().map(|l| l.id);
        self.history = vec![(label.to_string(), doc.clone(), revision)];
        self.index = 0;
        self.doc = Some(doc);
        self.path = path;
        self.dirty = false;
    }

    pub fn close(&mut self) {
        self.doc = None;
        self.history.clear();
        self.index = 0;
        self.path = None;
        self.dirty = false;
        self.saved_revision = None;
        self.active_hint = None;
    }

    pub fn new_raster_layer(&mut self, name: &str, img: RgbaImage, x: i32, y: i32) -> Layer {
        let id = self.next_id();
        let version = self.next_id();
        Layer {
            locks: Default::default(),
            id,
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            effects: Default::default(),
            x,
            y,
            version,
            content: Content::Raster(Arc::new(img)),
        }
    }

    pub fn new_vector_layer(&mut self, name: &str, mut shapes: Vec<Shape>) -> Layer {
        crate::groups::assign_ids(&mut shapes, &mut || self.next_id());
        let id = self.next_id();
        let version = self.next_id();
        Layer {
            locks: Default::default(),
            id,
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            effects: Default::default(),
            x: 0,
            y: 0,
            version,
            content: Content::Vector(shapes),
        }
    }

    pub fn layer_index(&self, id: u64) -> Result<usize, String> {
        self.doc()?
            .layers
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| format!("Calque {id} introuvable"))
    }

    /// Marque un calque comme modifié (nouvelle version de contenu).
    pub fn touch(&mut self, idx: usize) {
        let v = self.next_id();
        if let Some(doc) = self.doc.as_mut() {
            doc.layers[idx].version = v;
        }
    }

    /// Enregistre l'état courant dans l'historique.
    pub fn refresh_text_paths(&mut self) -> Result<(), String> {
        if let Some(doc) = &mut self.doc {
            crate::text_styles::validate(&doc.text_styles)?;
            crate::text_path::refresh_document(doc)?;
            if let Some(entry) = self.history.get_mut(self.index) {
                entry.1 = doc.clone();
            }
        }
        Ok(())
    }

    /// Enregistre l'état courant dans l'historique.
    pub fn commit(&mut self, label: &str) {
        let Some(doc) = self.doc.clone() else { return };
        self.history.truncate(self.index + 1);
        let revision = next_revision();
        self.history.push((label.to_string(), doc, revision));
        while self.history.len() > 1 && self.history_bytes() > HISTORY_BYTES {
            self.history.remove(0);
        }
        self.index = self.history.len() - 1;
        self.dirty = true;
    }

    pub fn jump(&mut self, index: usize) -> Result<(), String> {
        let (_, doc, revision) = self
            .history
            .get(index)
            .ok_or("Étape d'historique invalide")?;
        let revision = *revision;
        let doc = doc.clone();
        self.doc = Some(doc);
        self.index = index;
        self.dirty = self.saved_revision != Some(revision);
        Ok(())
    }

    pub fn mark_saved(&mut self, path: String) {
        self.path = Some(path);
        self.saved_revision = self.history.get(self.index).map(|h| h.2);
        self.dirty = false;
    }

    pub(crate) fn history_bytes(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        let mut bytes = 0usize;
        for (_, doc, _) in &self.history {
            bytes = bytes.saturating_add(std::mem::size_of::<Document>());
            bytes =
                bytes.saturating_add(serde_json::to_vec(&doc.text_styles).map_or(0, |v| v.len()));
            if seen.insert(Arc::as_ptr(&doc.metadata.exif) as usize) {
                bytes = bytes.saturating_add(doc.metadata.exif.len());
            }
            if let Some(indexed) = &doc.indexed {
                if seen.insert(Arc::as_ptr(&indexed.indices) as usize) {
                    bytes = bytes.saturating_add(indexed.indices.len());
                }
            }
            if let Some(animation) = &doc.animation {
                for frame in animation.frames.iter() {
                    if seen.insert(Arc::as_ptr(&frame.image) as usize) {
                        bytes = bytes.saturating_add(frame.image.as_raw().len());
                    }
                }
            }
            for selection in doc.selection.iter().chain(doc.selections.values()) {
                match &selection.shape {
                    SelShape::Mask { data, .. } if seen.insert(Arc::as_ptr(data) as usize) => {
                        bytes = bytes.saturating_add(data.len())
                    }
                    SelShape::Polygon { points } => bytes = bytes.saturating_add(points.len() * 8),
                    _ => (),
                }
            }
            for layer in &doc.layers {
                bytes = bytes.saturating_add(layer.name.len() + std::mem::size_of::<Layer>());
                match &layer.content {
                    Content::Raster(img) if seen.insert(Arc::as_ptr(img) as usize) => {
                        bytes = bytes.saturating_add(img.as_raw().len())
                    }
                    Content::Vector(shapes) => {
                        bytes =
                            bytes.saturating_add(serde_json::to_vec(shapes).map_or(0, |v| v.len()))
                    }
                    _ => (),
                }
            }
        }
        bytes
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if self.index == 0 {
            return Ok(());
        }
        self.jump(self.index - 1)
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if self.index + 1 >= self.history.len() {
            return Ok(());
        }
        self.jump(self.index + 1)
    }

    pub fn info(&mut self) -> DocInfo {
        let active_hint = self.active_hint.take();
        let Some(doc) = &self.doc else {
            return DocInfo {
                revision: 0,
                text_styles: vec![],
                font_notices: vec![],
                open: false,
                width: 0,
                height: 0,
                name: String::new(),
                path: None,
                dirty: false,
                layers: vec![],
                selection: None,
                saved_selections: vec![],
                indexed: None,
                animation: None,
                metadata: Default::default(),
                history: vec![],
                history_index: 0,
                active_hint: None,
            };
        };
        let layers = doc
            .layers
            .iter()
            .map(|l| {
                let (kind, width, height, shapes) = match &l.content {
                    Content::Raster(img) => ("raster", img.width(), img.height(), None),
                    Content::Vector(s) => ("vector", doc.width, doc.height, Some(s.clone())),
                };
                LayerInfo {
                    locks: l.locks,
                    id: l.id,
                    name: l.name.clone(),
                    visible: l.visible,
                    opacity: l.opacity,
                    blend: l.blend,
                    effects: l.effects.clone(),
                    x: l.x,
                    y: l.y,
                    version: l.version,
                    kind,
                    width,
                    height,
                    shapes,
                }
            })
            .collect();
        DocInfo {
            revision: self.revision(),
            text_styles: doc.text_styles.clone(),
            font_notices: crate::fonts::notices(doc),
            open: true,
            width: doc.width,
            height: doc.height,
            name: doc.name.clone(),
            path: self.path.clone(),
            dirty: self.dirty,
            layers,
            selection: doc.selection.clone(),
            saved_selections: doc.selections.keys().cloned().collect(),
            indexed: doc.indexed.as_ref().map(|i| i.info()),
            animation: doc.animation.as_ref().map(|a| a.info()),
            metadata: doc.metadata.info(),
            history: self.history.iter().map(|(l, _, _)| l.clone()).collect(),
            history_index: self.index,
            active_hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> Editor {
        let mut e = Editor::default();
        let layer = e.new_raster_layer("Fond", RgbaImage::new(2, 2), 0, 0);
        e.load(
            Document {
                text_styles: Vec::new(),
                width: 2,
                height: 2,
                layers: vec![layer],
                selection: None,
                name: "Test".into(),
                selections: Default::default(),
                metadata: Default::default(),
                indexed: None,
                animation: None,
            },
            None,
            "Nouveau",
        );
        e
    }
    #[test]
    fn save_undo_redo_and_branch_track_dirty_state() {
        let mut e = editor();
        e.doc_mut().unwrap().layers[0].name = "A".into();
        e.commit("A");
        e.mark_saved("test.prisme".into());
        e.undo().unwrap();
        assert!(e.dirty);
        e.redo().unwrap();
        assert!(!e.dirty);
        e.undo().unwrap();
        e.doc_mut().unwrap().layers[0].name = "B".into();
        e.commit("B");
        e.redo().unwrap();
        assert!(e.dirty);
        assert_eq!(e.info().history.len(), 2);
    }
    #[test]
    fn unchanged_pixels_are_shared_between_snapshots() {
        let mut e = editor();
        let before = e.doc().unwrap().layers[0].raster().unwrap().clone();
        e.commit("Métadonnées");
        assert!(Arc::ptr_eq(
            &before,
            e.history[1].1.layers[0].raster().unwrap()
        ));
        for _ in 0..80 {
            e.commit("Étape");
        }
        assert_eq!(e.info().history.len(), 82);
    }
}
