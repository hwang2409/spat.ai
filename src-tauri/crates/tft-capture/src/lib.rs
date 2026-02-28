use anyhow::{Context, Result};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tracing::{debug, info, warn};
use xcap::Window;

/// Normalized screen region (0.0-1.0 coordinates relative to game window)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Well-known screen regions for a 1920x1080 reference resolution
pub mod regions {
    use super::ScreenRegion;

    pub fn shop_slot(index: usize) -> ScreenRegion {
        let slot_width = 0.104;
        let start_x = 0.252;
        let gap = 0.0;
        ScreenRegion {
            x: start_x + (index as f64) * (slot_width + gap),
            y: 0.935,
            width: slot_width,
            height: 0.055,
        }
    }

    pub fn gold() -> ScreenRegion {
        ScreenRegion {
            x: 0.45,
            y: 0.82,
            width: 0.03,
            height: 0.02,
        }
    }

    pub fn level() -> ScreenRegion {
        ScreenRegion {
            x: 0.22,
            y: 0.95,
            width: 0.02,
            height: 0.02,
        }
    }

    pub fn stage() -> ScreenRegion {
        ScreenRegion {
            x: 0.47,
            y: 0.0,
            width: 0.06,
            height: 0.03,
        }
    }
}

/// Status of the capture pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus {
    pub is_capturing: bool,
    pub window_found: bool,
    pub window_title: Option<String>,
    pub fps: f64,
    pub last_capture_time: Option<u64>,
    pub resolution: Option<(u32, u32)>,
}

impl Default for CaptureStatus {
    fn default() -> Self {
        Self {
            is_capturing: false,
            window_found: false,
            window_title: None,
            fps: 0.0,
            last_capture_time: None,
            resolution: None,
        }
    }
}

/// TFT window titles to search for
const TFT_WINDOW_TITLES: &[&str] = &[
    "league of legends (tm) client",
    "league of legends",
    "riot games",
    "tft",
    "teamfight tactics",
];

/// Find the TFT game window by searching window titles
fn find_tft_window() -> Option<Window> {
    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            warn!("Failed to enumerate windows: {}", e);
            return None;
        }
    };

    for window in windows {
        let title = match window.title() {
            Ok(t) => t.to_lowercase(),
            Err(_) => continue,
        };
        if TFT_WINDOW_TITLES
            .iter()
            .any(|t| title.contains(t))
        {
            debug!("Found TFT window: {}", title);
            return Some(window);
        }
    }
    None
}

/// Capture a frame from the given window
fn capture_frame(window: &Window) -> Result<RgbaImage> {
    let img = window
        .capture_image()
        .context("Failed to capture window image")?;
    Ok(img)
}

/// Crop a region from a captured frame using normalized coordinates
pub fn crop_region(frame: &RgbaImage, region: &ScreenRegion) -> RgbaImage {
    let (w, h) = (frame.width(), frame.height());
    let x = (region.x * w as f64) as u32;
    let y = (region.y * h as f64) as u32;
    let rw = (region.width * w as f64) as u32;
    let rh = (region.height * h as f64) as u32;

    // Clamp to image bounds
    let x = x.min(w.saturating_sub(1));
    let y = y.min(h.saturating_sub(1));
    let rw = rw.min(w - x);
    let rh = rh.min(h - y);

    image::imageops::crop_imm(frame, x, y, rw, rh).to_image()
}

/// The capture loop that runs as a background task.
/// Sends frames through the watch channel and status updates through the status channel.
pub async fn capture_loop(
    frame_tx: watch::Sender<Option<Arc<RgbaImage>>>,
    status_tx: watch::Sender<CaptureStatus>,
    capture_interval: Duration,
    stop: Arc<AtomicBool>,
) {
    info!("Capture loop started, interval: {:?}", capture_interval);

    let mut last_capture = Instant::now();
    let mut frame_count = 0u64;
    let mut fps_timer = Instant::now();

    loop {
        if stop.load(Ordering::Relaxed) {
            info!("Capture loop stopping (stop signal received)");
            break;
        }

        // Try to find the TFT window
        let window = find_tft_window();

        match window {
            Some(win) => {
                let title = win.title().unwrap_or_default();

                // Capture frame on a blocking thread (xcap is sync)
                let capture_result = tokio::task::spawn_blocking(move || {
                    capture_frame(&win)
                })
                .await;

                match capture_result {
                    Ok(Ok(frame)) => {
                        let resolution = (frame.width(), frame.height());
                        frame_count += 1;

                        // Calculate FPS
                        let elapsed = fps_timer.elapsed().as_secs_f64();
                        let fps = if elapsed > 0.0 {
                            frame_count as f64 / elapsed
                        } else {
                            0.0
                        };

                        // Reset FPS counter every 5 seconds
                        if elapsed > 5.0 {
                            frame_count = 0;
                            fps_timer = Instant::now();
                        }

                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        let _ = status_tx.send(CaptureStatus {
                            is_capturing: true,
                            window_found: true,
                            window_title: Some(title),
                            fps,
                            last_capture_time: Some(now),
                            resolution: Some(resolution),
                        });

                        let _ = frame_tx.send(Some(Arc::new(frame)));
                        last_capture = Instant::now();
                    }
                    Ok(Err(e)) => {
                        warn!("Capture failed: {}", e);
                        let _ = status_tx.send(CaptureStatus {
                            is_capturing: false,
                            window_found: true,
                            window_title: Some(title),
                            fps: 0.0,
                            last_capture_time: None,
                            resolution: None,
                        });
                    }
                    Err(e) => {
                        warn!("Capture task panicked: {}", e);
                    }
                }
            }
            None => {
                let _ = status_tx.send(CaptureStatus::default());
            }
        }

        // Sleep until next capture interval
        let elapsed = last_capture.elapsed();
        if elapsed < capture_interval {
            tokio::time::sleep(capture_interval - elapsed).await;
        } else {
            // Yield to prevent busy loop
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    let _ = status_tx.send(CaptureStatus::default());
    info!("Capture loop stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shop_slot_regions() {
        for i in 0..5 {
            let region = regions::shop_slot(i);
            assert!(region.x >= 0.0 && region.x <= 1.0);
            assert!(region.y >= 0.0 && region.y <= 1.0);
            assert!(region.x + region.width <= 1.01); // small tolerance
        }
    }

    #[test]
    fn test_crop_region() {
        let img = RgbaImage::new(1920, 1080);
        let region = ScreenRegion {
            x: 0.5,
            y: 0.5,
            width: 0.1,
            height: 0.1,
        };
        let cropped = crop_region(&img, &region);
        assert_eq!(cropped.width(), 192);
        assert_eq!(cropped.height(), 108);
    }
}
