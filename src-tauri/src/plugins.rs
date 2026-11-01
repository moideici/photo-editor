//! Pilotage des hôtes séparés et catalogue de manifestes.
use crate::plugin_runtime::{self, Manifest, Request, Response};
use image::RgbaImage;
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub path: String,
    #[serde(flatten)]
    pub manifest: Manifest,
}
#[derive(Serialize)]
pub struct Catalog {
    pub plugins: Vec<Info>,
    pub errors: Vec<CatalogError>,
}
#[derive(Serialize)]
pub struct CatalogError {
    pub path: String,
    pub message: String,
}
pub fn scan(directory: &Path) -> Result<Catalog, String> {
    let mut result = Catalog {
        plugins: Vec::new(),
        errors: Vec::new(),
    };
    let entries = fs::read_dir(directory).map_err(|e| e.to_string())?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !path.is_file()
            || !path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .ends_with(".prisme-plugin.json")
        {
            continue;
        }
        if result.plugins.len() + result.errors.len() >= 200 {
            return Err("Au plus 200 manifestes par dossier de plugins".into());
        }
        match info(&path) {
            Ok(p) => result.plugins.push(p),
            Err(message) => result.errors.push(CatalogError {
                path: path.to_string_lossy().into_owned(),
                message,
            }),
        }
    }
    result
        .plugins
        .sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(result)
}
pub fn info(path: &Path) -> Result<Info, String> {
    let (manifest, _) = plugin_runtime::read_manifest(path)?;
    Ok(Info {
        path: path
            .canonicalize()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned(),
        manifest,
    })
}
pub fn apply(image: &RgbaImage, manifest: &Path, params: &Value) -> Result<RgbaImage, String> {
    apply_with_host(
        image,
        manifest,
        params,
        &std::env::current_exe().map_err(|e| e.to_string())?,
    )
}
pub fn apply_with_host(
    image: &RgbaImage,
    manifest: &Path,
    params: &Value,
    host: &Path,
) -> Result<RgbaImage, String> {
    let (m, _) = plugin_runtime::read_manifest(manifest)?;
    plugin_runtime::parameters(&m, params)?;
    crate::io::dimensions(image.width(), image.height())?;
    let folder = tempfile::Builder::new()
        .prefix("prisme-plugin-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let manifest = manifest.canonicalize().map_err(|e| e.to_string())?;
    let request = Request {
        manifest: manifest.clone(),
        width: image.width(),
        height: image.height(),
        params: params.clone(),
    };
    fs::write(
        folder.path().join("request.json"),
        serde_json::to_vec(&request).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(folder.path().join("input.rgba"), image.as_raw()).map_err(|e| e.to_string())?;
    let status = run_host(
        host,
        "--prisme-plugin-host-v1",
        folder.path(),
        manifest.parent().unwrap(),
        m.timeout_ms,
    )?;
    let response = plugin_runtime::read_limited(&folder.path().join("response.json"), 16 * 1024)
        .map_err(|_| {
            format!("L’hôte du plugin s’est interrompu ({status}) ; l’image est conservée")
        })?;
    let response: Response = serde_json::from_slice(&response)
        .map_err(|e| format!("Réponse invalide du plugin : {e}"))?;
    if !status.success() || !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| format!("L’hôte a échoué ({status})")));
    }
    let bytes = plugin_runtime::read_limited(
        &folder.path().join("output.rgba"),
        image.as_raw().len() as u64,
    )?;
    RgbaImage::from_raw(image.width(), image.height(), bytes)
        .ok_or("Taille de résultat du plugin invalide".into())
}

/// Seul point de lancement de code tiers, avec les mêmes limites pour chaque ABI.
pub(crate) fn run_host(
    host: &Path,
    flag: &str,
    folder: &Path,
    directory: &Path,
    timeout_ms: u64,
) -> Result<std::process::ExitStatus, String> {
    let mut command = Command::new(host);
    command
        .arg(flag)
        .arg(folder)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Démarrage de l’hôte : {e}"))?;
    #[cfg(windows)]
    let _job = match Job::assign(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => (),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "Le plugin a dépassé son délai de {} ms ; l’image est conservée",
                timeout_ms
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok(status)
}

#[cfg(windows)]
struct Job(windows::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Job {
    fn assign(child: &std::process::Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{Foundation::HANDLE, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(None, None)
                .map_err(|e| format!("Création du conteneur de plugin : {e}"))?;
            let job = Self(handle);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            limits.ProcessMemoryLimit = 512 * 1024 * 1024;
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )
            .map_err(|e| e.to_string())?;
            AssignProcessToJobObject(handle, HANDLE(child.as_raw_handle()))
                .map_err(|e| format!("Isolation du processus de plugin : {e}"))?;
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

pub fn examples_path() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        return Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugin-sdk/examples");
    }
    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new("."))
            .join("plugin-sdk/examples")
    }
}
