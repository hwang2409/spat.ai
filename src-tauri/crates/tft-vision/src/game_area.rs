use image::RgbaImage;
use tft_capture::ScreenRegion;
use tracing::debug;

/// Detected TFT game area within an arbitrary video frame.
#[derive(Debug, Clone)]
pub struct GameArea {
    /// Normalized coordinates (0.0-1.0) of the game area within the frame.
    pub region: ScreenRegion,
    pub confidence: f64,
}

/// Detect the TFT game window boundary within an arbitrary video frame.
///
/// Videos may show TFT in a window with desktop content, streamer overlays, etc.
/// This function finds the TFT game area by looking for the distinctive bottom
/// HUD bar (dark strip with shop cards below it).
///
/// Returns `None` if no TFT game area is detected.
pub fn detect_game_area(frame: &RgbaImage) -> Option<GameArea> {
    let (w, h) = (frame.width(), frame.height());
    if w < 100 || h < 100 {
        return None;
    }

    // Find candidate HUD bars across the full frame
    let candidates = find_hud_candidates(frame);

    if candidates.is_empty() {
        debug!("No HUD bar candidates found");
        return None;
    }

    debug!("Found {} HUD bar candidate(s)", candidates.len());

    // For each candidate, check for the 5-card shop pattern below it
    let mut best: Option<(HudCandidate, f64)> = None;
    for candidate in &candidates {
        let card_score = score_card_pattern(frame, candidate);
        debug!(
            "  HUD candidate y={} (brightness drop {:.1}): card_score={:.2}",
            candidate.y, candidate.drop, card_score
        );
        if card_score > 0.3 {
            let combined = card_score * 0.7 + (candidate.drop / 100.0).min(1.0) * 0.3;
            if best.as_ref().map_or(true, |(_, s)| combined > *s) {
                best = Some((candidate.clone(), combined));
            }
        }
    }

    let (hud, confidence) = best?;

    // Infer game bounds from the detected HUD position
    // The HUD bar sits at ~80% of game height
    let hud_fraction = 0.80;
    let game_height_est = (hud.y as f64) / hud_fraction;

    // Infer the game top
    let game_top = (hud.y as f64 - game_height_est * hud_fraction).max(0.0);
    let game_bottom = (game_top + game_height_est).min(h as f64);
    let actual_game_height = game_bottom - game_top;

    // Infer width from 16:9 aspect ratio
    let ideal_width = actual_game_height * 16.0 / 9.0;
    let game_width = ideal_width.min(w as f64);
    let game_left = ((w as f64 - game_width) / 2.0).max(0.0);

    // Fullscreen fast-path: if the game area covers most of the frame, return full frame
    let area_ratio = (actual_game_height * game_width) / (w as f64 * h as f64);
    if area_ratio > 0.85 {
        debug!(
            "Fullscreen fast-path: game area covers {:.0}% of frame",
            area_ratio * 100.0
        );
        return Some(GameArea {
            region: ScreenRegion {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            confidence,
        });
    }

    let region = ScreenRegion {
        x: game_left / w as f64,
        y: game_top / h as f64,
        width: game_width / w as f64,
        height: actual_game_height / h as f64,
    };

    debug!(
        "Game area detected: x={:.0} y={:.0} w={:.0} h={:.0} (confidence={:.2})",
        game_left, game_top, game_width, actual_game_height, confidence
    );

    Some(GameArea { region, confidence })
}

#[derive(Debug, Clone)]
struct HudCandidate {
    /// Y coordinate of the top of the dark bar
    y: u32,
    /// Brightness drop magnitude
    drop: f64,
}

/// Scan the full frame for horizontal dark bands that could be the TFT HUD bar.
fn find_hud_candidates(frame: &RgbaImage) -> Vec<HudCandidate> {
    let h = frame.height();
    let window = 5u32;
    let mut candidates = Vec::new();

    // Scan the full frame (not just 74-90% like layout.rs does)
    // because the game could be anywhere in the frame
    let search_top = h / 10;
    let search_bottom = h.saturating_sub(h / 10);
    let end = search_bottom.saturating_sub(window * 2);

    if search_top >= end {
        return candidates;
    }

    let mut prev_drop = 0.0f64;
    let mut prev_y = 0u32;

    for y in search_top..end {
        let above = avg_brightness_rows(frame, y, window);
        let below = avg_brightness_rows(frame, y + window, window);
        let drop = above - below;

        // Look for significant brightness drops where below is dark
        if drop > 15.0 && below < 55.0 {
            // Deduplicate: only keep the best drop in a local region
            if y.saturating_sub(prev_y) < 20 {
                if drop > prev_drop {
                    // Replace the previous candidate
                    if let Some(last) = candidates.last_mut() {
                        *last = HudCandidate {
                            y: y + window,
                            drop,
                        };
                    }
                    prev_drop = drop;
                    prev_y = y;
                }
            } else {
                candidates.push(HudCandidate {
                    y: y + window,
                    drop,
                });
                prev_drop = drop;
                prev_y = y;
            }
        }
    }

    candidates
}

/// Average brightness of sampled pixels in a row (middle 60% of width).
fn row_brightness(frame: &RgbaImage, y: u32) -> f64 {
    let w = frame.width();
    let step = (w / 50).max(1);
    let x_start = w / 5;
    let x_end = w * 4 / 5;

    let mut sum = 0.0;
    let mut count = 0u32;
    let mut x = x_start;
    while x < x_end {
        let px = frame.get_pixel(x, y);
        sum += (px[0] as f64 + px[1] as f64 + px[2] as f64) / 3.0;
        count += 1;
        x += step;
    }
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

/// Average brightness over `count` consecutive rows.
fn avg_brightness_rows(frame: &RgbaImage, start_y: u32, count: u32) -> f64 {
    let h = frame.height();
    let mut sum = 0.0;
    let mut n = 0u32;
    for i in 0..count {
        let y = start_y + i;
        if y < h {
            sum += row_brightness(frame, y);
            n += 1;
        }
    }
    if n > 0 {
        sum / n as f64
    } else {
        0.0
    }
}

/// Score how well the region below a HUD candidate matches the 5-card shop pattern.
///
/// Returns a score from 0.0 (no match) to 1.0 (perfect match).
fn score_card_pattern(frame: &RgbaImage, candidate: &HudCandidate) -> f64 {
    let (w, h) = (frame.width(), frame.height());
    let hud_y = candidate.y;

    // Cards occupy approximately the bottom 20% of the game area, below the HUD bar.
    // Look at the region from hud_y to hud_y + some amount
    let card_area_height = (h as f64 * 0.15) as u32;
    let card_area_top = hud_y + 5;
    let card_area_bottom = (hud_y + card_area_height).min(h.saturating_sub(5));

    if card_area_top >= card_area_bottom || card_area_bottom - card_area_top < 10 {
        return 0.0;
    }

    // Compute per-column brightness in the card area
    let y_step = ((card_area_bottom - card_area_top) / 10).max(1);
    let col_bright: Vec<f64> = (0..w)
        .map(|x| {
            let mut sum = 0.0;
            let mut cnt = 0u32;
            let mut y = card_area_top;
            while y < card_area_bottom {
                let px = frame.get_pixel(x, y);
                sum += (px[0] as f64 + px[1] as f64 + px[2] as f64) / 3.0;
                cnt += 1;
                y += y_step;
            }
            sum / cnt.max(1) as f64
        })
        .collect();

    // Smooth the profile
    let smooth_window = (w as usize / 200).max(3);
    let smoothed = smooth(&col_bright, smooth_window);

    // Find bright segments
    let threshold = adaptive_threshold(&smoothed);
    let segments = find_bright_segments(&smoothed, threshold, (w as usize / 60).max(10));

    // Score based on how many segments we found and how evenly spaced they are
    if segments.len() < 3 || segments.len() > 7 {
        return 0.0;
    }

    // Check for roughly even spacing
    let widths: Vec<usize> = segments.iter().map(|(s, e)| e - s).collect();
    let avg_width = widths.iter().sum::<usize>() as f64 / widths.len() as f64;
    let width_variance: f64 = widths
        .iter()
        .map(|&w| {
            let diff = w as f64 - avg_width;
            diff * diff
        })
        .sum::<f64>()
        / widths.len() as f64;
    let width_cv = width_variance.sqrt() / avg_width.max(1.0);

    // Check for roughly even gaps between segments
    let mut gap_regularity = 1.0;
    if segments.len() >= 2 {
        let gaps: Vec<usize> = segments
            .windows(2)
            .map(|pair| pair[1].0.saturating_sub(pair[0].1))
            .collect();
        let avg_gap = gaps.iter().sum::<usize>() as f64 / gaps.len() as f64;
        if avg_gap > 0.0 {
            let gap_variance: f64 = gaps
                .iter()
                .map(|&g| {
                    let diff = g as f64 - avg_gap;
                    diff * diff
                })
                .sum::<f64>()
                / gaps.len() as f64;
            let gap_cv = gap_variance.sqrt() / avg_gap;
            gap_regularity = (1.0 - gap_cv).max(0.0);
        }
    }

    // Combine scores
    let count_score = if segments.len() == 5 {
        1.0
    } else if segments.len() == 4 || segments.len() == 6 {
        0.7
    } else {
        0.4
    };
    let evenness_score = (1.0 - width_cv).max(0.0);

    let score = count_score * 0.4 + evenness_score * 0.3 + gap_regularity * 0.3;

    debug!(
        "  Card pattern: {} segments, width_cv={:.2}, gap_reg={:.2}, score={:.2}",
        segments.len(),
        width_cv,
        gap_regularity,
        score
    );

    score
}

fn smooth(data: &[f64], window: usize) -> Vec<f64> {
    let half = window / 2;
    (0..data.len())
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half + 1).min(data.len());
            data[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
        })
        .collect()
}

