mod champion_matcher;
mod digit_reader;
pub mod game_area;
pub mod layout;

pub use champion_matcher::{ChampionMatcher, MatchResult};
pub use digit_reader::DigitReader;
pub use game_area::{detect_game_area, GameArea};
pub use layout::{detect_layout, DetectedLayout};

use image::RgbaImage;
use serde::{Deserialize, Serialize};

/// Combined result from the vision pipeline for a single frame
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VisionResult {
    pub shop: Vec<ShopSlotResult>,
    pub gold: Option<u32>,
    pub level: Option<u32>,
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopSlotResult {
    pub slot_index: usize,
    pub champion_id: Option<String>,
    pub champion_name: Option<String>,
    pub cost: Option<u32>,
    pub confidence: f64,
}

/// Run the full vision pipeline on a captured frame.
/// Dynamically detects UI element positions instead of using fixed coordinates.
pub fn process_frame(
    frame: &RgbaImage,
    matcher: &ChampionMatcher,
    digit_reader: &DigitReader,
) -> VisionResult {
    let (w, h) = (frame.width(), frame.height());

    // Dynamically detect UI layout
    let layout = detect_layout(frame);

    // Process shop slots from detected positions
    let mut shop = Vec::with_capacity(5);
    for (i, region) in layout.shop_slots.iter().enumerate() {
        let crop = tft_capture::crop_region(frame, region);
        let portrait = extract_portrait(&crop);
        let result = matcher.match_champion(&portrait);
        shop.push(ShopSlotResult {
            slot_index: i,
            champion_id: result.as_ref().map(|r| r.champion_id.clone()),
            champion_name: result.as_ref().map(|r| r.champion_name.clone()),
            cost: result.as_ref().map(|r| r.cost),
            confidence: result.as_ref().map(|r| r.confidence).unwrap_or(0.0),
        });
    }

    // OCR using detected regions
    let gold = layout.gold.as_ref().and_then(|r| {
        let crop = tft_capture::crop_region(frame, r);
        tracing::debug!("Gold crop: {}x{}", crop.width(), crop.height());
        digit_reader.read_number(&crop)
    });

    let level = layout.level.as_ref().and_then(|r| {
        let crop = tft_capture::crop_region(frame, r);
        tracing::debug!("Level crop: {}x{}", crop.width(), crop.height());
        digit_reader.read_number(&crop)
    });

    let stage = layout.stage.as_ref().and_then(|r| {
        let crop = tft_capture::crop_region(frame, r);
        tracing::debug!("Stage crop: {}x{}", crop.width(), crop.height());
        digit_reader.read_stage(&crop)
    });

    tracing::debug!(
        "Vision: {} shop slots, gold={:?}, level={:?}, stage={:?} (frame {}x{}, hud_top={:.1}%)",
        shop.len(),
        gold,
        level,
        stage,
        w,
        h,
        layout.hud_top * 100.0,
    );

    VisionResult {
        shop,
        gold,
        level,
        stage,
    }
}

/// Extract the champion portrait area from a shop card crop.
/// The portrait is roughly the center 80% width and top 75% height of the card.
fn extract_portrait(card: &RgbaImage) -> RgbaImage {
    let (w, h) = (card.width(), card.height());
    if w < 4 || h < 4 {
        return card.clone();
    }
    let x = w / 10;
    let y = 0;
    let pw = w * 8 / 10;
    let ph = h * 3 / 4;
    image::imageops::crop_imm(card, x, y, pw.max(1), ph.max(1)).to_image()
}
