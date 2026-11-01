//! Adaptateur Photoshop 8BF : métadonnées sans chargement, exécution dans un enfant.
use crate::{
    plugin_runtime::{read_limited, Response},
    plugins,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use eightbf_host::host::{
    constants,
    descriptors::Descriptor,
    discovery::{self, PluginInfo},
    image::PlaneImage,
    session::{FilterHost, FilterRequest, Hooks, RunStatus},
};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const MAX_STATE: u64 = 1024 * 1024;
const FLAG: &str = "--prisme-8bf-host-v1";
fn timeout() -> u64 {
    120_000
}
fn white() -> [u8; 3] {
    [255; 3]
}
fn resolution() -> f64 {
    72.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub show_ui: bool,
    #[serde(default)]
    pub parameters: Option<String>,
    #[serde(default)]
    pub descriptor: Option<Value>,
    #[serde(default = "timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub foreground: [u8; 3],
    #[serde(default = "white")]
    pub background: [u8; 3],
    #[serde(default = "resolution")]
    pub resolution: f64,
    #[serde(skip)]
    pub parent_window: isize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            show_ui: false,
            parameters: None,
            descriptor: None,
            timeout_ms: timeout(),
            foreground: [0; 3],
            background: white(),
            resolution: resolution(),
            parent_window: 0,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !(100..=600_000).contains(&self.timeout_ms)
            || !self.resolution.is_finite()
            || self.resolution <= 0.0
            || self.resolution > 100_000.0
        {
            return Err("Délai ou résolution 8BF invalide".into());
        }
        if let Some(p) = &self.parameters {
            if p.len() > MAX_STATE as usize {
                return Err("Paramètres 8BF trop volumineux".into());
            }
            STANDARD
                .decode(p)
                .map_err(|_| "Paramètres 8BF : base64 invalide")?;
        }
        if let Some(d) = &self.descriptor {
            if serde_json::to_vec(d).map_err(|e| e.to_string())?.len() > MAX_STATE as usize {
                return Err("Descripteur 8BF trop volumineux".into());
            }
            Descriptor::from_json(d)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub key: String,
    pub path: String,
    pub name: String,
    pub category: String,
    pub architectures: Vec<String>,
    pub resource_id: Option<i32>,
    pub error: Option<String>,
}
fn summary(p: PluginInfo) -> Info {
    let error = if p.pipl.kind != constants::FILTER_KIND {
        Some("Cette ressource ne décrit pas un filtre Photoshop".into())
    } else if !p
        .runnable_archs()
        .iter()
        .any(|a| a == discovery::host_arch())
    {
        Some(format!(
            "Architecture incompatible : hôte {}",
            discovery::host_arch()
        ))
    } else {
        p.error.clone()
    };
    Info {
        key: p.key(),
        name: p.name(),
        category: p.category(),
        resource_id: p.pipl.resource_id,
        path: p.path,
        architectures: p.archs,
        error,
    }
}
fn entries(path: &Path) -> Result<Vec<PluginInfo>, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "8bf" | "eff") {
        return Err("Choisissez un fichier .8bf ou .eff".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 {
        return Err("Fichier 8BF invalide ou supérieur à 128 Mio".into());
    }
    let result = discovery::read_pe_plugin(&path).map_err(|e| e.to_string())?;
    if result.is_empty() {
        return Err("Aucune ressource PiPL dans ce fichier 8BF".into());
    }
    if result.len() > 200 {
        return Err("Au plus 200 filtres par fichier 8BF".into());
    }
    Ok(result)
}
pub fn info(path: &Path) -> Result<Vec<Info>, String> {
    Ok(entries(path)?.into_iter().map(summary).collect())
}
fn resolve(path: &Path, key: &str) -> Result<PluginInfo, String> {
    let p = entries(path)?
        .into_iter()
        .find(|p| p.key() == key)
        .ok_or("Filtre 8BF introuvable ; rechargez le fichier")?;
    if let Some(error) = summary(p.clone()).error {
        return Err(error);
    }
    Ok(p)
}
#[derive(Serialize)]
pub struct Catalog {
    pub plugins: Vec<Info>,
    pub errors: Vec<plugins::CatalogError>,
}
pub fn scan(directory: &Path) -> Result<Catalog, String> {
    let mut result = Catalog {
        plugins: vec![],
        errors: vec![],
    };
    let mut count = 0;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !path.is_file() || !matches!(ext.as_str(), "8bf" | "eff") {
            continue;
        }
        count += 1;
        if count > 200 {
            return Err("Au plus 200 fichiers 8BF par dossier".into());
        }
        match info(&path) {
            Ok(mut found) => result.plugins.append(&mut found),
            Err(message) => result.errors.push(plugins::CatalogError {
                path: path.to_string_lossy().into_owned(),
                message,
            }),
        }
        if result.plugins.len() > 2000 {
            return Err("Au plus 2000 filtres par catalogue".into());
        }
    }
    result
        .plugins
        .sort_by_key(|p| (p.category.clone(), p.name.clone(), p.key.clone()));
    Ok(result)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    path: PathBuf,
    key: String,
    settings: Settings,
    about: bool,
    width: u32,
    height: u32,
    mask: bool,
    parent_window: isize,
    document_title: String,
    layer_name: String,
}
#[derive(Serialize, Deserialize)]
pub struct Outcome {
    pub settings: Settings,
    pub message: String,
}
pub struct Applied {
    pub image: RgbaImage,
    pub settings: Settings,
}
pub fn apply(
    image: &RgbaImage,
    path: &Path,
    key: &str,
    settings: &Settings,
    mask: Option<&[u8]>,
    document_title: &str,
    layer_name: &str,
) -> Result<Applied, String> {
    settings.validate()?;
    crate::io::dimensions(image.width(), image.height())?;
    let p = resolve(path, key)?;
    if mask.is_some_and(|m| m.len() != image.width() as usize * image.height() as usize) {
        return Err("Masque local 8BF invalide".into());
    }
    let request = Request {
        path: PathBuf::from(&p.path),
        key: p.key(),
        settings: settings.clone(),
        about: false,
        width: image.width(),
        height: image.height(),
        mask: mask.is_some(),
        parent_window: settings.parent_window,
        document_title: document_title.into(),
        layer_name: layer_name.into(),
    };
    let (outcome, data) = dispatch(&request, Some(image.as_raw()), mask)?;
    let image = RgbaImage::from_raw(image.width(), image.height(), data)
        .ok_or("Résultat 8BF de taille incorrecte")?;
    Ok(Applied {
        image,
        settings: outcome.settings,
    })
}
pub fn about(path: &Path, key: &str, parent_window: isize) -> Result<String, String> {
    let p = resolve(path, key)?;
    let request = Request {
        path: PathBuf::from(&p.path),
        key: p.key(),
        settings: Settings::default(),
        about: true,
        width: 0,
        height: 0,
        mask: false,
        parent_window,
        document_title: String::new(),
        layer_name: String::new(),
    };
    Ok(dispatch(&request, None, None)?.0.message)
}
fn dispatch(
    request: &Request,
    input: Option<&[u8]>,
    mask: Option<&[u8]>,
) -> Result<(Outcome, Vec<u8>), String> {
    let folder = tempfile::Builder::new()
        .prefix("prisme-8bf-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    fs::write(
        folder.path().join("request.json"),
        serde_json::to_vec(request).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(input) = input {
        fs::write(folder.path().join("input.rgba"), input).map_err(|e| e.to_string())?;
    }
    if let Some(mask) = mask {
        fs::write(folder.path().join("mask.u8"), mask).map_err(|e| e.to_string())?;
    }
    let status = plugins::run_host(
        &std::env::current_exe().map_err(|e| e.to_string())?,
        FLAG,
        folder.path(),
        request.path.parent().ok_or("Dossier 8BF absent")?,
        request.settings.timeout_ms,
    )?;
    let response: Response = serde_json::from_slice(
        &read_limited(&folder.path().join("response.json"), 16 * 1024).map_err(|_| {
            format!("L’hôte 8BF s’est interrompu ({status}) ; l’image est conservée")
        })?,
    )
    .map_err(|e| format!("Réponse 8BF invalide : {e}"))?;
    if !status.success() || !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| format!("Échec 8BF ({status})")));
    }
    let outcome: Outcome = serde_json::from_slice(&read_limited(
        &folder.path().join("result.json"),
        3 * MAX_STATE,
    )?)
    .map_err(|e| e.to_string())?;
    outcome.settings.validate()?;
    if outcome.settings.show_ui {
        return Err("Paramètres 8BF de rejeu invalides".into());
    }
    let data = if request.about {
        vec![]
    } else {
        read_limited(
            &folder.path().join("output.rgba"),
            request.width as u64 * request.height as u64 * 4,
        )?
    };
    Ok((outcome, data))
}

