use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
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

    /// Shop card slot. Each card is ~139px wide, ~180px tall at 1080p.
    pub fn shop_slot(index: usize) -> ScreenRegion {
        let slot_width = 0.0724;
        let slot_height = 0.167;
        let start_x = 0.284;
        let stride = 0.0755;
        ScreenRegion {
            x: start_x + (index as f64) * stride,
            y: 0.769,
            width: slot_width,
            height: slot_height,
        }
    }

    pub fn gold() -> ScreenRegion {
        ScreenRegion {
            x: 0.870,
            y: 0.880,
            width: 0.035,
            height: 0.025,
        }
    }

    pub fn level() -> ScreenRegion {
        ScreenRegion {
            x: 0.255,
            y: 0.890,
            width: 0.020,
            height: 0.025,
        }
    }

    pub fn stage() -> ScreenRegion {
        ScreenRegion {
            x: 0.465,
            y: 0.005,
            width: 0.070,
            height: 0.030,
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

/// Info about a visible window on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub app_name: String,
    pub width: u32,
    pub height: u32,
}

/// Result of listing windows, includes diagnostic info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowListResult {
    pub windows: Vec<WindowInfo>,
    /// Total number of windows returned by the OS (before filtering)
    pub raw_count: usize,
    /// Whether screen recording permission appears to be granted
    pub has_permission: bool,
}

/// Default TFT-related window title substrings (matched case-insensitively)
const DEFAULT_TITLE_PATTERNS: &[&str] = &[
    "league of legends",
    "riot games",
    "tft",
    "teamfight tactics",
    "leagueclient",
];

/// Shared target window title — can be set by the user to override auto-detection
pub type TargetWindow = Arc<RwLock<Option<String>>>;

/// Create a new shared target window handle
pub fn new_target_window() -> TargetWindow {
    Arc::new(RwLock::new(None))
}

/// Helper to safely get window metadata
fn window_meta(window: &Window) -> (String, String, u32, u32) {
    let title = window.title().unwrap_or_default();
    let app_name = window.app_name().unwrap_or_default();
    let width = window.width().unwrap_or(0);
    let height = window.height().unwrap_or(0);
    (title, app_name, width, height)
}

/// Check if screen recording permission is likely granted (macOS).
/// On macOS, CGWindowListCopyWindowInfo returns only the caller's windows
/// without Screen Recording permission. We detect this by checking if we
/// can see windows from other apps.
fn check_screen_recording_permission(windows: &[Window]) -> bool {
    // If we can see at least a few different app names, permission is granted
    let mut app_names = std::collections::HashSet::new();
    for w in windows {
        let app = w.app_name().unwrap_or_default();
        if !app.is_empty() {
            app_names.insert(app);
        }
    }
    // If we see 3+ different apps, we almost certainly have permission
    // If we see 0-1, we likely don't (only seeing our own app)
    let has_perm = app_names.len() >= 3;
    if !has_perm {
        warn!(
            "Screen Recording permission likely not granted. Only seeing apps: {:?}",
            app_names
        );
    }
    has_perm
}

/// List all visible windows on the system, with diagnostic info
pub fn list_windows() -> WindowListResult {
    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            warn!("Failed to enumerate windows: {}", e);
            return WindowListResult {
                windows: Vec::new(),
                raw_count: 0,
                has_permission: false,
            };
        }
    };

    let raw_count = windows.len();
    let has_permission = check_screen_recording_permission(&windows);

    // Log all windows for debugging
    for w in &windows {
        let (title, app_name, width, height) = window_meta(w);
        debug!(
            "  window: '{}' app='{}' {}x{}",
            title, app_name, width, height
        );
    }

    let filtered: Vec<WindowInfo> = windows
        .iter()
        .filter_map(|w| {
            let (title, app_name, width, height) = window_meta(w);
            if title.is_empty() || width < 100 || height < 100 {
                return None;
            }
            Some(WindowInfo {
                title,
                app_name,
                width,
                height,
            })
        })
        .collect();

    info!(
        "Window scan: {} raw, {} visible, permission={}",
        raw_count,
        filtered.len(),
        has_permission
    );

    WindowListResult {
        windows: filtered,
        raw_count,
        has_permission,
    }
}

