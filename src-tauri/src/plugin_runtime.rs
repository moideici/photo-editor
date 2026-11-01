//! Contrat commun au catalogue et au processus hôte. Les DLL ne sont chargées que
//! dans le processus enfant ; le catalogue ne lit que le manifeste JSON.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
pub const MAX_PIXELS: u64 = 16_777_216;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameter {
    pub key: String,
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    #[serde(default = "one")]
    pub step: f64,
}
fn one() -> f64 {
    1.0
}
fn timeout() -> u64 {
    30_000
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub kind: Kind,
    pub file: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default = "timeout")]
    pub timeout_ms: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Native,
    Rhai,
}
pub fn read_manifest(path: &Path) -> Result<(Manifest, PathBuf), String> {
    let bytes = read_limited(path, 64 * 1024)?;
    let m: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("Manifeste invalide : {e}"))?;
    if m.schema_version != 1
        || m.id.is_empty()
        || m.id.len() > 80
        || !m
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
        || m.name.trim().is_empty()
        || m.name.len() > 160
        || m.version.len() > 80
        || m.author.len() > 200
        || m.description.len() > 2000
        || m.parameters.len() > 32
        || !(100..=120_000).contains(&m.timeout_ms)
    {
        return Err("Champs ou version de manifeste invalides".into());
    }
    let mut keys = std::collections::HashSet::new();
    for p in &m.parameters {
        if p.key.is_empty()
            || p.key.len() > 64
            || !p
                .key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || !keys.insert(&p.key)
            || p.label.len() > 160
            || [p.min, p.max, p.default, p.step]
                .iter()
                .any(|v| !v.is_finite())
            || p.min > p.max
            || p.default < p.min
            || p.default > p.max
            || p.step <= 0.0
        {
            return Err("Paramètre de plugin invalide".into());
        }
    }
    let root = path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Manifeste sans dossier")?
        .to_path_buf();
    let relative = Path::new(&m.file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        return Err("Le fichier du plugin doit être relatif au manifeste".into());
    }
    let file = root
        .join(relative)
        .canonicalize()
        .map_err(|e| format!("Fichier du plugin introuvable : {e}"))?;
    if !file.starts_with(&root) || !file.is_file() {
        return Err("Fichier du plugin hors de son dossier".into());
    }
    let ext = file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if (m.kind == Kind::Native && ext != "dll") || (m.kind == Kind::Rhai && ext != "rhai") {
        return Err("Extension du fichier incompatible avec le type de plugin".into());
    }
    if fs::metadata(&file).map_err(|e| e.to_string())?.len()
        > if m.kind == Kind::Rhai {
            256 * 1024
        } else {
            128 * 1024 * 1024
        }
    {
        return Err("Fichier de plugin trop volumineux".into());
    }
    Ok((m, file))
}
pub fn parameters(m: &Manifest, input: &Value) -> Result<BTreeMap<String, f64>, String> {
    let input = input
        .as_object()
        .ok_or("Les paramètres doivent former un objet JSON")?;
    if input
        .keys()
        .any(|k| !m.parameters.iter().any(|p| p.key == *k))
    {
        return Err("Paramètre de plugin inconnu".into());
    }
    m.parameters
        .iter()
        .map(|p| {
            let value = match input.get(&p.key) {
                Some(v) => v.as_f64().ok_or("Valeur de paramètre non numérique")?,
                None => p.default,
            };
            if !value.is_finite() || value < p.min || value > p.max {
                return Err(format!(
                    "{} doit être compris entre {} et {}",
                    p.label, p.min, p.max
                ));
            }
            Ok((p.key.clone(), value))
        })
        .collect()
}
pub fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Fichier de plugin trop volumineux".into());
    }
    Ok(bytes)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub manifest: PathBuf,
    pub width: u32,
    pub height: u32,
    pub params: Value,
}
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub error: Option<String>,
}

