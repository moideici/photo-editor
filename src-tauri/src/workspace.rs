//! Independent editor histories per tab, bounded recovery and recent files.
use crate::{
    document::Editor,
    operations::{self, Operation},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
pub type Shared = Arc<Mutex<Workspace>>;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub zoom: f32,
    pub active: Option<u64>,
    pub selected: Option<u64>,
    pub pan: [f32; 2],
}
#[derive(Clone)]
pub struct Tab {
    pub id: u64,
    pub editor: Option<Editor>,
    pub view: View,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    pub path: String,
    pub thumbnail: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    pub file: String,
    pub name: String,
    pub path: Option<String>,
    pub revision: u64,
}
#[derive(Default)]
pub struct Workspace {
    pub root: PathBuf,
    pub tabs: Vec<Tab>,
    pub active: u64,
    pub counter: u64,
    pub recent: Vec<Recent>,
    owned_recovery: std::collections::HashSet<String>,
    session: u128,
}
impl Workspace {
    pub fn init(&mut self, root: PathBuf) {
        self.session = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        self.root = root;
        self.recent = std::fs::read(self.root.join("recent.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<Recent>>(&b).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.path.len() < 32768 && r.thumbnail.len() < 300_000)
            .take(20)
            .collect();
    }
    fn ensure(&mut self) {
        if self.tabs.is_empty() {
            self.counter += 1;
            self.active = self.counter;
            self.tabs.push(Tab {
                id: self.active,
                editor: None,
                view: View {
                    zoom: 1.,
                    ..Default::default()
                },
            });
        }
    }
    pub fn status(&mut self, e: &Editor) -> serde_json::Value {
        self.ensure();
        let tabs=self.tabs.iter().map(|t|{let editor=if t.id==self.active{Some(e)}else{t.editor.as_ref()};let d=editor.and_then(|e|e.doc.as_ref());serde_json::json!({"id":t.id,"name":d.map(|d|d.name.as_str()).unwrap_or("Atelier"),"dirty":editor.is_some_and(|e|e.dirty),"path":editor.and_then(|e|e.path.as_ref()),"open":d.is_some()})}).collect::<Vec<_>>();
        serde_json::json!({"active":self.active,"tabs":tabs,"recent":self.recent.iter().map(|r|serde_json::json!({"path":r.path,"thumbnail":r.thumbnail,"missing":!Path::new(&r.path).is_file()})).collect::<Vec<_>>(),"recoveries":self.recoveries(),"view":self.tabs.iter().find(|t|t.id==self.active).map(|t|&t.view)})
    }
    pub fn view(&mut self, v: View) -> Result<(), String> {
        if !v.zoom.is_finite()
            || v.zoom < 0.05
            || v.zoom > 16.
            || v.pan.iter().any(|n| !n.is_finite() || n.abs() > 1e6)
        {
            return Err("Vue invalide".into());
        }
        self.ensure();
        self.tabs
            .iter_mut()
            .find(|t| t.id == self.active)
            .unwrap()
            .view = v;
        Ok(())
    }
    pub fn switch(&mut self, e: &mut Editor, id: u64) -> Result<(), String> {
        self.ensure();
        if id == self.active {
            return Ok(());
        }
        let to = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or("Onglet absent")?;
        let next = self.tabs[to].editor.take().ok_or("Document indisponible")?;
        let old = self.tabs.iter_mut().find(|t| t.id == self.active).unwrap();
        old.editor = Some(std::mem::replace(e, next));
        self.active = id;
        Ok(())
    }
    pub fn open(&mut self, e: &mut Editor, op: Operation) -> Result<(), String> {
        self.ensure();
        if !matches!(
            op,
            Operation::Open { .. } | Operation::New { .. } | Operation::OpenSvgEditable { .. }
        ) {
            return Err("Seuls Ouvrir et Nouveau créent un onglet".into());
        }
        let mut next = Editor::default();
        operations::execute(&mut next, op)?;
        let bytes = self
            .tabs
            .iter()
            .filter_map(|t| t.editor.as_ref())
            .map(Editor::history_bytes)
            .sum::<usize>()
            + e.history_bytes()
            + next.history_bytes();
        if bytes > 768 * 1024 * 1024 {
            return Err(
                "Les onglets dépasseraient 768 Mio d’historique ; fermez un document".into(),
            );
        }
        if e.doc.is_none() {
            *e = next;
            return Ok(());
        }
        if self.tabs.len() >= 8 {
            return Err("Huit documents ouverts maximum ; fermez un onglet".into());
        }
        self.tabs
            .iter_mut()
            .find(|t| t.id == self.active)
            .unwrap()
            .editor = Some(std::mem::replace(e, next));
        self.counter += 1;
        self.active = self.counter;
        self.tabs.push(Tab {
            id: self.active,
            editor: None,
            view: View {
                zoom: 1.,
                ..Default::default()
            },
        });
        Ok(())
    }
    pub fn close(&mut self, e: &mut Editor, discard: bool) -> Result<(), String> {
        self.ensure();
        if e.dirty && !discard {
            return Err(
                "Document modifié : enregistrez ou confirmez l’abandon avant de fermer".into(),
            );
        }
        let id = self.active;
        self.tabs.retain(|t| t.id != id);
        if let Some(last) = self.tabs.last_mut() {
            self.active = last.id;
            *e = last.editor.take().ok_or("Onglet indisponible")?;
        } else {
            e.close();
            self.ensure();
        }
        Ok(())
    }
    pub fn remember(&mut self, path: String, e: &Editor) -> Result<(), String> {
        if path.len() > 32768 || !Path::new(&path).is_file() {
            return Err("Fichier récent introuvable".into());
        }
        let thumbnail = if let Ok(d) = e.doc() {
            let mut combined = e.clone();
            let image = crate::vector::flatten(d)?;
            combined.doc_mut()?.layers = vec![combined.new_raster_layer("Aperçu", image, 0, 0)];
            crate::layer_tools::thumbnail(&combined.doc()?.layers[0], d.width, d.height, 80)?
        } else {
            String::new()
        };
        self.recent.retain(|r| r.path != path);
        self.recent.insert(0, Recent { path, thumbnail });
        self.recent.truncate(20);
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        crate::io::atomic_write(
            self.root
                .join("recent.json")
                .to_str()
                .ok_or("Chemin invalide")?,
            &serde_json::to_vec(&self.recent).map_err(|e| e.to_string())?,
        )
    }
    pub fn recoveries(&self) -> Vec<Recovery> {
        std::fs::read(self.root.join("recovery.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<Recovery>>(&b).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|r| safe_name(&r.file) && r.name.len() < 4096)
            .take(32)
            .collect()
    }
    pub fn checkpoint(&mut self, e: &Editor) -> Result<(), String> {
        self.ensure();
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let old = self.recoveries();
        let mut entries = old
            .iter()
            .filter(|r| !self.owned_recovery.contains(&r.file))
            .cloned()
            .collect::<Vec<_>>();
        let mut total = 0usize;
        for t in &self.tabs {
            let ed = if t.id == self.active {
                e
            } else {
                t.editor.as_ref().ok_or("Onglet indisponible")?
            };
            if ed.dirty {
                if let Some(d) = &ed.doc {
                    let file = format!(
                        "recovery-{}-{}-{}.prisme",
                        self.session,
                        t.id,
                        ed.revision()
                    );
                    let bytes = crate::io::project_bytes(d)?;
                    total += bytes.len();
                    if total > 512 * 1024 * 1024 {
                        return Err(
                            "Récupération supérieure à 512 Mio ; enregistrez les documents".into(),
                        );
                    }
                    crate::io::atomic_write(
                        self.root.join(&file).to_str().ok_or("Chemin invalide")?,
                        &bytes,
                    )?;
                    entries.push(Recovery {
                        file,
                        name: d.name.clone(),
                        path: ed.path.clone(),
                        revision: ed.revision(),
                    });
                }
            }
        }
        if entries.len() > 32 {
            return Err(
                "Plus de 32 récupérations : restaurez ou retirez les anciennes avant de continuer"
                    .into(),
            );
        }
        crate::io::atomic_write(
            self.root
                .join("recovery.json")
                .to_str()
                .ok_or("Chemin invalide")?,
            &serde_json::to_vec(&entries).map_err(|e| e.to_string())?,
        )?;
        for r in old {
            if self.owned_recovery.contains(&r.file) && !entries.iter().any(|n| n.file == r.file) {
                let _ = std::fs::remove_file(self.root.join(r.file));
            }
        }
        self.owned_recovery = entries
            .iter()
            .filter(|r| r.file.starts_with(&format!("recovery-{}-", self.session)))
            .map(|r| r.file.clone())
            .collect();
        Ok(())
    }
    pub fn restore(&mut self, e: &mut Editor, file: &str) -> Result<(), String> {
        if !safe_name(file) || !self.recoveries().iter().any(|r| r.file == file) {
            return Err("Récupération inconnue".into());
        }
        let p = self.root.join(file);
        self.open(
            e,
            Operation::Open {
                path: p.to_str().ok_or("Chemin invalide")?.into(),
            },
        )?;
        e.path = None;
        e.commit("Restaurer après interruption");
        Ok(())
    }
    pub fn forget_recovery(&self, file: &str) -> Result<(), String> {
        if !safe_name(file) {
            return Err("Nom de récupération invalide".into());
        }
        let mut entries = self.recoveries();
        if !entries.iter().any(|r| r.file == file) {
            return Err("Récupération inconnue".into());
        }
        entries.retain(|r| r.file != file);
        crate::io::atomic_write(
            self.root
                .join("recovery.json")
                .to_str()
                .ok_or("Chemin invalide")?,
            &serde_json::to_vec(&entries).unwrap(),
        )?;
        std::fs::remove_file(self.root.join(file)).map_err(|e| e.to_string())
    }
}
fn safe_name(s: &str) -> bool {
    s.starts_with("recovery-")
        && s.ends_with(".prisme")
        && s.len() < 100
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-.".contains(&c))
}
