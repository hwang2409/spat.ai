use image::RgbaImage;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tracing::{info, warn};

use tft_capture::CaptureStatus;
use tft_vision::{ChampionMatcher, DigitReader, VisionResult};

/// Manages the capture → CV → state → advice pipeline
pub struct Pipeline {
    stop: Arc<AtomicBool>,
    frame_rx: watch::Receiver<Option<Arc<RgbaImage>>>,
    status_rx: watch::Receiver<CaptureStatus>,
    vision_rx: watch::Receiver<Option<VisionResult>>,
}

impl Pipeline {
    /// Spawn the downstream tasks shared by both capture and video sources:
    /// vision processing loop, status emitter, and game-state emitter.
    fn spawn_downstream(
        app_handle: &AppHandle,
        frame_rx: &watch::Receiver<Option<Arc<RgbaImage>>>,
        status_rx: &watch::Receiver<CaptureStatus>,
        vision_tx: watch::Sender<Option<VisionResult>>,
        vision_rx: &watch::Receiver<Option<VisionResult>>,
        stop: &Arc<AtomicBool>,
        data_dir: PathBuf,
    ) {
        // Vision processing loop
        let mut vision_frame_rx = frame_rx.clone();
        let stop_vision = stop.clone();
        tauri::async_runtime::spawn(async move {
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
                        // Detect game area within the frame, then crop before vision
                        if let Some(game_area) = tft_vision::detect_game_area(&frame) {
                            let cropped = if game_area.region.width >= 1.0
                                && game_area.region.height >= 1.0
                            {
                                // Full frame — no crop needed
                                frame.as_ref().clone()
                            } else {
                                tft_capture::crop_region(&frame, &game_area.region)
                            };
                            tft_vision::process_frame(&cropped, &m, &dr)
                        } else {
                            // No game area detected — emit empty result
                            VisionResult::default()
                        }
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
    }

    /// Start the pipeline with video file analysis
    pub fn start_video(
        app_handle: AppHandle,
        video_path: PathBuf,
        frame_interval_ms: u64,
        data_dir: PathBuf,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));

        let (frame_tx, frame_rx) = watch::channel::<Option<Arc<RgbaImage>>>(None);
        let (status_tx, status_rx) = watch::channel(CaptureStatus::default());
        let (vision_tx, vision_rx) = watch::channel::<Option<VisionResult>>(None);

        let frame_interval = Duration::from_millis(frame_interval_ms);

        // Start video decode loop
        let stop_clone = stop.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) =
                tft_capture::video::video_loop(&video_path, frame_tx, status_tx, frame_interval, stop_clone).await
            {
                warn!("Video loop error: {}", e);
            }
        });

        Self::spawn_downstream(
            &app_handle,
            &frame_rx,
            &status_rx,
            vision_tx,
            &vision_rx,
            &stop,
            data_dir,
        );

        info!("Pipeline started (video analysis)");

        Self {
            stop,
            frame_rx,
            status_rx,
            vision_rx,
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

    /// Get the latest captured frame
    pub fn latest_frame(&self) -> Option<Arc<RgbaImage>> {
        self.frame_rx.borrow().clone()
    }

    /// Save the current frame and all dynamically-detected region crops for debugging.
    /// Returns the path to the debug directory.
    pub fn save_debug_frame(&self) -> Option<PathBuf> {
        let frame = self.latest_frame()?;

        let debug_dir = std::env::temp_dir().join("spat_ai_debug");
        let _ = std::fs::create_dir_all(&debug_dir);

        let (w, h) = (frame.width(), frame.height());

        // Save full frame
        let _ = frame.save(debug_dir.join("frame_full.png"));

        // Detect and save game area crop
        let game_frame = if let Some(game_area) = tft_vision::detect_game_area(&frame) {
            let cropped = if game_area.region.width >= 1.0 && game_area.region.height >= 1.0 {
                frame.as_ref().clone()
            } else {
                let crop = tft_capture::crop_region(&frame, &game_area.region);
                let _ = crop.save(debug_dir.join("game_area.png"));
                crop
            };
            let _ = std::fs::write(
                debug_dir.join("game_area_info.txt"),
                format!(
                    "Game area: x={:.0} y={:.0} w={:.0} h={:.0} (confidence={:.2})\n",
                    game_area.region.x * w as f64,
                    game_area.region.y * h as f64,
                    game_area.region.width * w as f64,
                    game_area.region.height * h as f64,
                    game_area.confidence,
                ),
            );
            cropped
        } else {
            let _ = std::fs::write(
                debug_dir.join("game_area_info.txt"),
                "No game area detected\n",
            );
            frame.as_ref().clone()
        };

        let (gw, gh) = (game_frame.width(), game_frame.height());

        // Dynamically detect layout on the game area
        let layout = tft_vision::detect_layout(&game_frame);

        // Save each detected shop slot
        for (i, region) in layout.shop_slots.iter().enumerate() {
            let crop = tft_capture::crop_region(&game_frame, region);
            let _ = crop.save(debug_dir.join(format!("shop_slot_{}.png", i)));
        }

        // Save detected economy regions
        if let Some(ref r) = layout.gold {
            let crop = tft_capture::crop_region(&game_frame, r);
            let _ = crop.save(debug_dir.join("gold.png"));
        }
        if let Some(ref r) = layout.level {
            let crop = tft_capture::crop_region(&game_frame, r);
            let _ = crop.save(debug_dir.join("level.png"));
        }
        if let Some(ref r) = layout.stage {
            let crop = tft_capture::crop_region(&game_frame, r);
            let _ = crop.save(debug_dir.join("stage.png"));
        }

        // Save detected layout info
        let mut info = format!("Frame: {}x{}\nGame area: {}x{}\n", w, h, gw, gh);
        info.push_str(&format!("HUD top: {:.1}% (y={:.0})\n\n", layout.hud_top * 100.0, layout.hud_top * gh as f64));

        for (i, r) in layout.shop_slots.iter().enumerate() {
            info.push_str(&format!(
                "Shop slot {}: x={:.0} y={:.0} w={:.0} h={:.0}\n",
                i,
                r.x * gw as f64,
                r.y * gh as f64,
                r.width * gw as f64,
                r.height * gh as f64,
            ));
        }
        if let Some(ref r) = layout.gold {
            info.push_str(&format!(
                "Gold: x={:.0} y={:.0} w={:.0} h={:.0}\n",
                r.x * gw as f64, r.y * gh as f64, r.width * gw as f64, r.height * gh as f64,
            ));
        }
        if let Some(ref r) = layout.level {
            info.push_str(&format!(
                "Level: x={:.0} y={:.0} w={:.0} h={:.0}\n",
                r.x * gw as f64, r.y * gh as f64, r.width * gw as f64, r.height * gh as f64,
            ));
        }
        if let Some(ref r) = layout.stage {
            info.push_str(&format!(
                "Stage: x={:.0} y={:.0} w={:.0} h={:.0}\n",
                r.x * gw as f64, r.y * gh as f64, r.width * gw as f64, r.height * gh as f64,
            ));
        }
        let _ = std::fs::write(debug_dir.join("regions.txt"), info);

        info!("Debug frame saved to {}", debug_dir.display());
        Some(debug_dir)
    }
}
