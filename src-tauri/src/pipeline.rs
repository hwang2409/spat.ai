use image::RgbaImage;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tracing::{info, warn};

use tft_capture::{CaptureStatus, TargetWindow};
use tft_vision::{ChampionMatcher, DigitReader, VisionResult};

/// Manages the capture → CV → state → advice pipeline
pub struct Pipeline {
    stop: Arc<AtomicBool>,
    status_rx: watch::Receiver<CaptureStatus>,
    vision_rx: watch::Receiver<Option<VisionResult>>,
    target_window: TargetWindow,
}

impl Pipeline {
    /// Start the pipeline background tasks
    pub fn start(app_handle: AppHandle, capture_interval_ms: u64, data_dir: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let target_window = tft_capture::new_target_window();

        let (frame_tx, frame_rx) = watch::channel::<Option<Arc<RgbaImage>>>(None);
        let (status_tx, status_rx) = watch::channel(CaptureStatus::default());
        let (vision_tx, vision_rx) = watch::channel::<Option<VisionResult>>(None);

        let capture_interval = Duration::from_millis(capture_interval_ms);

        // Start capture loop
        let stop_clone = stop.clone();
        let target_clone = target_window.clone();
        tauri::async_runtime::spawn(async move {
            tft_capture::capture_loop(
                frame_tx,
                status_tx,
                capture_interval,
                stop_clone,
                target_clone,
            )
            .await;
        });

        // Start vision processing loop
        let mut vision_frame_rx = frame_rx.clone();
        let stop_vision = stop.clone();
        tauri::async_runtime::spawn(async move {
            // Load matcher and OCR on a blocking thread
            let data_dir_clone = data_dir.clone();
            let init = tokio::task::spawn_blocking(move || {
                let matcher = ChampionMatcher::load(&data_dir_clone).unwrap_or_else(|e| {
                    warn!(
                        "Failed to load champion matcher: {}. Recognition disabled.",
                        e
                    );
                    ChampionMatcher::load(&PathBuf::from("/dev/null"))
                        .unwrap_or_else(|_| panic!("Failed to create empty matcher"))
                });
                let digit_reader = DigitReader::new();
                info!(
                    "Vision pipeline ready: {} templates, OCR {}",
                    matcher.template_count(),
                    if digit_reader.is_available() {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                (Arc::new(matcher), Arc::new(digit_reader))
            })
            .await;

            let (matcher, digit_reader) = match init {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to initialize vision: {}", e);
                    return;
                }
            };

            loop {
                if stop_vision.load(Ordering::Relaxed) {
                    break;
                }

                if vision_frame_rx.changed().await.is_err() {
                    break;
                }

                let frame = vision_frame_rx.borrow().clone();
                if let Some(frame) = frame {
                    let m = matcher.clone();
                    let dr = digit_reader.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        tft_vision::process_frame(&frame, &m, &dr)
                    })
                    .await;

                    if let Ok(vision_result) = result {
                        let _ = vision_tx.send(Some(vision_result));
                    }
                }
            }
        });

        // Emit capture status to frontend
        let mut status_rx_clone = status_rx.clone();
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                if status_rx_clone.changed().await.is_err() {
                    break;
                }
                let status = status_rx_clone.borrow().clone();
                let payload = serde_json::json!({
                    "isCapturing": status.is_capturing,
                    "windowFound": status.window_found,
                    "windowTitle": status.window_title,
                    "fps": status.fps,
                    "lastCaptureTime": status.last_capture_time,
                    "resolution": status.resolution,
                });
                let _ = app_clone.emit("capture-status", payload);
            }
        });

        // Emit vision results as game state to frontend
        let mut vision_rx_clone = vision_rx.clone();
        let app_clone2 = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                if vision_rx_clone.changed().await.is_err() {
                    break;
                }
                let result = vision_rx_clone.borrow().clone();
                if let Some(vision) = result {
                    let payload = serde_json::json!({
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
                    });
                    let _ = app_clone2.emit("game-state", payload);
                }
            }
        });

        info!("Pipeline started");

        Self {
            stop,
            status_rx,
            vision_rx,
            target_window,
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        info!("Pipeline stop requested");
    }

    pub fn capture_status(&self) -> CaptureStatus {
        self.status_rx.borrow().clone()
    }

    pub fn latest_vision(&self) -> Option<VisionResult> {
        self.vision_rx.borrow().clone()
    }

    /// Set the target window title for capture
    pub fn set_target_window(&self, title: Option<String>) {
        if let Ok(mut target) = self.target_window.write() {
            info!("Target window set to: {:?}", title);
            *target = title;
        }
    }
}
