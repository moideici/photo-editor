//! Hôte PFL Unicode x64. Même processus isolé pour metadata, About et Execute.
use crate::{
    document::{Document, Editor},
    pfl_abi as a,
    plugin_runtime::{read_limited, Response},
    plugins,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
const FLAG: &str = "--prisme-pfl-host-v1";
pub fn default_timeout() -> u64 {
    120000
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub path: String,
    pub caption: String,
    pub version: String,
    pub author: String,
    pub comment: String,
    pub plugin_type: i32,
    pub flags: u32,
    pub compatibility: i32,
    pub glyph: Option<String>,
}
#[derive(Serialize)]
pub struct Catalog {
    pub plugins: Vec<Info>,
    pub errors: Vec<plugins::CatalogError>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    path: PathBuf,
    mode: String,
    input: bool,
    active_id: Option<u64>,
    parent_window: isize,
    filename: String,
    config_path: PathBuf,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub info: Info,
    pub changed: bool,
    pub new_document: bool,
    pub touched: Vec<u64>,
    pub callbacks: Vec<String>,
    pub message: String,
}
fn recent() -> &'static std::sync::Mutex<Option<Outcome>> {
    static RECENT: std::sync::OnceLock<std::sync::Mutex<Option<Outcome>>> =
        std::sync::OnceLock::new();
    RECENT.get_or_init(std::sync::Mutex::default)
}
pub fn remember(outcome: Outcome) {
    if let Ok(mut value) = recent().lock() {
        *value = Some(outcome);
    }
}
pub fn last() -> Result<Option<Outcome>, String> {
    Ok(recent()
        .lock()
        .map_err(|_| "Résultat PFL indisponible")?
        .clone())
}
pub struct Applied {
    pub outcome: Outcome,
    pub document: Option<Document>,
}
/// Le transport de projet ne doit pas transformer les couches intactes en copies d’historique.
pub fn reuse_snapshot(
    document: &mut Document,
    current: &Document,
    touched: &[u64],
) -> Result<(), String> {
    // The PFL API has no operation for the typography library.
    document.text_styles = current.text_styles.clone();
    if current.layers.iter().map(|l| l.id).collect::<Vec<_>>()
        != document.layers.iter().map(|l| l.id).collect::<Vec<_>>()
    {
        return Err(
            "Le plugin PFL a changé la structure des calques sans API correspondante".into(),
        );
    }
    for (layer, before) in document.layers.iter_mut().zip(&current.layers) {
        if touched.contains(&layer.id) {
            if before.locks.pixels {
                return Err("Le PFL tente de modifier un calque verrouillé".into());
            }
            if before.locks.position && (layer.x != before.x || layer.y != before.y) {
                return Err("Le PFL tente de déplacer un calque verrouillé".into());
            }
            if before.locks.alpha {
                let old = before.raster().ok_or("Alpha verrouillé : bitmap requis")?;
                let current = layer
                    .raster()
                    .ok_or("Le PFL a changé le type d’un calque à alpha verrouillé")?;
                if old.dimensions() != current.dimensions() {
                    return Err(
                        "Le PFL a changé les dimensions d’un calque à alpha verrouillé".into(),
                    );
                }
                let mut pixels = (**current).clone();
                for (a, b) in old.pixels().zip(pixels.pixels_mut()) {
                    if a[3] == 0 || b[3] == 0 {
                        b[0] = a[0];
                        b[1] = a[1];
                        b[2] = a[2];
                    }
                    b[3] = a[3];
                }
                layer.content = crate::document::Content::Raster(std::sync::Arc::new(pixels));
            }
            layer.locks = before.locks;
            layer.version = before.version;
        } else {
            *layer = before.clone();
        }
    }
    if document.selection == current.selection {
        document.selection = current.selection.clone();
    }
    for (name, selection) in &mut document.selections {
        if let Some(before) = current.selections.get(name) {
            if selection == before {
                *selection = before.clone();
            }
        }
    }
    if document.metadata.exif == current.metadata.exif {
        document.metadata.exif = current.metadata.exif.clone();
    }
    if let (Some(after), Some(before)) = (&document.indexed, &current.indexed) {
        if after.palette == before.palette && after.indices == before.indices {
            document.indexed = Some(before.clone());
        }
    }
    Ok(())
}
fn validate_file(path: &Path) -> Result<PathBuf, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pfl"))
    {
        return Err("Choisissez un fichier .pfl".into());
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > 128 * 1024 * 1024 {
        return Err("Fichier PFL invalide ou supérieur à 128 Mio".into());
    }
    let pe = eightbf_host::host::pe::PeFile::open(&path).map_err(|e| e.to_string())?;
    if pe.arch() != "win-x64" {
        return Err(
            "Ce PFL n’est pas un binaire Windows x64 ; les plugins 32 bits demandent un autre hôte"
                .into(),
        );
    }
    Ok(path)
}
fn request(
    path: &Path,
    mode: &str,
    editor: Option<&Editor>,
    active_id: Option<u64>,
    parent: isize,
) -> Result<Request, String> {
    let config = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Prisme")
        .join("pfl");
    fs::create_dir_all(&config).map_err(|e| e.to_string())?;
    Ok(Request {
        path: validate_file(path)?,
        mode: mode.into(),
        input: editor.is_some_and(|e| e.doc.is_some()),
        active_id,
        parent_window: parent,
        filename: editor
            .and_then(|e| {
                e.path
                    .clone()
                    .or_else(|| e.doc.as_ref().map(|d| d.name.clone()))
            })
            .unwrap_or_default(),
        config_path: config,
    })
}
fn dispatch(
    request: &Request,
    editor: Option<&Editor>,
    timeout_ms: u64,
) -> Result<Applied, String> {
    if !(100..=600000).contains(&timeout_ms) {
        return Err("Délai PFL hors de 100–600000 ms".into());
    }
    let folder = tempfile::Builder::new()
        .prefix("prisme-pfl-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    fs::write(
        folder.path().join("request.json"),
        serde_json::to_vec(request).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if request.input {
        let doc = editor.ok_or("Document PFL absent")?.doc()?;
        fs::write(
            folder.path().join("input.prisme"),
            crate::io::project_bytes(doc)?,
        )
        .map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    let _restore_window = RestoreWindow::new(request.parent_window);
    let status = plugins::run_host(
        &std::env::current_exe().map_err(|e| e.to_string())?,
        FLAG,
        folder.path(),
        request
            .path
            .parent()
            .ok_or("Dossier du plugin PFL absent")?,
        timeout_ms,
    )?;
    let response: Response = serde_json::from_slice(
        &read_limited(&folder.path().join("response.json"), 16 * 1024).map_err(|_| {
            format!("L’hôte PFL s’est interrompu ({status}) ; le document est conservé")
        })?,
    )
    .map_err(|e| e.to_string())?;
    if !status.success() || !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| format!("Échec PFL ({status})")));
    }
    let outcome: Outcome = serde_json::from_slice(&read_limited(
        &folder.path().join("result.json"),
        256 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    let document = if outcome.changed {
        Some(crate::io::read_project(&read_limited(
            &folder.path().join("output.prisme"),
            512 * 1024 * 1024,
        )?)?)
    } else {
        None
    };
    if outcome.changed && request.mode != "execute" {
        return Err("Une requête de métadonnées PFL a renvoyé une modification".into());
    }
    Ok(Applied { outcome, document })
}
pub fn info(path: &Path) -> Result<Info, String> {
    Ok(
        dispatch(&request(path, "inspect", None, None, 0)?, None, 5000)?
            .outcome
            .info,
    )
}
pub fn scan(directory: &Path) -> Result<Catalog, String> {
    let mut catalog = Catalog {
        plugins: vec![],
        errors: vec![],
    };
    let mut count = 0;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("pfl"))
        {
            continue;
        }
        count += 1;
        if count > 200 {
            return Err("Au plus 200 fichiers PFL par dossier".into());
        }
        match info(&path) {
            Ok(p) => catalog.plugins.push(p),
            Err(message) => catalog.errors.push(plugins::CatalogError {
                path: path.to_string_lossy().into_owned(),
                message,
            }),
        }
    }
    catalog.plugins.sort_by(|a, b| a.caption.cmp(&b.caption));
    Ok(catalog)
}
pub fn apply(
    editor: &Editor,
    id: Option<u64>,
    path: &Path,
    timeout_ms: u64,
    parent: isize,
) -> Result<Applied, String> {
    dispatch(
        &request(path, "execute", Some(editor), id, parent)?,
        Some(editor),
        timeout_ms,
    )
}
pub fn about(path: &Path, parent: isize) -> Result<String, String> {
    let result = dispatch(&request(path, "about", None, None, parent)?, None, 120000)?;
    Ok(result.outcome.message)
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
#[cfg(not(windows))]
fn run_job(_folder: &Path) -> Result<(), String> {
    Err("L’hôte PFL exige Windows x64".into())
}
#[cfg(windows)]
fn run_job(folder: &Path) -> Result<(), String> {
    use crate::pfl_callbacks::{self, Context};
    use libloading::{Library, Symbol};
    let req: Request =
        serde_json::from_slice(&read_limited(&folder.join("request.json"), 128 * 1024)?)
            .map_err(|e| e.to_string())?;
    let path = validate_file(&req.path)?;
    if !matches!(req.mode.as_str(), "inspect" | "execute" | "about") {
        return Err("Commande hôte PFL invalide".into());
    }
    let mut editor = Editor::default();
    if req.input {
        let doc = crate::io::read_project(&read_limited(
            &folder.join("input.prisme"),
            512 * 1024 * 1024,
        )?)?;
        editor.load(doc, None, "Document du plugin");
    }
    // Aucun de ces exports, y compris le constructeur de DLL, n'est exécuté dans l'éditeur.
    let library = unsafe { Library::new(&path) }.map_err(|e| format!("Chargement PFL : {e}"))?;
    unsafe {
        let compatibility: Symbol<unsafe extern "system" fn() -> i32> = library
            .get(b"RegisterCompatibility\0")
            .map_err(|e| format!("Export PFL RegisterCompatibility absent : {e}"))?;
        let header: Symbol<unsafe extern "system" fn() -> *const a::Header> = library
            .get(b"RegisterPluginHeader\0")
            .map_err(|e| format!("Export PFL RegisterPluginHeader absent : {e}"))?;
        let structure: Symbol<unsafe extern "system" fn() -> *mut u8> = library
            .get(b"RegisterPluginStruct\0")
            .map_err(|e| format!("Export PFL RegisterPluginStruct absent : {e}"))?;
        let execute: Symbol<unsafe extern "system" fn() -> i32> = library
            .get(b"Execute\0")
            .map_err(|e| format!("Export PFL Execute absent : {e}"))?;
        let level = compatibility();
        if !(2..=4).contains(&level) {
            return Err(format!(
                "Compatibilité PFL {level} non prise en charge : niveaux DBmp 2 à 4 seulement"
            ));
        }
        if let Ok(language) = library
            .get::<unsafe extern "system" fn() -> *mut a::Language>(b"RegisterPluginLanguage\0")
        {
            let language = language();
            if !language.is_null() {
                let mut value = a::Language {
                    code: [0; 4],
                    name: [0; 64],
                };
                for (dst, src) in value.code.iter_mut().zip(a::wide("FR")) {
                    *dst = src
                }
                for (dst, src) in value.name.iter_mut().zip(a::wide("Français")) {
                    *dst = src
                }
                ptr_copy_language(language, &value);
            }
        }
        let p = header();
        if p.is_null() {
            return Err("En-tête PFL nul".into());
        }
        let h = std::ptr::read_unaligned(p);
        if !(0..=4).contains(&h.plugin_type) || !matches!(h.reserved[0], 0 | 55) {
            return Err("Type ou format d’en-tête PFL invalide".into());
        }
        let info = Info {
            path: path.to_string_lossy().into_owned(),
            caption: a::text(&h.caption)?,
            version: a::text(&h.version)?,
            author: a::text(&h.author)?,
            comment: a::text(&h.comment)?,
            plugin_type: h.plugin_type,
            flags: h.flags,
            compatibility: level,
            glyph: read_glyph(&library)?,
        };
        if info.caption.trim().is_empty() {
            return Err("Le PFL ne fournit aucun nom".into());
        }
        if req.mode == "inspect" {
            let result = Outcome {
                info,
                changed: false,
                new_document: false,
                touched: vec![],
                callbacks: vec![],
                message: String::new(),
            };
            fs::write(
                folder.join("result.json"),
                serde_json::to_vec(&result).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        if req.mode == "execute" {
            validate_conditions(&info, &editor)?;
        }
        let context = Context::new(
            editor,
            req.active_id,
            info.plugin_type,
            req.parent_window,
            req.filename,
            path.parent().unwrap().into(),
            req.config_path,
        );
        pfl_callbacks::install(context)?;
        let host = pfl_callbacks::host();
        let target = structure();
        if target.is_null() {
            let _ = pfl_callbacks::take();
            return Err("Structure PFL nulle".into());
        }
        // Les plugins de niveau 2 peuvent allouer un préfixe plus court. Ne jamais écrire au-delà.
        std::ptr::copy_nonoverlapping((&host as *const a::Host).cast::<u8>(), target, 0x38);
        for (index, function) in host.functions.iter().enumerate() {
            let offset = 0x38 + index * 8;
            if offset < a::Host::prefix_bytes(level) && offset != 0x170 {
                std::ptr::write_unaligned(target.add(offset).cast::<usize>(), *function);
            }
        }
        let status = if req.mode == "about" {
            match library.get::<unsafe extern "system" fn()>(b"AboutPlugin\0") {
                Ok(about) => {
                    about();
                    1
                }
                Err(_) => 1,
            }
        } else {
            execute()
        };
        let context = pfl_callbacks::take()?;
        if !context.errors.is_empty() {
            return Err(format!("Callback PFL : {}", context.errors.join(" ; ")));
        }
        if status == 0 {
            return Err("Plugin PFL annulé ou en échec ; le document est conservé".into());
        }
        if req.mode == "about" && context.changed {
            return Err("AboutPlugin a tenté de modifier une image".into());
        }
        if context.changed {
            fs::write(
                folder.join("output.prisme"),
                crate::io::project_bytes(context.editor.doc()?)?,
            )
            .map_err(|e| e.to_string())?;
        }
        let message = if req.mode == "about" {
            format!(
                "{}\n{}\n{}\n{}",
                info.caption, info.version, info.author, info.comment
            )
        } else {
            context.message
        };
        let result = Outcome {
            info,
            changed: context.changed,
            new_document: context.new_document,
            touched: context.touched.into_iter().collect(),
            callbacks: context
                .calls
                .into_iter()
                .filter(|name| name != "Host")
                .collect(),
            message,
        };
        fs::write(
            folder.join("result.json"),
            serde_json::to_vec(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(windows)]
unsafe fn ptr_copy_language(target: *mut a::Language, value: &a::Language) {
    std::ptr::copy_nonoverlapping(
        (value as *const a::Language).cast::<u8>(),
        target.cast::<u8>(),
        std::mem::size_of::<a::Language>(),
    );
}
fn validate_conditions(info: &Info, editor: &Editor) -> Result<(), String> {
    let doc = editor.doc.as_ref();
    let f = info.flags;
    if matches!(info.plugin_type, 0 | 1 | 3) && doc.is_none() {
        return Err("Ce plugin PFL nécessite une image".into());
    }
    if f & 0x40 != 0 && doc.is_none() {
        return Err("Le plugin demande une image ouverte".into());
    }
    if f & 0x80 != 0 {
        return Err("Ce plugin PFL nécessite plusieurs documents ouverts ; cette fonction reste prévue au plan".into());
    }
    if let Some(d) = doc {
        let indexed = d.indexed.is_some();
        let selected = d.selection.is_some();
        let alpha = d
            .layers
            .iter()
            .any(|l| l.raster().is_some_and(|i| i.pixels().any(|p| p[3] != 255)));
        for (yes, no, actual, label) in [
            (1, 2, !indexed, "mode couleur"),
            (4, 8, alpha, "transparence"),
            (16, 32, selected, "sélection"),
            (0x100, 0x200, d.layers.len() > 1, "calques"),
            (0x400, 0x800, alpha, "canal alpha"),
        ] {
            let pair = f & (yes | no);
            if (pair == yes && !actual) || (pair == no && actual) {
                return Err(format!("Le plugin PFL ne permet pas cet état : {label}"));
            }
        }
        let format_flags = f & 0x7f0000;
        if format_flags != 0 {
            let ext = Path::new(editor.path.as_deref().unwrap_or(&d.name))
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let bits = match ext.as_str() {
                "jpg" | "jpeg" | "jpe" | "jfif" => 0x10000,
                "gif" => 0x20000,
                "png" => 0x40000,
                "bmp" | "rle" => 0x80000,
                "tiff" | "tif" => 0x100000,
                "pfs" => 0x200000,
                _ => 0,
            };
            let exif =
                format_flags & 0x400000 != 0 && bits == 0x10000 && !d.metadata.exif.is_empty();
            if format_flags & bits == 0 && !exif {
                return Err(
                    "Le plugin PFL ne permet pas ce format ou demande des EXIF JPEG".into(),
                );
            }
        }
    }
    Ok(())
}
#[cfg(windows)]
unsafe fn read_glyph(library: &libloading::Library) -> Result<Option<String>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use windows_sys::Win32::Graphics::Gdi::*;
    let Ok(glyph) =
        library.get::<unsafe extern "system" fn() -> a::Bitmap>(b"RegisterPluginGlyph\0")
    else {
        return Ok(None);
    };
    let bitmap = glyph();
    if bitmap.is_null() {
        return Ok(None);
    }
    let mut bm: BITMAP = std::mem::zeroed();
    if GetObjectW(
        bitmap,
        std::mem::size_of::<BITMAP>() as i32,
        (&mut bm as *mut BITMAP).cast(),
    ) == 0
        || bm.bmWidth != 19
        || bm.bmHeight != 19
    {
        return Ok(None);
    }
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = 19;
    info.bmiHeader.biHeight = -19;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let dc = CreateCompatibleDC(std::ptr::null_mut());
    if dc.is_null() {
        return Ok(None);
    }
    let mut bytes = vec![0u8; 19 * 19 * 4];
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        19,
        bytes.as_mut_ptr().cast(),
        &mut info,
        DIB_RGB_COLORS,
    );
    DeleteDC(dc);
    if lines != 19 {
        return Ok(None);
    }
    let key = [bytes[2], bytes[1], bytes[0]];
    for p in bytes.chunks_exact_mut(4) {
        p.swap(0, 2);
        p[3] = if p[..3] == key { 0 } else { 255 }
    }
    let image = image::RgbaImage::from_raw(19, 19, bytes).ok_or("Icône PFL invalide")?;
    Ok(Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(crate::io::png_bytes(&image)?)
    )))
}
#[cfg(windows)]
struct RestoreWindow {
    window: isize,
    enabled: bool,
}
#[cfg(windows)]
impl RestoreWindow {
    fn new(window: isize) -> Self {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
        Self {
            window,
            enabled: window != 0 && unsafe { IsWindowEnabled(window as a::Bitmap) } != 0,
        }
    }
}
#[cfg(windows)]
impl Drop for RestoreWindow {
    fn drop(&mut self) {
        if self.enabled {
            unsafe {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    self.window as a::Bitmap,
                    1,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Content, SelShape, Selection};
    use image::{Rgba, RgbaImage};
    use std::sync::Arc;

    #[test]
    fn child_roundtrip_shares_unchanged_history_buffers() {
        let mut editor = Editor::default();
        let background = editor.new_raster_layer(
            "Fond",
            RgbaImage::from_pixel(4, 3, Rgba([20, 40, 60, 255])),
            0,
            0,
        );
        let upper = editor.new_raster_layer(
            "Intact",
            RgbaImage::from_pixel(4, 3, Rgba([80, 90, 100, 128])),
            0,
            0,
        );
        let selection = Selection {
            shape: SelShape::Mask {
                width: 4,
                height: 3,
                data: Arc::new(vec![128; 12]),
            },
            inverted: false,
        };
        let mut current = Document {
            text_styles: Vec::new(),
            width: 4,
            height: 3,
            layers: vec![background, upper],
            selection: Some(selection.clone()),
            selections: [("Masque".into(), selection)].into(),
            name: "Test".into(),
            indexed: None,
            animation: None,
            metadata: Default::default(),
        };
        current.metadata.exif = Arc::new(vec![1, 2, 3]);
        let mut returned =
            crate::io::read_project(&crate::io::project_bytes(&current).unwrap()).unwrap();
        assert!(!Arc::ptr_eq(
            current.layers[1].raster().unwrap(),
            returned.layers[1].raster().unwrap()
        ));
        let Content::Raster(image) = &mut returned.layers[0].content else {
            unreachable!()
        };
        Arc::make_mut(image).put_pixel(0, 0, Rgba([200, 210, 220, 255]));
        reuse_snapshot(&mut returned, &current, &[current.layers[0].id]).unwrap();
        assert!(!Arc::ptr_eq(
            current.layers[0].raster().unwrap(),
            returned.layers[0].raster().unwrap()
        ));
        assert_eq!(
            returned.layers[0].raster().unwrap().get_pixel(0, 0).0,
            [200, 210, 220, 255]
        );
        assert!(Arc::ptr_eq(
            current.layers[1].raster().unwrap(),
            returned.layers[1].raster().unwrap()
        ));
        assert!(Arc::ptr_eq(&current.metadata.exif, &returned.metadata.exif));
        let mask = |s: &Selection| match &s.shape {
            SelShape::Mask { data, .. } => data.clone(),
            _ => panic!("mask"),
        };
        assert!(Arc::ptr_eq(
            &mask(current.selection.as_ref().unwrap()),
            &mask(returned.selection.as_ref().unwrap())
        ));
        assert!(Arc::ptr_eq(
            &mask(&current.selections["Masque"]),
            &mask(&returned.selections["Masque"])
        ));
        returned.layers.swap(0, 1);
        assert!(reuse_snapshot(&mut returned, &current, &[]).is_err());
    }
}
