use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tracing::info;

use tft_capture::CaptureStatus;

/// Manages the capture → CV → state → advice pipeline
#[allow(dead_code)]
pub struct Pipeline {
    stop: Arc<AtomicBool>,
    frame_rx: watch::Receiver<Option<Arc<RgbaImage>>>,
    status_rx: watch::Receiver<CaptureStatus>,
}

impl Pipeline {
    /// Start the pipeline background tasks
    pub fn start(app_handle: AppHandle, capture_interval_ms: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));

        let (frame_tx, frame_rx) = watch::channel(None);
        let (status_tx, status_rx) = watch::channel(CaptureStatus::default());

        let capture_interval = Duration::from_millis(capture_interval_ms);

        // Start capture loop
        let stop_clone = stop.clone();
        tokio::spawn(async move {
            tft_capture::capture_loop(frame_tx, status_tx, capture_interval, stop_clone).await;
        });

        // Start status emission loop (sends capture status to frontend)
        let mut status_rx_clone = status_rx.clone();
        let app_clone = app_handle.clone();
        tokio::spawn(async move {
            loop {
                if status_rx_clone.changed().await.is_err() {
                    break;
                }
                let status = status_rx_clone.borrow().clone();

                // Emit to frontend as camelCase for TypeScript consumption
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

        info!("Pipeline started");

        Self {
            stop,
            frame_rx,
            status_rx,
        }
    }

    /// Stop the pipeline
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        info!("Pipeline stop requested");
    }

    /// Get the current capture status
    pub fn capture_status(&self) -> CaptureStatus {
        self.status_rx.borrow().clone()
    }

    /// Get the latest frame (if any)
    #[allow(dead_code)]
    pub fn latest_frame(&self) -> Option<Arc<RgbaImage>> {
        self.frame_rx.borrow().clone()
    }
}
