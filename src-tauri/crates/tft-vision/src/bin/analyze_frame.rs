//! CLI tool to analyze a saved TFT screenshot through the full vision pipeline.
//! Usage: cargo run --bin analyze_frame -- <path_to_screenshot.png>

use std::path::PathBuf;
use tft_vision::{detect_layout, DigitReader};

fn main() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <screenshot.png> [output_dir]", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let output_dir = if args.len() >= 3 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("./debug_output")
    };
    let _ = std::fs::create_dir_all(&output_dir);

    println!("Loading image: {}", input_path.display());
    let img = image::open(&input_path)
        .expect("Failed to open image")
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    println!("Image size: {}x{}", w, h);

    // Detect layout
    println!("\n=== Layout Detection ===");
    let layout = detect_layout(&img);
    println!("HUD top: {:.1}% (y={:.0})", layout.hud_top * 100.0, layout.hud_top * h as f64);
    println!("Shop slots: {}", layout.shop_slots.len());
    for (i, slot) in layout.shop_slots.iter().enumerate() {
        println!(
            "  Slot {}: x={:.0} y={:.0} w={:.0} h={:.0}",
            i,
            slot.x * w as f64,
            slot.y * h as f64,
            slot.width * w as f64,
            slot.height * h as f64,
        );
        let crop = tft_capture::crop_region(&img, slot);
        let _ = crop.save(output_dir.join(format!("shop_slot_{}.png", i)));
    }

    if let Some(ref r) = layout.gold {
        println!(
            "Gold region: x={:.0} y={:.0} w={:.0} h={:.0}",
            r.x * w as f64, r.y * h as f64, r.width * w as f64, r.height * h as f64,
        );
        let crop = tft_capture::crop_region(&img, r);
        let _ = crop.save(output_dir.join("gold_crop.png"));
    } else {
        println!("Gold region: NOT FOUND");
    }

    if let Some(ref r) = layout.level {
        println!(
            "Level region: x={:.0} y={:.0} w={:.0} h={:.0}",
            r.x * w as f64, r.y * h as f64, r.width * w as f64, r.height * h as f64,
        );
        let crop = tft_capture::crop_region(&img, r);
        let _ = crop.save(output_dir.join("level_crop.png"));
    } else {
        println!("Level region: NOT FOUND");
    }

    if let Some(ref r) = layout.stage {
        println!(
            "Stage region: x={:.0} y={:.0} w={:.0} h={:.0}",
            r.x * w as f64, r.y * h as f64, r.width * w as f64, r.height * h as f64,
        );
        let crop = tft_capture::crop_region(&img, r);
        let _ = crop.save(output_dir.join("stage_crop.png"));
    } else {
        println!("Stage region: NOT FOUND");
    }

    // OCR
    println!("\n=== OCR Results ===");
    let digit_reader = DigitReader::new();
    if !digit_reader.is_available() {
        println!("Tesseract not available! Install with: brew install tesseract");
        return;
    }

    if let Some(ref r) = layout.gold {
        let crop = tft_capture::crop_region(&img, r);
        let result = digit_reader.read_number(&crop);
        println!("Gold: {:?}", result);
    }

    if let Some(ref r) = layout.level {
        let crop = tft_capture::crop_region(&img, r);
        let result = digit_reader.read_number(&crop);
        println!("Level: {:?}", result);
    }

    if let Some(ref r) = layout.stage {
        let crop = tft_capture::crop_region(&img, r);
        let result = digit_reader.read_stage(&crop);
        println!("Stage: {:?}", result);
    }

    println!("\nDebug images saved to: {}", output_dir.display());
}
