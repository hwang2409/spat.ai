use crate::pipeline::Pipeline;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct PipelineState(pub Mutex<Option<Pipeline>>);

#[tauri::command]
pub fn start_capture(
    app_handle: tauri::AppHandle,
    pipeline_state: State<'_, PipelineState>,
) -> Result<(), String> {
    let mut pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;

    if pipeline.is_some() {
        return Ok(());
    }

    let data_dir = app_handle
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
        });

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

/// List all visible windows on the system
#[tauri::command]
pub fn list_windows() -> Result<serde_json::Value, String> {
    let windows = tft_capture::list_windows();
    serde_json::to_value(&windows).map_err(|e| e.to_string())
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
