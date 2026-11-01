//! Styles réutilisables par copie des attributs, sans contenu ni lien de tracé.
use crate::{
    document::{Document, Shape, ShapeKind},
    groups, io,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextStyle {
    pub name: String,
    pub template: Shape,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pack {
    version: u32,
    styles: Vec<TextStyle>,
}
impl TextStyle {
    pub fn capture(name: &str, s: &Shape) -> Result<Self, String> {
        let ShapeKind::Text {
            font_family,
            font_size,
            bold,
            italic,
            ..
        } = &s.kind
        else {
            return Err("Sélectionnez un texte pour enregistrer son style".into());
        };
        let mut t = crate::clip::base();
        t.kind = ShapeKind::Text {
            text: String::new(),
            font_family: font_family.clone(),
            font_size: *font_size,
            bold: *bold,
            italic: *italic,
        };
        t.w = font_size * 4.8;
        t.h = font_size * 1.3;
        t.font_options = s.font_options.clone();
        t.fill = s.fill.clone();
        t.stroke = s.stroke.clone();
        t.stroke_width = s.stroke_width;
        t.opacity = s.opacity;
        t.fill_gradient = s.fill_gradient.clone();
        t.stroke_gradient = s.stroke_gradient.clone();
        t.fill_pattern = s.fill_pattern.clone();
        t.stroke_pattern = s.stroke_pattern.clone();
        let style = Self {
            name: name.trim().into(),
            template: t,
        };
        style.validate()?;
        Ok(style)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.len() > 256 || self.name != self.name.trim() {
            return Err("Nom de style vide ou supérieur à 256 octets".into());
        }
        let t = &self.template;
        if !matches!(&t.kind,ShapeKind::Text{text,..} if text.is_empty())
            || t.text_path.is_some()
            || t.transform.is_some()
            || t.x != 0.
            || t.y != 0.
            || t.rotation != 0.
            || t.id != 0
            || t.svg_viewport
            || t.svg_unclipped
            || t.group_style.is_some()
            || t.filters.is_some()
            || t.mask.is_some()
            || t.clip.is_some()
        {
            return Err("Style invalide : attributs typographiques et peintures uniquement".into());
        }
        io::validate_shape_fields(t)
    }
    pub fn apply(&self, s: &mut Shape) -> Result<(), String> {
        self.validate()?;
        crate::fonts::validate_controls(&self.template)?;
        let ShapeKind::Text {
            font_family,
            font_size,
            bold,
            italic,
            ..
        } = &mut s.kind
        else {
            return Err("Un style de texte ne s’applique qu’à du texte".into());
        };
        let ShapeKind::Text {
            font_family: f,
            font_size: z,
            bold: b,
            italic: i,
            ..
        } = &self.template.kind
        else {
            unreachable!()
        };
        if s.text_path.is_none() {
            let ratio = z / *font_size;
            s.w *= ratio;
            s.h *= ratio;
        }
        *font_family = f.clone();
        *font_size = *z;
        *bold = *b;
        *italic = *i;
        s.font_options = self.template.font_options.clone();
        s.fill = self.template.fill.clone();
        s.stroke = self.template.stroke.clone();
        s.stroke_width = self.template.stroke_width;
        s.opacity = self.template.opacity;
        s.fill_gradient = self.template.fill_gradient.clone();
        s.stroke_gradient = self.template.stroke_gradient.clone();
        s.fill_pattern = self.template.fill_pattern.clone();
        s.stroke_pattern = self.template.stroke_pattern.clone();
        io::validate_shape_fields(s)
    }
}
pub fn validate(styles: &[TextStyle]) -> Result<(), String> {
    if styles.len() > 128 {
        return Err("Au plus 128 styles de texte par document".into());
    }
    let mut names = std::collections::HashSet::new();
    for s in styles {
        s.validate()?;
        if !names.insert(s.name.to_lowercase()) {
            return Err("Noms de styles dupliqués".into());
        }
    }
    Ok(())
}
pub fn save(doc: &mut Document, layer_id: u64, shape_id: u64, name: &str) -> Result<(), String> {
    let layer = doc
        .layers
        .iter()
        .find(|l| l.id == layer_id)
        .ok_or("Calque introuvable")?;
    let crate::document::Content::Vector(shapes) = &layer.content else {
        return Err("Calque vectoriel requis".into());
    };
    let style = TextStyle::capture(
        name,
        groups::find(shapes, shape_id).ok_or("Texte introuvable")?,
    )?;
    if let Some(old) = doc
        .text_styles
        .iter_mut()
        .find(|s| s.name.to_lowercase() == style.name.to_lowercase())
    {
        *old = style;
    } else {
        doc.text_styles.push(style);
    }
    validate(&doc.text_styles)
}
pub fn merge(target: &mut Vec<TextStyle>, incoming: Vec<TextStyle>) -> Result<(), String> {
    validate(&incoming)?;
    if target.len() + incoming.len() > 128 {
        return Err("L’import dépasserait 128 styles de texte".into());
    }
    for mut s in incoming {
        let original = s.name.clone();
        let mut index = 2;
        while target
            .iter()
            .any(|t| t.name.to_lowercase() == s.name.to_lowercase())
        {
            let suffix = format!(" (importé {index})");
            let mut name = original.clone();
            while name.len() + suffix.len() > 256 {
                name.pop();
            }
            s.name = name + suffix.as_str();
            index += 1;
        }
        target.push(s);
    }
    validate(target)
}
pub fn read(path: &str) -> Result<Vec<TextStyle>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
        return Err("Bibliothèque de styles supérieure à 4 Mio".into());
    }
    let p: Pack = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Bibliothèque de styles invalide : {e}"))?;
    if p.version != 1 {
        return Err("Version de bibliothèque de styles inconnue".into());
    }
    validate(&p.styles)?;
    Ok(p.styles)
}
pub fn write(path: &str, styles: &[TextStyle]) -> Result<(), String> {
    validate(styles)?;
    let data = serde_json::to_vec_pretty(&Pack {
        version: 1,
        styles: styles.to_vec(),
    })
    .map_err(|e| e.to_string())?;
    if data.len() > 4 * 1024 * 1024 {
        return Err("Bibliothèque de styles supérieure à 4 Mio".into());
    }
    io::atomic_write(path, &data)
}
