use image::{GrayImage, RgbaImage};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{debug, warn};

/// Counter to save debug OCR images for the first few invocations
static OCR_DEBUG_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Reads digits from cropped game UI regions using Tesseract OCR.
/// Falls back gracefully when Tesseract is not installed.
pub struct DigitReader {
    tesseract_available: bool,
    temp_dir: std::path::PathBuf,
}

impl DigitReader {
    pub fn new() -> Self {
        let tesseract_available = check_tesseract();
        if tesseract_available {
            debug!("Tesseract OCR available");
        } else {
            warn!("Tesseract not found. OCR disabled. Install with: brew install tesseract");
        }

        let temp_dir = std::env::temp_dir().join("spat_ai_ocr");
        let _ = std::fs::create_dir_all(&temp_dir);

        Self {
            tesseract_available,
            temp_dir,
        }
    }

    /// Check if OCR is available
    pub fn is_available(&self) -> bool {
        self.tesseract_available
    }

    /// Read a number (e.g., gold count, level) from a cropped region
    pub fn read_number(&self, image: &RgbaImage) -> Option<u32> {
        if !self.tesseract_available {
            return None;
        }

        let processed = preprocess_for_ocr(image);
        self.save_debug_ocr(image, &processed, "number");
        let text = self.run_tesseract(&processed, "0123456789")?;
        text.parse::<u32>().ok()
    }

    /// Read a stage string (e.g., "3-2") from a cropped region
    pub fn read_stage(&self, image: &RgbaImage) -> Option<String> {
        if !self.tesseract_available {
            return None;
        }

        let processed = preprocess_for_ocr(image);
        self.save_debug_ocr(image, &processed, "stage");
        let text = self.run_tesseract(&processed, "0123456789-")?;

        // Validate stage format (digit-digit)
        if text.contains('-') && text.len() >= 3 {
            Some(text)
        } else {
            None
        }
    }

    /// Save debug OCR images (raw + processed) for the first few invocations
    fn save_debug_ocr(&self, raw: &RgbaImage, processed: &GrayImage, label: &str) {
        let count = OCR_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
        if count < 15 {
            let debug_dir = std::env::temp_dir().join("spat_ai_debug");
            let _ = std::fs::create_dir_all(&debug_dir);
            let _ = raw.save(debug_dir.join(format!("ocr_{}_raw_{}.png", label, count)));
            let _ = processed.save(debug_dir.join(format!("ocr_{}_processed_{}.png", label, count)));
        }
    }

    /// Run Tesseract on a pre-processed grayscale image
    fn run_tesseract(&self, image: &GrayImage, whitelist: &str) -> Option<String> {
        let (w, h) = image.dimensions();
        debug!("OCR input: {}x{} whitelist='{}'", w, h, whitelist);

        // Skip images that are too small for OCR
        if w < 5 || h < 5 {
            debug!("OCR skipped: image too small ({}x{})", w, h);
            return None;
        }

        // Save to temp file
        let temp_path = self.temp_dir.join("ocr_input.png");
        if let Err(e) = image.save(&temp_path) {
            debug!("OCR: failed to save temp image: {}", e);
            return None;
        }

        let output = Command::new("tesseract")
            .arg(&temp_path)
            .arg("stdout")
            .arg("--psm")
            .arg("7") // Single text line
            .arg("-c")
            .arg(format!("tessedit_char_whitelist={}", whitelist))
            .output()
            .ok()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("OCR: tesseract failed (status={}): {}", output.status, stderr);
            return None;
        }

        let text = String::from_utf8(output.stdout).ok()?;
        let trimmed = text.trim().to_string();

        if trimmed.is_empty() {
            debug!("OCR: tesseract returned empty result");
            None
        } else {
            debug!("OCR result: '{}'", trimmed);
            Some(trimmed)
        }
    }
}

/// Pre-process an RGBA image for OCR:
/// 1. Upscale small images (Tesseract works better with larger input)
/// 2. Convert to grayscale
/// 3. Adaptive threshold to isolate bright text (game UI text is light on dark)
/// 4. Invert so text is dark on white (Tesseract preference)
fn preprocess_for_ocr(image: &RgbaImage) -> GrayImage {
    // Upscale if the image is small (Tesseract needs ~30+ px character height)
    let (w, h) = image.dimensions();
    let scale = if h < 30 { 3u32 } else if h < 60 { 2u32 } else { 1u32 };
    let scaled = if scale > 1 {
        image::imageops::resize(
            image,
            w * scale,
            h * scale,
            image::imageops::FilterType::Nearest,
        )
    } else {
        image.clone()
    };

    let gray = image::imageops::grayscale(&scaled);
    let (gw, gh) = gray.dimensions();

    // Compute adaptive threshold from image content:
    // find the brightness of the brightest 15% of pixels (likely text)
    // and use a threshold between text and background
    let mut pixels: Vec<u8> = gray.pixels().map(|p| p[0]).collect();
    pixels.sort();
    let bright_ref = pixels[pixels.len() * 85 / 100] as f64;
    let dark_ref = pixels[pixels.len() * 30 / 100] as f64;
    let threshold = (dark_ref + (bright_ref - dark_ref) * 0.4) as u8;

    debug!(
        "OCR preprocess: {}x{} → {}x{} (scale={}), threshold={} (dark={:.0}, bright={:.0})",
        w, h, gw, gh, scale, threshold, dark_ref, bright_ref
    );

    GrayImage::from_fn(gw, gh, |x, y| {
        let pixel = gray.get_pixel(x, y)[0];
        if pixel > threshold {
            image::Luma([0u8]) // Text → black
        } else {
            image::Luma([255u8]) // Background → white
        }
    })
}

/// Check if Tesseract is installed and accessible
fn check_tesseract() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess() {
        let img = RgbaImage::from_fn(10, 10, |x, _| {
            if x < 5 {
                image::Rgba([200, 200, 200, 255]) // bright → should become black (text)
            } else {
                image::Rgba([30, 30, 30, 255]) // dark → should become white (bg)
            }
        });
        let processed = preprocess_for_ocr(&img);
        assert_eq!(processed.get_pixel(0, 0)[0], 0); // text is black
        assert_eq!(processed.get_pixel(9, 0)[0], 255); // bg is white
    }
}
