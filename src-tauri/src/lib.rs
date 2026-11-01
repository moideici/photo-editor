mod adjustments;
mod animation;
mod assets;
mod automation;
mod clip;
mod document;
mod effects;
mod eightbf;
mod embedded;
#[cfg(test)]
mod experience_tests;
#[cfg(test)]
mod feimage_tests;
#[cfg(test)]
mod font_tests;
mod fonts;
mod formats;
mod group_style;
#[cfg(test)]
mod group_tests;
mod groups;
mod indexed;
mod interop;
mod io;
mod layer_tools;
mod metadata;
#[cfg(test)]
mod milestone12;
#[cfg(test)]
mod milestone18;
mod operations;
mod paint;
mod paths;
#[cfg(test)]
mod pattern_tests;
mod pfl;
mod pfl_abi;
#[cfg(windows)]
mod pfl_callbacks;
mod pfv;
mod plugin_runtime;
mod plugins;
mod preview_jobs;
mod psd;
mod raster;
mod retouch;
mod selection;
#[cfg(test)]
mod styled_group_tests;
mod svg_edit;
mod svg_filter;
#[cfg(test)]
mod svg_filter_tests;
mod svg_mask;
#[cfg(test)]
mod svg_mask_tests;
#[cfg(test)]
mod svg_media_tests;
mod text_path;
#[cfg(test)]
mod text_path_tests;
mod text_styles;
#[cfg(test)]
mod updater_tests;
mod vector;
mod vector_export;
mod vectorize;
#[cfg(test)]
mod vectorize_tests;
mod workspace;

use base64::{engine::general_purpose::STANDARD, Engine};
use document::{DocInfo, Editor};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
type SharedEditor = Arc<Mutex<Editor>>;
type SharedRecorder = Arc<Mutex<automation::Recorder>>;

