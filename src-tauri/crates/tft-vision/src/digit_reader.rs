use image::{GrayImage, RgbaImage};
use std::process::Command;
use tracing::{debug, warn};

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
        let text = self.run_tesseract(&processed, "0123456789")?;
        text.parse::<u32>().ok()
    }

    /// Read a stage string (e.g., "3-2") from a cropped region
    pub fn read_stage(&self, image: &RgbaImage) -> Option<String> {
        if !self.tesseract_available {
            return None;
        }

        let processed = preprocess_for_ocr(image);
        let text = self.run_tesseract(&processed, "0123456789-")?;

        // Validate stage format (digit-digit)
        if text.contains('-') && text.len() >= 3 {
            Some(text)
        } else {
            None
        }
    }

    /// Run Tesseract on a pre-processed grayscale image
    fn run_tesseract(&self, image: &GrayImage, whitelist: &str) -> Option<String> {
        // Save to temp file
        let temp_path = self.temp_dir.join("ocr_input.png");
        if image.save(&temp_path).is_err() {
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
            return None;
        }

        let text = String::from_utf8(output.stdout).ok()?;
        let trimmed = text.trim().to_string();

        if trimmed.is_empty() {
            None
        } else {
            debug!("OCR result: '{}'", trimmed);
            Some(trimmed)
        }
    }
}

/// Pre-process an RGBA image for OCR:
/// 1. Convert to grayscale
/// 2. Threshold to isolate bright text (game UI text is light on dark)
/// 3. Invert so text is dark on white (Tesseract preference)
fn preprocess_for_ocr(image: &RgbaImage) -> GrayImage {
    let gray = image::imageops::grayscale(image);
    let (w, h) = gray.dimensions();

    GrayImage::from_fn(w, h, |x, y| {
        let pixel = gray.get_pixel(x, y)[0];
        // Threshold: bright pixels (text) become black, dark pixels (background) become white
        if pixel > 140 {
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
