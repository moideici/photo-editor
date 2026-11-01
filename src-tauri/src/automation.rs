//! Actions déclaratives : aucune commande système ni script exécutable.
use crate::{
    document::Editor,
    io,
    operations::{self, Operation},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

const MAX_STEPS: usize = 512;
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub operation: Operation,
    /// 0 = calque du haut, 1 = celui juste au-dessous, etc.
    pub layer_from_top: Option<usize>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub version: u32,
    pub name: String,
    pub steps: Vec<Step>,
}
impl Default for Action {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Nouvelle action".into(),
            steps: vec![],
        }
    }
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recorder {
    pub recording: bool,
    pub action: Action,
}
#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Control {
    Start { name: String },
    Stop,
    Load { path: String },
    Save { path: String },
    Rename { name: String },
    Remove { index: usize },
    Clear,
}
pub fn control(rec: &mut Recorder, e: &Editor, control: Control) -> Result<(), String> {
    match control {
        Control::Start { name } => rec.start(e, name),
        Control::Stop => {
            rec.recording = false;
            Ok(())
        }
        Control::Load { path } => rec.read(&path),
        Control::Save { path } => rec.save(&path),
        Control::Rename { name } => {
            if name.len() > 256 {
                return Err("Nom trop long".into());
            }
            rec.action.name = name;
            Ok(())
        }
        Control::Remove { index } => {
            if rec.recording {
                return Err("Arrêtez l'enregistrement".into());
            }
            if index >= rec.action.steps.len() {
                return Err("Étape introuvable".into());
            }
            rec.action.steps.remove(index);
            Ok(())
        }
        Control::Clear => {
            if rec.recording {
                return Err("Arrêtez l'enregistrement".into());
            }
            rec.action = Action::default();
            Ok(())
        }
    }
}