#[tauri::command]
async fn operation_preview(
    state: State<'_, SharedEditor>,
    jobs: State<'_, Arc<preview_jobs::Jobs>>,
    operation: operations::Operation,
    revision: u64,
    owner: Option<String>,
    request: Option<u64>,
) -> Result<serde_json::Value, String> {
    if !matches!(
        operation,
        operations::Operation::Filter { .. }
            | operations::Operation::Asset { .. }
            | operations::Operation::Effects { .. }
            | operations::Operation::Transparency { .. }
    ) {
        return Err("Cette commande ne propose pas d’aperçu".into());
    }
    let token=if let Some(owner)=owner {jobs.begin(&owner,request.ok_or("Requête d’aperçu absente")?)?}else{preview_jobs::Token::default()};
    let jobs=jobs.inner().clone();
    let mut editor = {
        let e = state.lock().map_err(|_| "État indisponible")?;
        if e.revision() != revision {
            return Err("Le document a changé".into());
        }
        e.clone()
    };
    tauri::async_runtime::spawn_blocking(move || {
      let _worker=jobs.worker.lock().map_err(|_|"Aperçus indisponibles")?;
      preview_jobs::run(token,||{
        let preview = |editor: &Editor| -> Result<String, String> {
            use image::imageops;
            let d = editor.doc()?;
            let image = vector::flatten(d)?;
            let small = imageops::thumbnail(&image, 320, 240);
            Ok(format!(
                "data:image/png;base64,{}",
                STANDARD.encode(io::png_bytes(&small)?)
            ))
        };
        let before = preview(&editor)?;
        preview_jobs::checkpoint()?;
        operations::execute(&mut editor, operation)?;
        preview_jobs::checkpoint()?;
        let after = preview(&editor)?;
        Ok(serde_json::json!({"before":before,"after":after}))
      })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn cancel_operation_preview(jobs:State<'_,Arc<preview_jobs::Jobs>>,owner:String,request:u64)->Result<(),String>{jobs.cancel(&owner,request)}

#[tauri::command]
async fn workspace_control(
    state: State<'_, SharedEditor>,
    pool: State<'_, workspace::Shared>,
    recorder: State<'_, SharedRecorder>,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    let pool = pool.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut w = pool.lock().map_err(|_| "Onglets indisponibles")?;
        let mut e = state.lock().map_err(|_| "État indisponible")?;
        let kind = request["kind"].as_str().unwrap_or("");
        if kind == "checkpoint" {
            // Compress a consistent snapshot without holding the paint/selection mutex.
            let snapshot = e.clone();
            drop(e);
            w.checkpoint(&snapshot)?;
            let current = state.lock().map_err(|_| "État indisponible")?;
            return Ok(w.status(&current));
        }
        if !["status", "view", "checkpoint", "remember"].contains(&kind)
            && recorder
                .lock()
                .map_err(|_| "Actions indisponibles")?
                .recording
        {
            return Err("Arrêtez l’enregistrement de l’action avant de changer d’onglet".into());
        }
        match kind {
            "status" => return Ok(w.status(&e)),
            "view" => {
                w.view(serde_json::from_value(request["view"].clone()).map_err(|e| e.to_string())?)?
            }
            "open" => w.open(
                &mut e,
                serde_json::from_value(request["operation"].clone()).map_err(|e| e.to_string())?,
            )?,
            "switch" => w.switch(&mut e, request["id"].as_u64().ok_or("Onglet absent")?)?,
            "close" => w.close(&mut e, request["discard"].as_bool().unwrap_or(false))?,
            "checkpoint" => w.checkpoint(&e)?,
            "remember" => {
                w.remember(request["path"].as_str().ok_or("Chemin absent")?.into(), &e)?
            }
            "restore" => w.restore(
                &mut e,
                request["file"].as_str().ok_or("Récupération absente")?,
            )?,
            "forgetRecovery" => {
                w.forget_recovery(request["file"].as_str().ok_or("Récupération absente")?)?
            }
            _ => return Err("Commande d’onglets inconnue".into()),
        }
        let mut result = w.status(&e);
        if ["open", "switch", "close", "restore"].contains(&kind) {
            result["document"] = serde_json::to_value(e.info()).map_err(|e| e.to_string())?;
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn layer_thumbnail(
    state: State<'_, SharedEditor>,
    layer_id: u64,
    revision: u64,
    size: u32,
) -> Result<String, String> {
    let (layer, w, h) = {
        let e = state.lock().map_err(|_| "État indisponible")?;
        if e.revision() != revision {
            return Err("Miniature périmée".into());
        }
        let d = e.doc()?;
        (
            d.layers
                .iter()
                .find(|l| l.id == layer_id)
                .ok_or("Calque absent")?
                .clone(),
            d.width,
            d.height,
        )
    };
    tauri::async_runtime::spawn_blocking(move || layer_tools::thumbnail(&layer, w, h, size))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn vectorize_preview(
    state: State<'_, SharedEditor>,
    layer_id: u64,
    revision: u64,
    settings: vectorize::Settings,
) -> Result<serde_json::Value, String> {
    let input = {
        let editor = state.lock().map_err(|_| "État indisponible")?;
        vectorize::input(&editor, layer_id, revision)?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let data = vectorize::trace(&input, &settings)?;
        let (source, result) = vectorize::preview(&data)?;
        Ok(serde_json::json!({
            "revision": revision, "stats": data.stats,
            "source":format!("data:image/png;base64,{}",STANDARD.encode(io::png_bytes(&source)?)),
            "src":format!("data:image/png;base64,{}",STANDARD.encode(io::png_bytes(&result)?))
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn font_catalog() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(fonts::catalog)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn font_info(family: String, bold: bool, italic: bool) -> Result<fonts::Info, String> {
    tauri::async_runtime::spawn_blocking(move || fonts::info(&family, bold, italic))
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn font_preview(
    mut shapes: Vec<document::Shape>,
    shape: document::Shape,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fonts::validate_controls(&shape)?;
        io::validate_shape(&shape)?;
        let id = shape.id;
        *groups::find_mut(&mut shapes, id).ok_or("Texte introuvable")? = shape;
        groups::validate(&shapes)?;
        text_path::refresh(&mut shapes)?;
        let shape = groups::find(&shapes, id).unwrap();
        let src = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&svg_filter::preview(shape)?)?)
        );
        Ok(serde_json::json!({"shape":shape,"src":src,"font":fonts::shape_info(shape)}))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn text_path_layout(
    shapes: Vec<document::Shape>,
    shape_id: u64,
    settings: text_path::Settings,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        groups::validate(&shapes)?;
        let shape = text_path::layout(shapes, shape_id, settings)?;
        let src = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&svg_filter::preview(&shape)?)?)
        );
        Ok(serde_json::json!({"shape":shape,"src":src}))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn filter_catalog() -> serde_json::Value {
    serde_json::json!({"effects":svg_filter::catalog(),"children":svg_filter::child_catalog()})
}
#[tauri::command]
async fn filter_resource_preview(resource: svg_filter::FilterResource) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&resource.preview()?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn filter_resource_image(
    asset: embedded::ImageAsset,
    width: f32,
    height: f32,
    fit: String,
    align: String,
) -> Result<svg_filter::FilterResource, String> {
    tauri::async_runtime::spawn_blocking(move || {
        svg_filter::FilterResource::from_asset(asset, width, height, &fit, &align)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn filter_preview(mut shape: document::Shape) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        io::validate_shape(&shape)?;
        let mut id = 0;
        groups::assign_ids(std::slice::from_mut(&mut shape), &mut || {
            id += 1;
            id
        });
        groups::validate(std::slice::from_ref(&shape))?;
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&svg_filter::preview(&shape)?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn paint_bounds(shape: document::Shape) -> Result<[f32; 4], String> {
    tauri::async_runtime::spawn_blocking(move || {
        io::validate_shape(&shape)?;
        paint::bounds(&shape)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn vector_mask_hit(
    editor: State<'_, SharedEditor>,
    layer_id: u64,
    version: u64,
    candidates: Vec<u64>,
    point: [f32; 2],
) -> Result<Option<u64>, String> {
    let layer = {
        let e = editor.lock().map_err(|e| e.to_string())?;
        let layer = &e.doc()?.layers[e.layer_index(layer_id)?];
        if layer.version != version {
            return Err("Le calque a changé pendant la sélection".into());
        }
        layer.clone()
    };
    tauri::async_runtime::spawn_blocking(move || svg_mask::pick(&layer, &candidates, point))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn embedded_image_file(path: String) -> Result<embedded::ImageAsset, String> {
    tauri::async_runtime::spawn_blocking(move || embedded::file(&path))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn mask_preview(mask: svg_mask::VectorMask) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&mask.preview()?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn decompose_svg_shape(shape: document::Shape) -> Result<Vec<document::Shape>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        io::validate_shape(&shape)?;
        svg_edit::decompose(&shape)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn embedded_image_preview(asset: embedded::ImageAsset) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move||{
        asset.validate()?;let scale=128./asset.width.max(asset.height)as f32;
        let (w,h)=((asset.width as f32*scale).round().max(1.)as u32,(asset.height as f32*scale).round().max(1.)as u32);
        let source=vector::svg_document(w,h,&format!("<image width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\" style=\"image-rendering:{}\" href=\"{}\"/>",asset.rendering,asset.url()));
        Ok(format!("data:image/png;base64,{}",STANDARD.encode(io::png_bytes(&vector::render_svg(&source,w,h)?)?)))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn pattern_preview(pattern: paint::Pattern) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        pattern.validate()?;
        let scale=128./pattern.width.max(pattern.height);
        let w=(pattern.width*scale).round().max(1.) as u32;
        let h=(pattern.height*scale).round().max(1.) as u32;
        let source=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {} {}\" preserveAspectRatio=\"none\">{}</svg>",pattern.content_width,pattern.content_height,pattern.children.iter().map(vector::shape_svg).collect::<String>());
        let image=vector::render_svg(&source,w,h)?;
        Ok(format!("data:image/png;base64,{}",STANDARD.encode(io::png_bytes(&image)?)))
    }).await.map_err(|e|e.to_string())?
}

pub fn plugin_host_entry() -> Option<i32> {
    pfl::host_entry()
        .or_else(eightbf::host_entry)
        .or_else(plugin_runtime::host_entry)
}
#[tauri::command]
async fn plugins_scan(directory: String) -> Result<plugins::Catalog, String> {
    tauri::async_runtime::spawn_blocking(move || plugins::scan(std::path::Path::new(&directory)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn plugin_manifest(path: String) -> Result<plugins::Info, String> {
    plugins::info(std::path::Path::new(&path))
}
#[tauri::command]
fn plugin_examples() -> Result<plugins::Catalog, String> {
    plugins::scan(&plugins::examples_path())
}

#[tauri::command]
async fn pfl_file(path: String) -> Result<pfl::Info, String> {
    tauri::async_runtime::spawn_blocking(move || pfl::info(std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn pfl_scan(directory: String) -> Result<pfl::Catalog, String> {
    tauri::async_runtime::spawn_blocking(move || pfl::scan(std::path::Path::new(&directory)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn pfl_about(window: tauri::WebviewWindow, path: String) -> Result<String, String> {
    #[cfg(windows)]
    let parent = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    #[cfg(not(windows))]
    let parent = 0;
    tauri::async_runtime::spawn_blocking(move || pfl::about(std::path::Path::new(&path), parent))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn pfl_last_run() -> Result<Option<pfl::Outcome>, String> {
    pfl::last()
}
#[tauri::command]
async fn pfl_examples() -> Result<pfl::Catalog, String> {
    tauri::async_runtime::spawn_blocking(move || {
        pfl::scan(
            &plugins::examples_path()
                .parent()
                .ok_or("Dossier SDK absent")?
                .join("pfl/examples"),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn eightbf_file(path: String) -> Result<Vec<eightbf::Info>, String> {
    tauri::async_runtime::spawn_blocking(move || eightbf::info(std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn eightbf_scan(directory: String) -> Result<eightbf::Catalog, String> {
    tauri::async_runtime::spawn_blocking(move || eightbf::scan(std::path::Path::new(&directory)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn eightbf_about(
    window: tauri::WebviewWindow,
    path: String,
    key: String,
) -> Result<String, String> {
    #[cfg(windows)]
    let parent = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    #[cfg(not(windows))]
    let parent = 0;
    tauri::async_runtime::spawn_blocking(move || {
        eightbf::about(std::path::Path::new(&path), &key, parent)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn eightbf_last_settings(key: String) -> Result<Option<eightbf::Settings>, String> {
    eightbf::last(&key)
}
#[tauri::command]
fn eightbf_save_preset(path: String, key: String, destination: String) -> Result<(), String> {
    eightbf::save_preset(std::path::Path::new(&path), &key, &destination)
}
#[tauri::command]
fn eightbf_load_preset(
    path: String,
    key: String,
    source: String,
) -> Result<eightbf::Settings, String> {
    eightbf::load_preset(
        std::path::Path::new(&path),
        &key,
        std::path::Path::new(&source),
    )
}

#[tauri::command]
async fn editor_command(
    window: tauri::WebviewWindow,
    state: State<'_, SharedEditor>,
    recorder: State<'_, SharedRecorder>,
    mut operation: operations::Operation,
) -> Result<DocInfo, String> {
    #[cfg(windows)]
    if let operations::Operation::EightBf { settings, .. } = &mut operation {
        settings.parent_window = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    }
    #[cfg(windows)]
    if let operations::Operation::Pfl { parent_window, .. } = &mut operation {
        *parent_window = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    }
    let state = state.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut editor = state.lock().map_err(|_| "État de l'éditeur indisponible")?;
        let mut recorder = recorder
            .lock()
            .map_err(|_| "État des actions indisponible")?;
        let step = if matches!(&operation, operations::Operation::EightBf { .. }) {
            // Un dialogue peut produire un état différent de la requête initiale.
            // Ne valider le document et l’action qu’une fois cet état vérifié.
            let mut staged = editor.clone();
            let captured = operations::execute_captured(&mut staged, operation)?
                .ok_or("Le filtre 8BF n’a pas fourni ses paramètres de rejeu")?;
            let step = recorder.prepare(&editor, &captured)?;
            if let operations::Operation::EightBf { key, settings, .. } = captured {
                eightbf::remember(key, settings);
            }
            *editor = staged;
            step
        } else {
            let step = recorder.prepare(&editor, &operation)?;
            operations::execute(&mut editor, operation)?;
            step
        };
        if let Some(step) = step {
            recorder.action.steps.push(step);
        }
        Ok(editor.info())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn automation_state(recorder: State<'_, SharedRecorder>) -> Result<automation::Recorder, String> {
    Ok(recorder
        .lock()
        .map_err(|_| "État des actions indisponible")?
        .clone())
}
#[tauri::command]
fn automation_control(
    state: State<'_, SharedEditor>,
    recorder: State<'_, SharedRecorder>,
    control: automation::Control,
) -> Result<automation::Recorder, String> {
    let editor = state.lock().map_err(|_| "État indisponible")?;
    let mut rec = recorder
        .lock()
        .map_err(|_| "État des actions indisponible")?;
    automation::control(&mut rec, &editor, control)?;
    Ok(rec.clone())
}
#[tauri::command]
async fn run_action(
    state: State<'_, SharedEditor>,
    recorder: State<'_, SharedRecorder>,
) -> Result<DocInfo, String> {
    let action = {
        let rec = recorder
            .lock()
            .map_err(|_| "État des actions indisponible")?;
        if rec.recording {
            return Err("Arrêtez l'enregistrement avant de rejouer".into());
        }
        rec.action.clone()
    };
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut e = state.lock().map_err(|_| "État indisponible")?;
        automation::replay(&mut e, &action, &std::sync::atomic::AtomicBool::new(false))?;
        Ok(e.info())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn batch_status(
    batch: State<'_, automation::SharedBatch>,
) -> Result<automation::BatchStatus, String> {
    Ok(batch
        .lock()
        .map_err(|_| "État du lot indisponible")?
        .status
        .clone())
}
#[tauri::command]
fn batch_cancel(batch: State<'_, automation::SharedBatch>, id: u64) -> Result<(), String> {
    let b = batch.lock().map_err(|_| "État du lot indisponible")?;
    if b.status.id != id {
        return Err("Ce lot n'est plus actif".into());
    }
    b.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
#[tauri::command]
fn batch_start(
    batch: State<'_, automation::SharedBatch>,
    recorder: State<'_, SharedRecorder>,
    request: automation::BatchRequest,
) -> Result<automation::BatchStatus, String> {
    let rec = recorder
        .lock()
        .map_err(|_| "État des actions indisponible")?;
    if rec.recording {
        return Err("Arrêtez l'enregistrement avant de lancer le lot".into());
    }
    let action = rec.action.clone();
    let batch = batch.inner().clone();
    let status = automation::begin(&batch, &request, &action)?;
    tauri::async_runtime::spawn_blocking(move || automation::run(batch, request, action));
    Ok(status)
}

#[tauri::command]
async fn render_document(state: State<'_, SharedEditor>) -> Result<String, String> {
    let doc = state
        .lock()
        .map_err(|_| "État indisponible")?
        .doc()?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&vector::flatten(&doc)?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn document_info(state: State<'_, SharedEditor>) -> Result<DocInfo, String> {
    Ok(state.lock().map_err(|_| "État indisponible")?.info())
}

#[tauri::command]
async fn selection_overlay(state: State<'_, SharedEditor>) -> Result<String, String> {
    let doc = state
        .lock()
        .map_err(|_| "État indisponible")?
        .doc()?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let selection = doc.selection.as_ref().ok_or("Aucune sélection")?;
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&selection::overlay(
                selection, doc.width, doc.height
            ))?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn sample_color(state: State<'_, SharedEditor>, x: u32, y: u32) -> Result<String, String> {
    let editor = state.lock().map_err(|_| "État indisponible")?;
    let doc = editor.doc()?;
    if x >= doc.width || y >= doc.height {
        return Err("Point hors de l'image".into());
    }
    let img = vector::flatten(doc)?;
    let p = img.get_pixel(x, y);
    Ok(format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
}

#[tauri::command]
async fn layer_histogram(
    state: State<'_, SharedEditor>,
    id: u64,
) -> Result<adjustments::Histogram, String> {
    let doc = state
        .lock()
        .map_err(|_| "État indisponible")?
        .doc()?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let layer = doc
            .layers
            .iter()
            .find(|l| l.id == id)
            .ok_or("Calque introuvable")?;
        let (pixels, _, _) = vector::content_surface(layer, doc.width, doc.height)?;
        Ok(adjustments::histogram(&pixels))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn asset_preview(settings: assets::Settings) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&assets::preview(&settings)?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn preview_effects(
    state: State<'_, SharedEditor>,
    id: u64,
    effects: effects::LayerEffects,
) -> Result<String, String> {
    let mut doc = state
        .lock()
        .map_err(|_| "État indisponible")?
        .doc()?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        vector::style_dimensions(&effects, doc.width, doc.height)?;
        let layer = doc
            .layers
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or("Calque introuvable")?;
        layer.effects = effects;
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&vector::flatten(&doc)?)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn animation_frame(
    state: State<'_, SharedEditor>,
    id: u64,
    index: usize,
) -> Result<String, String> {
    let image = {
        let editor = state.lock().map_err(|_| "État indisponible")?;
        let animation = editor.doc()?.animation.as_ref().ok_or("Aucune animation")?;
        if animation.id != id {
            return Err("L'animation a changé".into());
        }
        animation
            .frames
            .get(index)
            .ok_or("Image introuvable")?
            .image
            .clone()
    };
    tauri::async_runtime::spawn_blocking(move || {
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(io::png_bytes(&image)?)
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(SharedEditor::default())
        .manage(Arc::new(preview_jobs::Jobs::default()))
        .manage(SharedRecorder::default())
        .manage(workspace::Shared::default())
        .manage(automation::SharedBatch::default())
        .setup(|app| {
            let root = std::env::var_os("PRISME_WORKSPACE_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("workspace"));
            app.state::<workspace::Shared>().lock().unwrap().init(root);
            if let Some(path) = std::env::args().nth(1).filter(|p| !p.starts_with('-')) {
                if let Ok(mut editor) = app.state::<SharedEditor>().lock() {
                    if let Err(error) =
                        operations::execute(&mut editor, operations::Operation::Open { path })
                    {
                        eprintln!("{error}");
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            editor_command,
            workspace_control,
            render_document,
            document_info,
            sample_color,
            paint_bounds,
            vector_mask_hit,
            text_path_layout,
            font_catalog,
            font_info,
            font_preview,
            vectorize_preview,
            layer_thumbnail,
            operation_preview,
            cancel_operation_preview,
            mask_preview,
            filter_preview,
            filter_catalog,
            filter_resource_preview,
            filter_resource_image,
            decompose_svg_shape,
            embedded_image_file,
            embedded_image_preview,
            pattern_preview,
            selection_overlay,
            layer_histogram,
            asset_preview,
            preview_effects,
            animation_frame,
            automation_state,
            automation_control,
            run_action,
            batch_status,
            batch_start,
            batch_cancel,
            plugins_scan,
            plugin_manifest,
            plugin_examples,
            eightbf_file,
            eightbf_scan,
            eightbf_about,
            eightbf_last_settings,
            eightbf_save_preset,
            eightbf_load_preset,
            pfl_file,
            pfl_scan,
            pfl_about,
            pfl_last_run,
            pfl_examples
        ])
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer Prisme");
}