#[repr(C)]
pub struct ImageV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub reserved: u32,
    pub pixels: *mut u8,
    pub pixels_len: usize,
    pub params_json: *const u8,
    pub params_json_len: usize,
}
pub fn host_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--prisme-plugin-host-v1")) {
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
        serde_json::from_slice(&read_limited(&folder.join("request.json"), 128 * 1024)?)
            .map_err(|e| e.to_string())?;
    let len = request.width as u64 * request.height as u64 * 4;
    if request.width == 0
        || request.height == 0
        || request.width > 16384
        || request.height > 16384
        || len / 4 > MAX_PIXELS
    {
        return Err("Dimensions du plugin invalides".into());
    }
    let mut data = read_limited(&folder.join("input.rgba"), len)?;
    if data.len() as u64 != len {
        return Err("Pixels d’entrée incomplets".into());
    }
    let (manifest, file) = read_manifest(&request.manifest)?;
    let params = parameters(&manifest, &request.params)?;
    match manifest.kind {
        Kind::Native => native(&file, request.width, request.height, &mut data, &params)?,
        Kind::Rhai => script(&file, request.width, &mut data, &params)?,
    }
    fs::write(folder.join("output.rgba"), data).map_err(|e| e.to_string())
}
fn native(
    file: &Path,
    w: u32,
    h: u32,
    pixels: &mut [u8],
    params: &BTreeMap<String, f64>,
) -> Result<(), String> {
    let params = serde_json::to_vec(params).map_err(|e| e.to_string())?;
    let mut error = [0u8; 4096];
    let mut image = ImageV1 {
        struct_size: std::mem::size_of::<ImageV1>() as u32,
        abi_version: 1,
        width: w,
        height: h,
        stride: w * 4,
        reserved: 0,
        pixels: pixels.as_mut_ptr(),
        pixels_len: pixels.len(),
        params_json: params.as_ptr(),
        params_json_len: params.len(),
    };
    // L’initialisation, l’appel et le déchargement étrangers restent dans cet hôte.
    unsafe {
        let lib = libloading::Library::new(file).map_err(|e| format!("Chargement DLL : {e}"))?;
        let run: libloading::Symbol<unsafe extern "C" fn(*mut ImageV1, *mut u8, usize) -> i32> =
            lib.get(b"prisme_filter_v1\0")
                .map_err(|e| format!("Export prisme_filter_v1 absent : {e}"))?;
        let status = run(&mut image, error.as_mut_ptr(), error.len());
        if status != 0 {
            let end = error.iter().position(|b| *b == 0).unwrap_or(error.len());
            return Err(format!(
                "Plugin ({status}) : {}",
                String::from_utf8_lossy(&error[..end])
            ));
        }
    }
    if image.struct_size != std::mem::size_of::<ImageV1>() as u32
        || image.abi_version != 1
        || image.width != w
        || image.height != h
        || image.stride != w * 4
        || image.pixels != pixels.as_mut_ptr()
        || image.pixels_len != pixels.len()
        || image.params_json != params.as_ptr()
        || image.params_json_len != params.len()
    {
        return Err("Le plugin a modifié le contrat de l’image".into());
    }
    Ok(())
}
pub fn script(
    file: &Path,
    width: u32,
    pixels: &mut [u8],
    params: &BTreeMap<String, f64>,
) -> Result<(), String> {
    use rhai::{Array, Dynamic, Engine, Map, Scope, FLOAT, INT};
    let source = String::from_utf8(read_limited(file, 256 * 1024)?).map_err(|e| e.to_string())?;
    let mut engine = Engine::new();
    engine
        .disable_symbol("eval")
        .set_max_expr_depths(64, 32)
        .set_max_call_levels(32)
        .set_max_operations(10_000)
        .set_max_array_size(64)
        .set_max_map_size(64)
        .set_max_string_size(4096)
        .set_max_variables(128)
        .set_max_functions(64);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    let ast = engine
        .compile(&source)
        .map_err(|e| format!("Script : {e}"))?;
    let params: Map = params
        .iter()
        .map(|(k, v)| (k.clone().into(), Dynamic::from_float(*v as FLOAT)))
        .collect();
    let mut scope = Scope::new();
    for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
        let x = (i % width as usize) as INT;
        let y = (i / width as usize) as INT;
        let result: Array = engine
            .call_fn(
                &mut scope,
                &ast,
                "pixel",
                (
                    x,
                    y,
                    p[0] as INT,
                    p[1] as INT,
                    p[2] as INT,
                    p[3] as INT,
                    params.clone(),
                ),
            )
            .map_err(|e| format!("Script au pixel ({x}, {y}) : {e}"))?;
        if result.len() != 4 {
            return Err("pixel doit retourner [rouge, vert, bleu, alpha]".into());
        }
        for (dst, value) in p.iter_mut().zip(result) {
            let n = if value.is::<INT>() {
                value.cast::<INT>() as f64
            } else if value.is::<FLOAT>() {
                value.cast::<FLOAT>() as f64
            } else {
                return Err("Un canal du script n’est pas un nombre".into());
            };
            if !n.is_finite() {
                return Err("Canal non fini dans le script".into());
            }
            *dst = n.round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn manifests_are_metadata_only_and_paths_and_parameters_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("fake.dll"),
            b"This is deliberately not a DLL",
        )
        .unwrap();
        let path = dir.path().join("fake.prisme-plugin.json");
        let mut m = json!({"schemaVersion":1,"id":"tests.fake","name":"Fake","version":"1","kind":"native","file":"fake.dll","parameters":[{"key":"amount","label":"Amount","min":-10,"max":10,"default":2,"step":1}]});
        fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
        let (manifest, _) = read_manifest(&path).unwrap();
        assert_eq!(crate::plugins::scan(dir.path()).unwrap().plugins.len(), 1);
        assert_eq!(parameters(&manifest, &json!({})).unwrap()["amount"], 2.0);
        assert!(parameters(&manifest, &json!({"amount":99})).is_err());
        assert!(parameters(&manifest, &json!({"unknown":1})).is_err());
        for file in ["../fake.dll", "C:\\Windows\\fake.dll"] {
            m["file"] = json!(file);
            fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
            assert!(read_manifest(&path).is_err());
        }
        m["file"] = json!("fake.dll");
        m["parameters"][0]["default"] = json!(30);
        fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(read_manifest(&path).is_err());
    }
    #[test]
    fn original_rhai_examples_produce_rgba_and_numeric_parameters() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugin-sdk/examples");
        let mut pixels = vec![20, 60, 100, 128, 0, 255, 40, 0];
        script(
            &folder.join("negative.rhai"),
            2,
            &mut pixels,
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(pixels, [235, 195, 155, 128, 255, 0, 215, 0]);
        let mut pixels = vec![20, 60, 250, 128];
        script(
            &folder.join("brightness.rhai"),
            1,
            &mut pixels,
            &BTreeMap::from([("amount".into(), 20.0)]),
        )
        .unwrap();
        assert_eq!(pixels, [40, 80, 255, 128]);
    }
    #[test]
    fn rhai_rejects_unbounded_loops_imports_eval_and_invalid_returns() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("invalid.rhai");
        for source in [
            "fn pixel(x,y,r,g,b,a,p) { loop {} }",
            "import \"elsewhere\" as a; fn pixel(x,y,r,g,b,a,p){[r,g,b,a]}",
            "fn pixel(x,y,r,g,b,a,p){eval(\"1+1\")}",
            "fn pixel(x,y,r,g,b,a,p){[r,g]}",
            "fn pixel(x,y,r,g,b,a,p){[\"oops\",g,b,a]}",
        ] {
            fs::write(&file, source).unwrap();
            assert!(
                script(&file, 1, &mut [20, 30, 40, 255], &BTreeMap::new()).is_err(),
                "{source}"
            );
        }
    }
    #[test]
    fn native_v1_layout_matches_the_c_header() {
        assert_eq!(std::mem::size_of::<ImageV1>(), 56);
        assert_eq!(std::mem::offset_of!(ImageV1, pixels), 24);
        assert_eq!(std::mem::offset_of!(ImageV1, params_json), 40);
    }
}