fn target(op: &mut Operation) -> Option<&mut u64> {
    match op {
        Operation::DeleteLayer { id }
        | Operation::DuplicateLayer { id }
        | Operation::Layer { id, .. }
        | Operation::Reorder { id, .. }
        | Operation::MergeDown { id }
        | Operation::MergePrefix { id }
        | Operation::Rasterize { id }
        | Operation::Effects { id, .. }
        | Operation::Transparency { id, .. }
        | Operation::Shape { id, .. }
        | Operation::Stroke { id, .. }
        | Operation::Retouch { id, .. }
        | Operation::Asset { id, .. }
        | Operation::Fill { id, .. }
        | Operation::Gradient { id, .. }
        | Operation::Filter { id, .. }
        | Operation::Plugin { id, .. }
        | Operation::EightBf { id, .. } => Some(id),
        _ => None,
    }
}
fn recordable(op: &Operation) -> bool {
    matches!(
        op,
        Operation::AddLayer { .. }
            | Operation::DeleteLayer { .. }
            | Operation::DuplicateLayer { .. }
            | Operation::Layer { .. }
            | Operation::Reorder { .. }
            | Operation::MergeDown { .. }
            | Operation::MergePrefix { .. }
            | Operation::Flatten
            | Operation::Rasterize { .. }
            | Operation::Effects { .. }
            | Operation::ConvertIndexed { .. }
            | Operation::ConvertRgb
            | Operation::PaletteColor { .. }
            | Operation::Transparency { .. }
            | Operation::Stroke { .. }
            | Operation::Retouch { .. }
            | Operation::Asset { .. }
            | Operation::Fill { .. }
            | Operation::Gradient { .. }
            | Operation::Filter { .. }
            | Operation::Plugin { .. }
            | Operation::EightBf { .. }
            | Operation::Selection { .. }
            | Operation::SelectColor { .. }
            | Operation::ModifySelection { .. }
            | Operation::RememberSelection { .. }
            | Operation::RestoreSelection { .. }
            | Operation::ForgetSelection { .. }
            | Operation::InvertSelection
            | Operation::Resize { .. }
            | Operation::Canvas { .. }
            | Operation::Crop
            | Operation::Rotate { .. }
            | Operation::Flip { .. }
    ) || matches!(op,Operation::Shape{shape,..} if shape.id==0&&shape.text_path.as_ref().is_none_or(|p|p.source_id.is_none()))
}
impl Action {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.name.len() > 256 || self.steps.len() > MAX_STEPS {
            return Err(
                "Action invalide : version 1, nom de 256 octets et 512 étapes maximum".into(),
            );
        }
        for step in &self.steps {
            let mut op = step.operation.clone();
            if let Operation::EightBf { settings, .. } = &op {
                settings.validate()?;
                if settings.show_ui {
                    return Err(
                        "Une action 8BF doit conserver les paramètres sans rouvrir le dialogue"
                            .into(),
                    );
                }
            }
            if !recordable(&op) {
                return Err("Cette commande ne peut pas appartenir à une action".into());
            }
            if target(&mut op).is_some() != step.layer_from_top.is_some()
                || step.layer_from_top.is_some_and(|i| i >= 256)
            {
                return Err("Cible de calque invalide dans l'action".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_BYTES {
            return Err("Action trop volumineuse (16 Mio maximum)".into());
        }
        Ok(())
    }
}
impl Recorder {
    pub fn prepare(&self, e: &Editor, op: &Operation) -> Result<Option<Step>, String> {
        if !self.recording || matches!(op, Operation::Save { .. } | Operation::Export { .. }) {
            return Ok(None);
        }
        if !recordable(op) {
            return Err("Arrêtez l'enregistrement avant d'ouvrir un fichier, d'importer, de modifier une forme existante ou de parcourir l'historique.".into());
        }
        let mut op = op.clone();
        let layer_from_top = if let Some(id) = target(&mut op) {
            let index = e.layer_index(*id)?;
            *id = 0;
            Some(e.doc()?.layers.len() - 1 - index)
        } else {
            None
        };
        let step = Step {
            operation: op,
            layer_from_top,
        };
        let mut action = self.action.clone();
        action.steps.push(step.clone());
        action.validate()?;
        Ok(Some(step))
    }
    pub fn start(&mut self, e: &Editor, name: String) -> Result<(), String> {
        if e.doc()?.animation.is_some() {
            return Err("Extrayez d'abord une image fixe du GIF".into());
        }
        let action = Action {
            version: 1,
            name,
            steps: vec![],
        };
        action.validate()?;
        self.action = action;
        self.recording = true;
        Ok(())
    }
    pub fn read(&mut self, path: &str) -> Result<(), String> {
        if self.recording {
            return Err("Arrêtez d'abord l'enregistrement".into());
        }
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_BYTES as u64 {
            return Err("Action trop volumineuse".into());
        }
        let action: Action = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        action.validate()?;
        self.action = action;
        Ok(())
    }
    pub fn save(&self, path: &str) -> Result<(), String> {
        self.action.validate()?;
        io::atomic_write(
            path,
            &serde_json::to_vec_pretty(&self.action).map_err(|e| e.to_string())?,
        )
    }
}

pub fn replay(e: &mut Editor, action: &Action, cancel: &AtomicBool) -> Result<(), String> {
    action.validate()?;
    if action.steps.is_empty() {
        return Err("L'action est vide".into());
    }
    let mut staged = e.clone();
    for (i, step) in action.steps.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Traitement annulé".into());
        }
        let mut op = step.operation.clone();
        if let Some(offset) = step.layer_from_top {
            let layers = &staged.doc()?.layers;
            let index = layers.len().checked_sub(offset + 1).ok_or_else(|| {
                format!(
                    "Étape {} : calque {} depuis le haut absent",
                    i + 1,
                    offset + 1
                )
            })?;
            *target(&mut op).ok_or("Cible absente")? = layers[index].id;
        }
        operations::execute(&mut staged, op).map_err(|err| format!("Étape {} : {err}", i + 1))?;
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Traitement annulé".into());
    }
    e.adopt(staged, &format!("Action : {}", action.name));
    Ok(())
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchStatus {
    pub id: u64,
    pub running: bool,
    pub cancelled: bool,
    pub total: usize,
    pub current: String,
    pub results: Vec<BatchResult>,
}
#[derive(Clone, Serialize)]
pub struct BatchResult {
    pub source: String,
    pub output: Option<String>,
    pub error: Option<String>,
}
#[derive(Default)]
pub struct Batch {
    pub status: BatchStatus,
    pub cancel: Arc<AtomicBool>,
}
pub type SharedBatch = Arc<Mutex<Batch>>;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub paths: Vec<String>,
    pub directory: String,
    pub format: String,
    pub quality: u8,
}