fn adaptive_threshold(profile: &[f64]) -> f64 {
    let mut sorted: Vec<f64> = profile.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let dark_ref = sorted[sorted.len() * 40 / 100];
    let bright_ref = sorted[sorted.len() * 85 / 100];
    dark_ref + (bright_ref - dark_ref) * 0.35
}

fn find_bright_segments(
    profile: &[f64],
    threshold: f64,
    min_width: usize,
) -> Vec<(usize, usize)> {
    let mut segments = Vec::new();
    let mut in_bright = false;
    let mut start = 0;

    for (i, &val) in profile.iter().enumerate() {
        if !in_bright && val > threshold {
            start = i;
            in_bright = true;
        } else if in_bright && val <= threshold {
            if i - start >= min_width {
                segments.push((start, i));
            }
            in_bright = false;
        }
    }
    if in_bright && profile.len() - start >= min_width {
        segments.push((start, profile.len()));
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a synthetic TFT-like frame: bright game board, dark HUD bar, 5 bright card columns.
    fn make_tft_frame(width: u32, height: u32) -> RgbaImage {
        let mut frame = RgbaImage::new(width, height);

        let hud_y = (height as f64 * 0.78) as u32;
        let card_top = (height as f64 * 0.82) as u32;
        let card_bottom = (height as f64 * 0.96) as u32;

        // Game board (bright)
        for y in 0..hud_y {
            for x in 0..width {
                frame.put_pixel(x, y, image::Rgba([100, 120, 90, 255]));
            }
        }

        // HUD bar (dark)
        for y in hud_y..card_top {
            for x in 0..width {
                frame.put_pixel(x, y, image::Rgba([15, 15, 20, 255]));
            }
        }

        // 5 shop cards (bright columns with dark gaps)
        let card_area_width = (width as f64 * 0.43) as u32;
        let card_start_x = (width as f64 * 0.285) as u32;
        let card_width = card_area_width / 5;
        let gap = card_width / 10;

        for y in card_top..card_bottom {
            for x in 0..width {
                // Default dark background
                frame.put_pixel(x, y, image::Rgba([10, 10, 10, 255]));
            }
            for i in 0..5 {
                let cx = card_start_x + i * card_width + gap;
                let cw = card_width - gap * 2;
                for x in cx..(cx + cw).min(width) {
                    frame.put_pixel(x, y, image::Rgba([140, 130, 120, 255]));
                }
            }
        }

        // Bottom edge (dark)
        for y in card_bottom..height {
            for x in 0..width {
                frame.put_pixel(x, y, image::Rgba([5, 5, 5, 255]));
            }
        }

        frame
    }

    #[test]
    fn test_fullscreen_tft_returns_full_frame() {
        // A fullscreen TFT frame should return the full frame as game area
        let frame = make_tft_frame(1920, 1080);
        let result = detect_game_area(&frame);
        assert!(result.is_some(), "Should detect game area in fullscreen TFT");
        let area = result.unwrap();
        // Fullscreen fast-path: should return full frame
        assert!(
            area.region.width > 0.8 && area.region.height > 0.8,
            "Fullscreen TFT should return ~full frame, got w={:.2} h={:.2}",
            area.region.width,
            area.region.height
        );
    }

    #[test]
    fn test_windowed_tft_detected() {
        // TFT game in a window surrounded by desktop (gray)
        let mut frame = RgbaImage::new(2560, 1440);

        // Fill with desktop-like content (uniform gray)
        for y in 0..1440 {
            for x in 0..2560 {
                frame.put_pixel(x, y, image::Rgba([80, 80, 85, 255]));
            }
        }

        // Place a smaller TFT frame in the center
        let tft = make_tft_frame(1920, 1080);
        let offset_x = (2560 - 1920) / 2;
        let offset_y = (1440 - 1080) / 2;
        for y in 0..1080u32 {
            for x in 0..1920u32 {
                let px = tft.get_pixel(x, y);
                frame.put_pixel(x + offset_x, y + offset_y, *px);
            }
        }

        let result = detect_game_area(&frame);
        assert!(
            result.is_some(),
            "Should detect game area in windowed TFT"
        );
        let area = result.unwrap();
        // Should have detected a sub-region, not the full frame
        assert!(
            area.region.width < 0.95 || area.region.height < 0.95,
            "Windowed TFT should not return full frame"
        );
        assert!(area.confidence > 0.3, "Should have reasonable confidence");
    }

    #[test]
    fn test_non_tft_returns_none() {
        // Uniform gray frame — not TFT
        let mut frame = RgbaImage::new(1920, 1080);
        for y in 0..1080 {
            for x in 0..1920 {
                frame.put_pixel(x, y, image::Rgba([128, 128, 128, 255]));
            }
        }
        let result = detect_game_area(&frame);
        assert!(result.is_none(), "Uniform gray should not be detected as TFT");
    }

    #[test]
    fn test_small_frame_returns_none() {
        let frame = RgbaImage::new(50, 50);
        assert!(detect_game_area(&frame).is_none());
    }
}
