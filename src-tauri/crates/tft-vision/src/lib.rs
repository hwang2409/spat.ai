mod champion_matcher;
mod digit_reader;

pub use champion_matcher::{ChampionMatcher, MatchResult};
pub use digit_reader::DigitReader;

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

/// Run the full vision pipeline on a captured frame
pub fn process_frame(
    frame: &RgbaImage,
    matcher: &ChampionMatcher,
    digit_reader: &DigitReader,
) -> VisionResult {
    let (w, h) = (frame.width(), frame.height());

    // Process shop slots
    let mut shop = Vec::with_capacity(5);
    for i in 0..5 {
        let region = tft_capture::regions::shop_slot(i);
        let crop = tft_capture::crop_region(frame, &region);

        // Extract portrait area from the shop card (center portion)
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

    // OCR for gold, level, stage
    let gold_region = tft_capture::regions::gold();
    let gold_crop = tft_capture::crop_region(frame, &gold_region);
    let gold = digit_reader.read_number(&gold_crop);

    let level_region = tft_capture::regions::level();
    let level_crop = tft_capture::crop_region(frame, &level_region);
    let level = digit_reader.read_number(&level_crop);

    let stage_region = tft_capture::regions::stage();
    let stage_crop = tft_capture::crop_region(frame, &stage_region);
    let stage = digit_reader.read_stage(&stage_crop);

    tracing::debug!(
        "Vision: {} shop slots processed, gold={:?}, level={:?}, stage={:?} (frame {}x{})",
        shop.len(),
        gold,
        level,
        stage,
        w,
        h,
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