pub fn begin(
    batch: &SharedBatch,
    request: &BatchRequest,
    action: &Action,
) -> Result<BatchStatus, String> {
    action.validate()?;
    if action.steps.is_empty() {
        return Err("L'action est vide".into());
    }
    if request.paths.is_empty() || request.paths.len() > 10_000 {
        return Err("Choisissez entre 1 et 10 000 fichiers".into());
    }
    if !Path::new(&request.directory).is_dir() {
        return Err("Dossier de sortie introuvable".into());
    }
    if ![
        "png", "jpg", "webp", "bmp", "tiff", "svg", "prisme", "avif", "pdf", "gif",
    ]
    .contains(&request.format.as_str())
    {
        return Err("Format de lot inconnu".into());
    }
    let mut b = batch.lock().map_err(|_| "État du lot indisponible")?;
    if b.status.running {
        return Err("Un lot est déjà en cours".into());
    }
    b.cancel = Arc::new(AtomicBool::new(false));
    b.status = BatchStatus {
        id: b.status.id + 1,
        running: true,
        total: request.paths.len(),
        ..Default::default()
    };
    Ok(b.status.clone())
}
/// Réserve un nouveau nom avec create_new : aucun fichier préexistant n'est remplacé.
fn write_unique(directory: &str, source: &str, ext: &str, bytes: &[u8]) -> Result<String, String> {
    let stem = Path::new(source)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    for n in 1..=100_000 {
        let suffix = if n == 1 {
            String::new()
        } else {
            format!("_{n}")
        };
        let path = Path::new(directory).join(format!("{stem}_prisme{suffix}.{ext}"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let result = file.write_all(bytes).and_then(|_| file.sync_all());
                drop(file);
                if let Err(e) = result {
                    let _ = fs::remove_file(&path);
                    return Err(e.to_string());
                }
                return Ok(path.to_string_lossy().into_owned());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Impossible de réserver un nom de sortie libre".into())
}
pub fn run(batch: SharedBatch, request: BatchRequest, action: Action) {
    let cancel = match batch.lock() {
        Ok(b) => b.cancel.clone(),
        Err(_) => return,
    };
    for source in request.paths {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(mut b) = batch.lock() {
            b.status.current = source.clone();
        }
        let result = (|| {
            let doc = io::import(&source)?;
            let mut e = Editor::default();
            e.load(doc, None, "Lot");
            replay(&mut e, &action, &cancel)?;
            let bytes = if request.format == "prisme" {
                io::project_bytes(e.doc()?)?
            } else {
                io::export_bytes(e.doc()?, &request.format, request.quality)?
            };
            if cancel.load(Ordering::Relaxed) {
                return Err("Traitement annulé".into());
            }
            write_unique(&request.directory, &source, &request.format, &bytes)
        })();
        if cancel.load(Ordering::Relaxed) && result.is_err() {
            break;
        }
        let (output, error) = match result {
            Ok(path) => (Some(path), None),
            Err(err) => (None, Some(err)),
        };
        if let Ok(mut b) = batch.lock() {
            b.status.results.push(BatchResult {
                source,
                output,
                error,
            });
        }
    }
    if let Ok(mut b) = batch.lock() {
        b.status.running = false;
        b.status.cancelled = cancel.load(Ordering::Relaxed);
        b.status.current.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn command(e: &mut Editor, v: serde_json::Value) {
        operations::execute(e, serde_json::from_value(v).unwrap()).unwrap();
    }
    #[test]
    fn recorded_layer_targets_rebind_and_replay_is_one_atomic_undo() {
        let mut e = Editor::default();
        command(
            &mut e,
            json!({"type":"new","width":4,"height":3,"background":"#ffffff"}),
        );
        let mut rec = Recorder::default();
        rec.start(&e, "Exemple".into()).unwrap();
        let id = e.doc().unwrap().layers[0].id;
        for v in [
            json!({"type":"filter","id":id,"name":"invert","params":{}}),
            json!({"type":"resize","width":8,"height":6}),
        ] {
            let op = serde_json::from_value(v).unwrap();
            let step = rec.prepare(&e, &op).unwrap().unwrap();
            operations::execute(&mut e, op).unwrap();
            rec.action.steps.push(step);
        }
        assert!(rec.prepare(&e, &Operation::Undo).is_err());
        command(
            &mut e,
            json!({"type":"new","width":4,"height":3,"background":"#ff0000"}),
        );
        assert_ne!(e.doc().unwrap().layers[0].id, id);
        let before = io::png_bytes(&crate::vector::flatten(e.doc().unwrap()).unwrap()).unwrap();
        let history = e.info().history.len();
        replay(&mut e, &rec.action, &AtomicBool::new(false)).unwrap();
        assert_eq!(e.doc().unwrap().width, 8);
        assert_eq!(e.info().history.len(), history + 1);
        e.undo().unwrap();
        assert_eq!(
            io::png_bytes(&crate::vector::flatten(e.doc().unwrap()).unwrap()).unwrap(),
            before
        );
        let mut bad = rec.action.clone();
        bad.steps[0].layer_from_top = Some(15);
        assert!(replay(&mut e, &bad, &AtomicBool::new(false)).is_err());
        assert_eq!(e.doc().unwrap().width, 4);
        assert_eq!(e.info().history.len(), history + 1);
        assert!(replay(&mut e, &rec.action, &AtomicBool::new(true)).is_err());
    }
    #[test]
    fn action_files_reject_io_and_batch_never_overwrites() {
        let malicious = Action {
            version: 1,
            name: "x".into(),
            steps: vec![Step {
                operation: Operation::Save { path: "x".into() },
                layer_from_top: None,
            }],
        };
        assert!(malicious.validate().is_err());
        let dir = std::env::temp_dir().join(format!("prisme-batch-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let first = write_unique(dir.to_str().unwrap(), "source.png", "png", b"first").unwrap();
        let second = write_unique(dir.to_str().unwrap(), "source.png", "png", b"second").unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"first");
        assert_eq!(fs::read(&second).unwrap(), b"second");
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