pub fn host_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return None;
    }
    let Some(folder) = args.next() else {
        return Some(2);
    };
    if args.next().is_some() {
        return Some(2);
    }
    #[cfg(windows)]
    unsafe {
        windows::Win32::System::Diagnostics::Debug::SetErrorMode(
            windows::Win32::System::Diagnostics::Debug::SEM_FAILCRITICALERRORS
                | windows::Win32::System::Diagnostics::Debug::SEM_NOGPFAULTERRORBOX,
        );
    }
    let folder = Path::new(&folder);
    let result = run_job(folder);
    let response = Response {
        ok: result.is_ok(),
        error: result.err().map(|s| s.chars().take(2000).collect()),
    };
    if fs::write(
        folder.join("response.json"),
        serde_json::to_vec(&response).unwrap(),
    )
    .is_err()
    {
        return Some(2);
    }
    Some(if response.ok { 0 } else { 1 })
}
fn run_job(folder: &Path) -> Result<(), String> {
    let request: Request =
        serde_json::from_slice(&read_limited(&folder.join("request.json"), 3 * MAX_STATE)?)
            .map_err(|e| e.to_string())?;
    request.settings.validate()?;
    let info = resolve(&request.path, &request.key)?;
    let mut host = FilterHost::new();
    let result = if request.about {
        host.about(
            &info,
            request.parent_window,
            Box::new(|_| {}),
            Some(discovery::host_arch()),
        )
    } else {
        crate::io::dimensions(request.width, request.height)?;
        let count = request.width as u64 * request.height as u64;
        let data = read_limited(&folder.join("input.rgba"), count * 4)?;
        let image =
            PlaneImage::from_data(request.width as usize, request.height as usize, 4, 8, data)
                .map_err(|e| e.to_string())?;
        let mut filter = FilterRequest::new(image, 3, true);
        if request.mask {
            filter.mask = Some(
                PlaneImage::from_data(
                    request.width as usize,
                    request.height as usize,
                    1,
                    8,
                    read_limited(&folder.join("mask.u8"), count)?,
                )
                .map_err(|e| e.to_string())?,
            );
        }
        filter.parameters = request
            .settings
            .parameters
            .as_ref()
            .map(|s| STANDARD.decode(s))
            .transpose()
            .map_err(|e| e.to_string())?;
        filter.descriptor = request
            .settings
            .descriptor
            .as_ref()
            .map(Descriptor::from_json)
            .transpose()?;
        filter.show_ui = request.settings.show_ui;
        filter.parent_window = request.parent_window;
        filter.foreground = request.settings.foreground;
        filter.background = request.settings.background;
        filter.resolution = request.settings.resolution;
        filter.document_title = request.document_title;
        filter.layer_name = request.layer_name;
        host.run(
            &info,
            filter,
            Hooks::default(),
            Some(discovery::host_arch()),
        )
    };
    match result.status {
        RunStatus::Cancelled => return Err("Filtre 8BF annulé ; l’image est conservée".into()),
        RunStatus::Error => return Err(format!("8BF ({}): {}", result.code, result.message)),
        RunStatus::Ok => (),
    }
    let mut settings = request.settings;
    settings.show_ui = false;
    settings.parent_window = 0;
    // Les filtres déclarant une terminologie de script se rejouent par leur
    // descripteur public. Un bloc opaque peut dépendre de globals ou pointeurs
    // propres au processus précédent et ne doit pas supplanter ce descripteur.
    settings.parameters = if info.pipl.has_terminology
        && result
            .descriptor
            .as_ref()
            .is_some_and(|d| !d.items.is_empty())
    {
        None
    } else {
        result.parameters.map(|p| STANDARD.encode(p))
    };
    settings.descriptor = result.descriptor.map(|d| d.to_json());
    settings.validate()?;
    if !request.about {
        let image = result.image.ok_or("Aucun résultat image du filtre 8BF")?;
        if image.width != request.width as usize
            || image.height != request.height as usize
            || image.channels != 4
            || image.depth != 8
            || image.data.len() != request.width as usize * request.height as usize * 4
        {
            return Err("Le filtre 8BF a modifié le format de l’image".into());
        }
        fs::write(folder.join("output.rgba"), image.data).map_err(|e| e.to_string())?;
    }
    fs::write(
        folder.join("result.json"),
        serde_json::to_vec(&Outcome {
            settings,
            message: result.message.chars().take(4000).collect(),
        })
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn recent() -> &'static Mutex<BTreeMap<String, Settings>> {
    static RECENT: OnceLock<Mutex<BTreeMap<String, Settings>>> = OnceLock::new();
    RECENT.get_or_init(Mutex::default)
}
pub fn remember(key: String, settings: Settings) {
    if let Ok(mut values) = recent().lock() {
        if values.len() >= 200 && !values.contains_key(&key) {
            values.pop_first();
        }
        values.insert(key, settings);
    }
}
pub fn last(key: &str) -> Result<Option<Settings>, String> {
    Ok(recent()
        .lock()
        .map_err(|_| "Paramètres 8BF indisponibles")?
        .get(key)
        .cloned())
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preset {
    version: u32,
    name: String,
    resource_id: Option<i32>,
    settings: Settings,
}
pub fn save_preset(path: &Path, key: &str, destination: &str) -> Result<(), String> {
    let p = resolve(path, key)?;
    let settings = last(key)?.ok_or("Appliquez d’abord le filtre pour obtenir ses paramètres")?;
    settings.validate()?;
    crate::io::atomic_write(
        destination,
        &serde_json::to_vec_pretty(&Preset {
            version: 1,
            name: p.name(),
            resource_id: p.pipl.resource_id,
            settings,
        })
        .map_err(|e| e.to_string())?,
    )
}
pub fn load_preset(path: &Path, key: &str, source: &Path) -> Result<Settings, String> {
    let p = resolve(path, key)?;
    let preset: Preset =
        serde_json::from_slice(&read_limited(source, 3 * MAX_STATE)?).map_err(|e| e.to_string())?;
    if preset.version != 1
        || preset.name != p.name()
        || preset.resource_id != p.pipl.resource_id
        || preset.settings.show_ui
    {
        return Err("Ce préréglage ne correspond pas à ce filtre 8BF".into());
    }
    preset.settings.validate()?;
    remember(key.into(), preset.settings.clone());
    Ok(preset.settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_states_before_foreign_code() {
        let mut s = Settings::default();
        s.parameters = Some("%%%".into());
        assert!(s.validate().is_err());
        s.parameters = None;
        s.descriptor = Some(serde_json::json!({}));
        assert!(s.validate().is_err());
        s.descriptor = None;
        s.timeout_ms = 99;
        assert!(s.validate().is_err());
        s.timeout_ms = 120000;
        assert!(s.validate().is_ok());
    }
    #[test]
    fn catalog_rejects_invalid_files_without_loading_them() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("invalid.8bf"), b"not a DLL").unwrap();
        fs::write(folder.path().join("unrelated.txt"), b"ignored").unwrap();
        let c = scan(folder.path()).unwrap();
        assert!(c.plugins.is_empty());
        assert_eq!(c.errors.len(), 1);
    }
}
