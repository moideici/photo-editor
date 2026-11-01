//! Catalogue et axes réels des polices utilisées par le moteur de rendu.
use crate::document::{Content, Document, Shape, ShapeKind};
use resvg::usvg::fontdb;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontOptions {
    #[serde(default)]
    pub axes: BTreeMap<String, f32>,
    #[serde(default)]
    pub optical_sizing: OpticalSizing,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpticalSizing {
    #[default]
    Auto,
    None,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Axis {
    pub tag: String,
    pub name: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
    pub hidden: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub requested: String,
    pub available: bool,
    pub resolved: Option<String>,
    pub axes: Vec<Axis>,
}
pub fn database() -> Arc<fontdb::Database> {
    static DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        #[cfg(test)]
        db.load_font_data(include_bytes!("fixtures/prisme-variable.ttf").to_vec());
        Arc::new(db)
    })
    .clone()
}
pub fn catalog() -> Vec<String> {
    let db = database();
    db.faces()
        .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
pub fn info(family: &str, bold: bool, italic: bool) -> Info {
    static CACHE: OnceLock<Mutex<HashMap<(String, bool, bool), Info>>> = OnceLock::new();
    let key = (family.to_owned(), bold, italic);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return v.clone();
    }
    let db = database();
    let parsed = svgtypes::parse_font_families(family).unwrap_or_default();
    let families = parsed
        .iter()
        .map(|f| match f {
            svgtypes::FontFamily::Named(n) => fontdb::Family::Name(n),
            svgtypes::FontFamily::Serif => fontdb::Family::Serif,
            svgtypes::FontFamily::SansSerif => fontdb::Family::SansSerif,
            svgtypes::FontFamily::Cursive => fontdb::Family::Cursive,
            svgtypes::FontFamily::Fantasy => fontdb::Family::Fantasy,
            svgtypes::FontFamily::Monospace => fontdb::Family::Monospace,
        })
        .collect::<Vec<_>>();
    let query = |families| {
        db.query(&fontdb::Query {
            families,
            weight: fontdb::Weight(if bold { 700 } else { 400 }),
            style: if italic {
                fontdb::Style::Italic
            } else {
                fontdb::Style::Normal
            },
            stretch: fontdb::Stretch::Normal,
        })
    };
    let exact = query(&families);
    let selected = exact.or_else(|| query(&[fontdb::Family::Serif]));
    let axes = exact
        .and_then(|id| {
            db.with_face_data(id, |data, index| {
                let Ok(face) = ttf_parser::Face::parse(data, index) else {
                    return vec![];
                };
                face.variation_axes()
                    .into_iter()
                    .map(|a| Axis {
                        tag: String::from_utf8_lossy(&a.tag.to_bytes()).into(),
                        name: face
                            .names()
                            .into_iter()
                            .filter(|n| n.name_id == a.name_id)
                            .find_map(|n| n.to_string())
                            .unwrap_or_else(|| a.tag.to_string()),
                        min: a.min_value,
                        default: a.def_value,
                        max: a.max_value,
                        hidden: a.hidden,
                    })
                    .collect()
            })
        })
        .unwrap_or_default();
    let value = Info {
        requested: family.into(),
        available: exact.is_some(),
        resolved: selected
            .and_then(|id| db.face(id))
            .and_then(|f| f.families.first())
            .map(|p| p.0.clone()),
        axes,
    };
    let mut entries = cache.lock().unwrap();
    if entries.len() >= 4096 {
        entries.clear();
    }
    entries.insert(key, value.clone());
    value
}
pub fn shape_info(s: &Shape) -> Option<Info> {
    if let ShapeKind::Text {
        font_family,
        bold,
        italic,
        ..
    } = &s.kind
    {
        Some(info(font_family, *bold, *italic))
    } else {
        None
    }
}
pub fn validate(s: &Shape) -> Result<(), String> {
    if let Some(p) = &s.font_options {
        if !matches!(s.kind, ShapeKind::Text { .. }) {
            return Err("Les axes de police sont réservés au texte".into());
        }
        if p.axes.len() > 32
            || p.axes.iter().any(|(tag, v)| {
                tag.len() != 4
                    || !tag.bytes().all(|c| c.is_ascii_alphanumeric())
                    || !v.is_finite()
                    || v.abs() > 1e6
            })
        {
            return Err("Axes de police invalides : 32 tags de quatre lettres/chiffres et valeurs finies bornées".into());
        }
    }
    if let ShapeKind::Text { font_family, .. } = &s.kind {
        if font_family.trim().is_empty() || font_family.len() > 256 || font_family.contains('\0') {
            return Err("Nom de police vide ou supérieur à 256 octets".into());
        }
    }
    Ok(())
}
pub fn axis_issues(s: &Shape) -> Vec<String> {
    let Some(p) = &s.font_options else {
        return vec![];
    };
    let Some(f) = shape_info(s) else {
        return vec![];
    };
    if !f.available {
        return vec![];
    }
    p.axes.iter().filter_map(|(tag,v)|match f.axes.iter().find(|a|&a.tag==tag){None=>Some(format!("Axe {tag} absent de « {} » ; il est ignoré par le rendu. Réinitialisez les axes ou choisissez une autre police.",f.requested)),Some(a) if *v<a.min||*v>a.max=>Some(format!("Axe {tag} hors de la plage {}–{} de « {} » ; le rendu le borne sans changer la valeur du projet.",a.min,a.max,f.requested)),_=>None}).collect()
}
pub fn validate_controls(s: &Shape) -> Result<(), String> {
    validate(s)?;
    let issues = axis_issues(s);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("\n"))
    }
}
pub fn attributes(s: &Shape) -> String {
    let Some(p) = &s.font_options else {
        return String::new();
    };
    let settings = if p.axes.is_empty() {
        "normal".into()
    } else {
        p.axes
            .iter()
            .map(|(tag, v)| format!("'{tag}' {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "style=\"font-variation-settings:{};font-optical-sizing:{}\"",
        crate::vector::escape(&settings),
        match p.optical_sizing {
            OpticalSizing::Auto => "auto",
            OpticalSizing::None => "none",
        }
    )
}
pub fn needs_outlines(s: &Shape) -> bool {
    shape_info(s).is_some_and(|f| {
        !f.axes.is_empty() || s.font_options.as_ref().is_some_and(|p| !p.axes.is_empty())
    })
}
pub fn notices(doc: &Document) -> Vec<String> {
    fn shapes(list: &[Shape], messages: &mut BTreeSet<String>, depth: usize) {
        if depth > 32 {
            return;
        }
        for s in list {
            if let Some(f) = shape_info(s) {
                if !f.available {
                    messages.insert(format!("Police absente « {} ». Aperçu avec « {} » ; choisissez explicitement un remplacement dans le panneau Police variable et styles.",f.requested,f.resolved.as_deref().unwrap_or("aucune police disponible")));
                }
                for message in axis_issues(s) {
                    messages.insert(message);
                }
            }
            if let ShapeKind::Group { children, .. } = &s.kind {
                shapes(children, messages, depth + 1);
            }
            for p in [s.fill_pattern.as_ref(), s.stroke_pattern.as_ref()]
                .into_iter()
                .flatten()
            {
                shapes(&p.children, messages, depth + 1);
            }
            fn mask(
                m: &crate::svg_mask::VectorMask,
                messages: &mut BTreeSet<String>,
                depth: usize,
            ) {
                shapes(&m.children, messages, depth + 1);
                if depth < 32 {
                    if let Some(m) = &m.intersect {
                        mask(m, messages, depth + 1);
                    }
                }
            }
            if let Some(m) = &s.mask {
                mask(m, messages, depth);
            }
            if let Some(f) = &s.filters {
                for p in f.filters.iter().flat_map(|f| &f.primitives) {
                    if let Some(r) = &p.resource {
                        shapes(&r.children, messages, depth + 1);
                    }
                }
            }
        }
    }
    let mut messages = BTreeSet::new();
    for l in &doc.layers {
        if let Content::Vector(s) = &l.content {
            shapes(s, &mut messages, 0);
        }
    }
    messages.into_iter().take(64).collect()
}
