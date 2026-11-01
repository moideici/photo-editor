use crate::document::{Content, Editor, Layer};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Locks {
    pub position: bool,
    pub pixels: bool,
    pub alpha: bool,
}
pub fn thumbnail(layer: &Layer, width: u32, height: u32, size: u32) -> Result<String, String> {
    use base64::Engine;
    let size = size.clamp(32, 256);
    crate::io::dimensions(width, height)?;
    let mut layer = layer.clone();
    layer.visible = true;
    layer.blend = Default::default();
    // No full-canvas blank bitmap or history is needed for an isolated thumbnail.
    let document = crate::document::Document {
        text_styles: vec![],
        width,
        height,
        layers: vec![layer],
        selection: None,
        selections: Default::default(),
        name: String::new(),
        indexed: None,
        animation: None,
        metadata: Default::default(),
    };
    let svg = crate::vector::export_svg(&document)?;
    let scale = size as f32 / width.max(height) as f32;
    let outer = crate::vector::svg_document(
        size,
        size,
        &format!(
            "<g transform=\"translate({} {}) scale({scale})\">{svg}</g>",
            (size as f32 - width as f32 * scale) / 2.,
            (size as f32 - height as f32 * scale) / 2.
        ),
    );
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(crate::io::png_bytes(
            &crate::vector::render_svg(&outer, size, size)?
        )?)
    ))
}
pub fn batch(
    e: &mut Editor,
    ids: Vec<u64>,
    kind: String,
    value: Option<String>,
    locks: Option<Locks>,
) -> Result<(), String> {
    if ids.is_empty()
        || ids.len() > 256
        || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
    {
        return Err("Choisissez des calques distincts".into());
    }
    let mut indices = ids
        .iter()
        .map(|id| e.layer_index(*id))
        .collect::<Result<Vec<_>, _>>()?;
    indices.sort();
    if e.doc()?.indexed.is_some() || e.doc()?.animation.is_some() {
        return Err("Ces opérations multiples nécessitent un document RVB fixe".into());
    }
    match kind.as_str() {
        "locks" => {
            let locks = locks.ok_or("Verrouillages manquants")?;
            if locks.alpha
                && indices
                    .iter()
                    .any(|&i| e.doc().unwrap().layers[i].raster().is_none())
            {
                return Err("Le verrou alpha est réservé aux calques bitmap ; utilisez Pixels pour la géométrie vectorielle".into());
            }
            for &i in &indices {
                e.doc_mut()?.layers[i].locks = locks;
                e.touch(i);
            }
        }
        "show" | "hide" => {
            for &i in &indices {
                e.doc_mut()?.layers[i].visible = kind == "show";
                e.touch(i);
            }
        }
        "rename" => {
            let name = value.ok_or("Nom manquant")?;
            if name.trim().is_empty() || name.len() > 256 {
                return Err("Nom de calque vide ou trop long".into());
            }
            for &i in &indices {
                e.doc_mut()?.layers[i].name = name.clone();
                e.touch(i);
            }
        }
        "delete" => {
            if indices.len() == e.doc()?.layers.len() {
                return Err("Conservez au moins un calque".into());
            }
            if indices
                .iter()
                .any(|&i| e.doc().unwrap().layers[i].locks != Locks::default())
            {
                return Err("Déverrouillez les calques avant de les supprimer".into());
            }
            for &i in indices.iter().rev() {
                e.doc_mut()?.layers.remove(i);
            }
        }
        "duplicate" => {
            if e.doc()?.layers.len() + indices.len() > 256 {
                return Err("256 calques maximum".into());
            }
            for &i in indices.iter().rev() {
                let mut l = e.doc()?.layers[i].clone();
                l.id = e.next_id();
                l.version = e.next_id();
                l.name.push_str(" — copie");
                if let Content::Vector(ss) = &mut l.content {
                    crate::groups::assign_ids(ss, &mut || e.next_id());
                }
                e.doc_mut()?.layers.insert(i + 1, l);
            }
        }
        _ => return Err("Opération multiple inconnue".into()),
    }
    // Property batches keep the user's active layer. Only deletion needs a fallback.
    e.active_hint = if kind == "delete" {
        e.doc()?.layers.last().map(|l| l.id)
    } else {
        None
    };
    e.commit("Modifier les calques sélectionnés");
    Ok(())
}
pub fn guard(e: &Editor, op: &crate::operations::Operation) -> Result<(), String> {
    use crate::operations::Operation as O;
    let check = |id: u64, position: bool, pixels: bool, remove: bool| -> Result<(), String> {
        let l = &e.doc()?.layers[e.layer_index(id)?];
        if (position && l.locks.position)
            || (pixels && l.locks.pixels)
            || (remove && l.locks != Locks::default())
        {
            Err(format!(
                "Calque « {} » verrouillé : déverrouillez la position ou les pixels dans Calques.",
                l.name
            ))
        } else {
            Ok(())
        }
    };
    match op {
        O::Layer { id, x, y, .. } => check(*id, x.is_some() || y.is_some(), false, false),
        O::Shape { id, .. }
        | O::DeleteShape { id, .. }
        | O::GroupShapes { id, .. }
        | O::UngroupShapes { id, .. }
        | O::ToPath { id, .. }
        | O::Boolean { id, .. }
        | O::DecomposeSvg { id, .. }
        | O::TextStyleApply { id, .. } => check(*id, true, true, false),
        O::Stroke { id, .. }
        | O::Retouch { id, .. }
        | O::Filter { id, .. }
        | O::Fill { id, .. }
        | O::Gradient { id, .. }
        | O::Asset { id, .. }
        | O::Transparency { id, .. }
        | O::Plugin { id, .. }
        | O::EightBf { id, .. }
        | O::Rasterize { id }
        | O::Effects { id, .. } => check(*id, false, true, false),
        O::Pfl { id: Some(id), .. } => check(*id, false, true, false),
        O::DeleteLayer { id } | O::Reorder { id, .. } => check(*id, true, false, true),
        O::MergeDown { id } => {
            let i = e.layer_index(*id)?;
            check(*id, true, true, true)?;
            if i > 0 {
                check(e.doc()?.layers[i - 1].id, true, true, true)?;
            }
            Ok(())
        }
        O::MergePrefix { id } => {
            let i = e.layer_index(*id)?;
            for l in &e.doc()?.layers[..=i] {
                check(l.id, true, true, true)?;
            }
            Ok(())
        }
        O::Flatten
        | O::Resize { .. }
        | O::Canvas { .. }
        | O::Crop
        | O::Rotate { .. }
        | O::Flip { .. }
        | O::ConvertIndexed { .. }
        | O::PaletteColor { .. }
        | O::Pfl { id: None, .. } => {
            for l in &e.doc()?.layers {
                check(l.id, true, true, true)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
