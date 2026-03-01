pub mod video;

use image::RgbaImage;
use serde::{Deserialize, Serialize};

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
