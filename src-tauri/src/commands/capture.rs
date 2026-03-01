use crate::pipeline::Pipeline;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct PipelineState(pub Mutex<Option<Pipeline>>);

/// Resolve the data directory for vision assets
fn resolve_data_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    app_handle
        .path()
        .resource_dir()
        .ok()
        .map(|p| p.join("data"))
        .unwrap_or_else(|| {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));
            if let Some(dir) = exe_dir {
                let project_root = dir
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent());
                if let Some(root) = project_root {
                    let data = root.join("data");
                    if data.exists() {
                        return data;
                    }
                }
            }
            PathBuf::from("data")
        })
}

#[tauri::command]
pub fn start_capture(
    app_handle: tauri::AppHandle,
    pipeline_state: State<'_, PipelineState>,
) -> Result<(), String> {
    let mut pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;

    if pipeline.is_some() {
        return Ok(());
    }

    let data_dir = resolve_data_dir(&app_handle);
    tracing::info!("Data directory: {}", data_dir.display());

    let p = Pipeline::start(app_handle, 500, data_dir);
    *pipeline = Some(p);

    Ok(())
}

#[tauri::command]
pub fn stop_capture(pipeline_state: State<'_, PipelineState>) -> Result<(), String> {
    let pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;
    if let Some(ref p) = *pipeline {
        p.stop();
    }
    Ok(())
}

#[tauri::command]
pub fn get_capture_status(
    pipeline_state: State<'_, PipelineState>,
) -> Result<serde_json::Value, String> {
    let pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;
    match &*pipeline {
        Some(p) => {
            let status = p.capture_status();
            serde_json::to_value(&status).map_err(|e| e.to_string())
        }
        None => Ok(serde_json::json!({
            "isCapturing": false,
            "windowFound": false,
            "windowTitle": null,
            "fps": 0.0,
            "lastCaptureTime": null,
            "resolution": null,
        })),
    }
}

#[tauri::command]
pub fn get_game_state(
    pipeline_state: State<'_, PipelineState>,
) -> Result<serde_json::Value, String> {
    let pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;
    match &*pipeline {
        Some(p) => match p.latest_vision() {
            Some(vision) => Ok(serde_json::json!({
                "shop": vision.shop.iter().map(|s| serde_json::json!({
                    "index": s.slot_index,
                    "championId": s.champion_id,
                    "championName": s.champion_name,
                    "cost": s.cost,
                    "confidence": s.confidence,
                })).collect::<Vec<_>>(),
                "gold": vision.gold,
                "level": vision.level,
                "stage": vision.stage,
            })),
            None => Ok(serde_json::json!(null)),
        },
        None => Ok(serde_json::json!(null)),
    }
}

/// List all visible windows on the system, with permission diagnostics
#[tauri::command]
pub fn list_windows() -> Result<serde_json::Value, String> {
    let result = tft_capture::list_windows();
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

/// Save the current frame and region crops for debugging.
/// Returns the path to the debug directory.
#[tauri::command]
pub fn save_debug_frame(
    pipeline_state: State<'_, PipelineState>,
) -> Result<Option<String>, String> {
    let pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;
    match &*pipeline {
        Some(p) => Ok(p.save_debug_frame().map(|p| p.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

/// Set which window to capture. Pass null/empty to revert to auto-detection.
#[tauri::command]
pub fn set_target_window(
    title: Option<String>,
    pipeline_state: State<'_, PipelineState>,
) -> Result<(), String> {
    let pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;
    if let Some(ref p) = *pipeline {
        let target = title.filter(|t| !t.is_empty());
        p.set_target_window(target);
    }
    Ok(())
}

/// Start video file analysis — decodes a video and feeds frames through the vision pipeline.
#[tauri::command]
pub fn start_video_analysis(
    app_handle: tauri::AppHandle,
    path: String,
    pipeline_state: State<'_, PipelineState>,
) -> Result<(), String> {
    let mut pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;

    // Stop existing pipeline if running
    if let Some(ref p) = *pipeline {
        p.stop();
    }

    let video_path = Path::new(&path);
    if !video_path.exists() {
        return Err(format!("Video file not found: {}", path));
    }

    let data_dir = resolve_data_dir(&app_handle);
    tracing::info!("Starting video analysis: {}", path);

    let p = Pipeline::start_video(app_handle, video_path.to_path_buf(), 500, data_dir);
    *pipeline = Some(p);

    Ok(())
}
