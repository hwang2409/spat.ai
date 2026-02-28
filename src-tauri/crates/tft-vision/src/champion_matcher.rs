use anyhow::{Context, Result};
use image::{GrayImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

/// Standard size for template matching (both templates and crops are resized to this)
const MATCH_SIZE: u32 = 48;

/// Minimum confidence to consider a match valid
const MIN_CONFIDENCE: f64 = 0.4;

/// Result of matching a shop slot against champion templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub champion_id: String,
    pub champion_name: String,
    pub cost: u32,
    pub confidence: f64,
}

/// Pre-processed champion template for matching
struct ChampionTemplate {
    id: String,
    name: String,
    cost: u32,
    gray: GrayImage,
    /// Pre-computed mean and std for NCC
    mean: f64,
    std_dev: f64,
}

/// Matches shop slot images against champion icon templates
pub struct ChampionMatcher {
    templates: Vec<ChampionTemplate>,
}

impl ChampionMatcher {
    /// Load champion templates from the data directory.
    /// Expects:
    ///   - data_dir/champions.json (champion metadata)
    ///   - data_dir/templates/champions/{id}.png (icon images)
    pub fn load(data_dir: &Path) -> Result<Self> {
        let game_data = tft_data::GameData::load(data_dir)?;
        let templates_dir = data_dir.join("templates").join("champions");

        let mut templates = Vec::new();

        for (id, champ) in &game_data.champions {
            let icon_path = templates_dir.join(&champ.icon);
            if !icon_path.exists() {
                debug!("Missing icon for {}: {}", id, icon_path.display());
                continue;
            }

            match load_template(&icon_path, id, &champ.name, champ.cost) {
                Ok(tmpl) => templates.push(tmpl),
                Err(e) => warn!("Failed to load template for {}: {}", id, e),
            }
        }

        info!(
            "ChampionMatcher loaded {} templates from {}",
            templates.len(),
            templates_dir.display()
        );

        Ok(Self { templates })
    }

    /// Match a shop slot image against all templates.
    /// Returns the best match above the confidence threshold, or None.
    pub fn match_champion(&self, slot_image: &RgbaImage) -> Option<MatchResult> {
        if self.templates.is_empty() {
            return None;
        }

        // Convert and resize the input image
        let gray = image::imageops::grayscale(slot_image);
        let resized = image::imageops::resize(
            &gray,
            MATCH_SIZE,
            MATCH_SIZE,
            image::imageops::FilterType::Triangle,
        );

        let (input_mean, input_std) = compute_stats(&resized);

        // Skip if the input is nearly uniform (probably empty slot)
        if input_std < 5.0 {
            return None;
        }

        let mut best_score = f64::NEG_INFINITY;
        let mut best_idx = 0;

        for (i, tmpl) in self.templates.iter().enumerate() {
            let score = normalized_cross_correlation(
                &resized,
                input_mean,
                input_std,
                &tmpl.gray,
                tmpl.mean,
                tmpl.std_dev,
            );
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        if best_score >= MIN_CONFIDENCE {
            let tmpl = &self.templates[best_idx];
            Some(MatchResult {
                champion_id: tmpl.id.clone(),
                champion_name: tmpl.name.clone(),
                cost: tmpl.cost,
                confidence: best_score,
            })
        } else {
            None
        }
    }

    /// Number of loaded templates
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }
}

/// Load and pre-process a single champion icon template
fn load_template(path: &Path, id: &str, name: &str, cost: u32) -> Result<ChampionTemplate> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    let gray = img.to_luma8();
    let resized = image::imageops::resize(
        &gray,
        MATCH_SIZE,
        MATCH_SIZE,
        image::imageops::FilterType::Triangle,
    );
    let (mean, std_dev) = compute_stats(&resized);

    Ok(ChampionTemplate {
        id: id.to_string(),
        name: name.to_string(),
        cost,
        gray: resized,
        mean,
        std_dev,
    })
}

/// Compute mean and standard deviation of pixel values
fn compute_stats(img: &GrayImage) -> (f64, f64) {
    let pixels: Vec<f64> = img.pixels().map(|p| p[0] as f64).collect();
    let n = pixels.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mean = pixels.iter().sum::<f64>() / n;
    let variance = pixels.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n;
    (mean, variance.sqrt())
}

/// Zero-mean Normalized Cross-Correlation between two same-sized images.
/// Returns a value between -1.0 (inverse) and 1.0 (perfect match).
fn normalized_cross_correlation(
    img: &GrayImage,
    img_mean: f64,
    img_std: f64,
    tmpl: &GrayImage,
    tmpl_mean: f64,
    tmpl_std: f64,
) -> f64 {
    debug_assert_eq!(img.dimensions(), tmpl.dimensions());

    let denom = img_std * tmpl_std;
    if denom < 1e-10 {
        return 0.0;
    }

    let n = (img.width() * img.height()) as f64;
    let cross: f64 = img
        .pixels()
        .zip(tmpl.pixels())
        .map(|(ip, tp)| (ip[0] as f64 - img_mean) * (tp[0] as f64 - tmpl_mean))
        .sum();

    cross / (n * denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ncc_identical() {
        let img = GrayImage::from_fn(48, 48, |x, y| {
            image::Luma([(x.wrapping_mul(7).wrapping_add(y.wrapping_mul(13)) % 256) as u8])
        });
        let (mean, std) = compute_stats(&img);
        let score = normalized_cross_correlation(&img, mean, std, &img, mean, std);
        assert!(
            (score - 1.0).abs() < 0.001,
            "Identical images should have NCC ≈ 1.0, got {}",
            score
        );
    }

    #[test]
    fn test_ncc_different() {
        let img1 = GrayImage::from_fn(48, 48, |x, _| image::Luma([(x % 256) as u8]));
        let img2 = GrayImage::from_fn(48, 48, |_, y| image::Luma([(y % 256) as u8]));
        let (m1, s1) = compute_stats(&img1);
        let (m2, s2) = compute_stats(&img2);
        let score = normalized_cross_correlation(&img1, m1, s1, &img2, m2, s2);
        assert!(
            score < 0.5,
            "Different images should have low NCC, got {}",
            score
        );
    }

    #[test]
    fn test_uniform_image_returns_none() {
        let matcher = ChampionMatcher {
            templates: vec![],
        };
        let img = RgbaImage::from_pixel(100, 100, image::Rgba([128, 128, 128, 255]));
        assert!(matcher.match_champion(&img).is_none());
    }
}
