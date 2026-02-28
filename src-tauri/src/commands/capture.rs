use crate::pipeline::Pipeline;
use std::sync::Mutex;
use tauri::State;

pub struct PipelineState(pub Mutex<Option<Pipeline>>);

#[tauri::command]
pub fn start_capture(
    app_handle: tauri::AppHandle,
    pipeline_state: State<'_, PipelineState>,
) -> Result<(), String> {
    let mut pipeline = pipeline_state.0.lock().map_err(|e| e.to_string())?;

    if pipeline.is_some() {
        return Ok(()); // Already running
    }

    let p = Pipeline::start(app_handle, 500); // 2 FPS
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
            "is_capturing": false,
            "window_found": false,
            "window_title": null,
            "fps": 0.0,
            "last_capture_time": null,
            "resolution": null,
        })),
    }
}
