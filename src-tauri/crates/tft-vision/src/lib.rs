use serde::{Deserialize, Serialize};

/// Result of template matching for a single champion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub champion_id: String,
    pub confidence: f64,
}

/// Placeholder for the vision pipeline.
/// Will be implemented in Phase 2 with template matching.
pub fn recognize_champion(_slot_image: &image::RgbaImage) -> Option<MatchResult> {
    // TODO: Phase 2 - template matching against champion icons
    None
}