/// Find and capture a frame from the target window.
/// Returns (title, frame) on success.
fn find_and_capture(target: &TargetWindow) -> Option<(String, RgbaImage)> {
    let windows = match Window::all() {
        Ok(w) => w,
        Err(e) => {
            warn!("Failed to enumerate windows: {}", e);
            return None;
        }
    };

    let user_target = target.read().ok().and_then(|t| t.clone());

    // Find the matching window
    let matched = if let Some(ref target_title) = user_target {
        let target_lower = target_title.to_lowercase();
        windows.iter().find(|w| {
            w.title()
                .unwrap_or_default()
                .to_lowercase()
                .contains(&target_lower)
        })
    } else {
        // Auto-detect using known title patterns
        windows.iter().find(|w| {
            let title_lower = w.title().unwrap_or_default().to_lowercase();
            DEFAULT_TITLE_PATTERNS
                .iter()
                .any(|p| title_lower.contains(p))
        })
    };

    match matched {
        Some(window) => {
            let title = window.title().unwrap_or_default();
            match window.capture_image() {
                Ok(img) => {
                    debug!("Captured window: {}", title);
                    Some((title, img))
                }
                Err(e) => {
                    warn!("Capture failed for '{}': {}", title, e);
                    None
                }
            }
        }
        None => {
            // Log available windows at debug level
            let available: Vec<String> = windows
                .iter()
                .filter_map(|w| {
                    let (title, _, width, height) = window_meta(w);
                    if title.is_empty() || width < 100 || height < 100 {
                        None
                    } else {
                        Some(format!("'{}' ({}x{})", title, width, height))
                    }
                })
                .collect();
            debug!("No target window found. Available: {}", available.join(", "));
            None
        }
    }
}

/// Crop a region from a captured frame using normalized coordinates
pub fn crop_region(frame: &RgbaImage, region: &ScreenRegion) -> RgbaImage {
    let (w, h) = (frame.width(), frame.height());
    let x = (region.x * w as f64) as u32;
    let y = (region.y * h as f64) as u32;
    let rw = (region.width * w as f64) as u32;
    let rh = (region.height * h as f64) as u32;

    let x = x.min(w.saturating_sub(1));
    let y = y.min(h.saturating_sub(1));
    let rw = rw.min(w - x);
    let rh = rh.min(h - y);

    image::imageops::crop_imm(frame, x, y, rw, rh).to_image()
}

/// The capture loop that runs as a background task.
pub async fn capture_loop(
    frame_tx: watch::Sender<Option<Arc<RgbaImage>>>,
    status_tx: watch::Sender<CaptureStatus>,
    capture_interval: Duration,
    stop: Arc<AtomicBool>,
    target: TargetWindow,
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

        // Find and capture the target window on a blocking thread
        let target_clone = target.clone();
        let capture_result =
            tokio::task::spawn_blocking(move || find_and_capture(&target_clone)).await;

        match capture_result {
            Ok(Some((title, frame))) => {
                let resolution = (frame.width(), frame.height());
                frame_count += 1;

                let elapsed = fps_timer.elapsed().as_secs_f64();
                let fps = if elapsed > 0.0 {
                    frame_count as f64 / elapsed
                } else {
                    0.0
                };

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
            Ok(None) => {
                let _ = status_tx.send(CaptureStatus::default());
            }
            Err(e) => {
                warn!("Capture task panicked: {}", e);
            }
        }

        // Sleep until next capture interval
        let elapsed = last_capture.elapsed();
        if elapsed < capture_interval {
            tokio::time::sleep(capture_interval - elapsed).await;
        } else {
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
            assert!(region.x + region.width <= 1.01);
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
