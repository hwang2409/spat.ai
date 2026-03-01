use image::RgbaImage;
use tft_capture::ScreenRegion;
use tracing::debug;

/// Detected positions of TFT UI elements, found dynamically by analyzing the frame.
#[derive(Debug, Clone)]
pub struct DetectedLayout {
    pub shop_slots: Vec<ScreenRegion>,
    pub gold: Option<ScreenRegion>,
    pub level: Option<ScreenRegion>,
    pub stage: Option<ScreenRegion>,
    /// Y coordinate (normalized) of the HUD top boundary
    pub hud_top: f64,
}

/// Analyze a captured frame to dynamically find TFT UI element positions.
/// Adapts to different resolutions, window sizes, and title bar offsets.
pub fn detect_layout(frame: &RgbaImage) -> DetectedLayout {
    let h = frame.height();
    let hf = h as f64;

    // 1. Find the bottom HUD boundary (dark bar at bottom of screen)
    let hud_top_px = find_hud_top(frame);
    let hud_top = hud_top_px as f64 / hf;

    debug!(
        "HUD boundary at y={} ({:.1}%)",
        hud_top_px,
        hud_top * 100.0
    );

    // 2. Find shop card positions using column brightness analysis
    let shop_slots = find_shop_cards(frame, hud_top_px);
    debug!("Detected {} shop card(s)", shop_slots.len());

    // 3. Determine the search area for gold/level/stage
    // The HUD bar is the thin strip between the game board and the shop cards.
    // Expand the search area slightly to catch elements that sit at boundaries.
    let card_top_px = if !shop_slots.is_empty() {
        (shop_slots[0].y * hf) as u32
    } else {
        hud_top_px + ((h - hud_top_px) as f64 * 0.25) as u32
    };
    // Expand the bar search area by 10px each direction for robustness
    let bar_search_top = hud_top_px.saturating_sub(10);
    let bar_search_bottom = (card_top_px + 10).min(h);

    // 4. Find gold using coin color detection in the HUD bar area
    let gold = find_gold_region(frame, bar_search_top, bar_search_bottom);

    // 5. Find level text on far left of HUD bar area
    let level = find_level_region(frame, bar_search_top, bar_search_bottom);

    // 6. Find stage text at top center
    let stage = find_stage_region(frame);

    DetectedLayout {
        shop_slots,
        gold,
        level,
        stage,
        hud_top,
    }
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

/// Find where the bottom HUD starts.
///
/// Strategy: the HUD bar is always at approximately 78-88% of frame height.
/// We find the biggest brightness DROP in this range where the area below
/// is dark. This is robust against varying game states (empty shop, combat, etc.)
/// because the HUD bar is always present and darker than the game board above it.
fn find_hud_top(frame: &RgbaImage) -> u32 {
    let h = frame.height();
    let search_top = (h as f64 * 0.74) as u32;
    let search_bottom = (h as f64 * 0.90) as u32;

    // Use a 5-row window to smooth out single-row anomalies
    let window = 5u32;
    let mut best_drop = 0.0f64;
    let mut best_y = (h as f64 * 0.80) as u32;

    let end = search_bottom.saturating_sub(window * 2);
    if search_top >= end {
        return best_y;
    }

    for y in search_top..end {
        let above = avg_brightness_rows(frame, y, window);
        let below = avg_brightness_rows(frame, y + window, window);
        let drop = above - below;

        // Look for a brightness drop where the region below is genuinely dark (<55)
        if drop > best_drop && below < 55.0 && drop > 5.0 {
            best_drop = drop;
            best_y = y + window;
        }
    }

    debug!(
        "HUD boundary: brightness drop {:.1} at y={} ({:.1}%)",
        best_drop,
        best_y,
        best_y as f64 / h as f64 * 100.0
    );
    best_y
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

/// Find shop card positions by analyzing column brightness in the HUD area.
fn find_shop_cards(frame: &RgbaImage, hud_top: u32) -> Vec<ScreenRegion> {
    let (w, h) = (frame.width(), frame.height());
    let wf = w as f64;
    let hf = h as f64;
    let hud_h = h - hud_top;

    if hud_h < 20 {
        return Vec::new();
    }

    // Shop cards occupy the lower ~75% of the HUD
    let card_area_top = hud_top + hud_h / 4;
    let card_area_bottom = h.saturating_sub(8);
    if card_area_top >= card_area_bottom {
        return Vec::new();
    }

    // Compute per-column average brightness in the card area
    let y_step = ((card_area_bottom - card_area_top) / 15).max(1);
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

    // Smooth the profile (window ~ 0.5% of width)
    let window = (w as usize / 200).max(3);
    let smoothed = smooth(&col_bright, window);

    // Find threshold to separate cards (bright) from gaps (dark)
    let threshold = adaptive_threshold(&smoothed);
    let segments = find_bright_segments(&smoothed, threshold, 30);

    debug!(
        "Card detection: threshold={:.1}, {} bright segment(s)",
        threshold,
        segments.len()
    );

    // Determine card y range more precisely
    let card_y_start = find_card_y_start(frame, hud_top, &segments);
    let card_y_start_norm = card_y_start as f64 / hf;
    let card_height_norm = card_area_bottom as f64 / hf - card_y_start_norm;

    // Normalize to exactly 5 cards
    let cards = normalize_to_five(&segments, w as usize);
    cards
        .iter()
        .map(|&(start, end)| ScreenRegion {
            x: start as f64 / wf,
            y: card_y_start_norm,
            width: (end - start) as f64 / wf,
            height: card_height_norm,
        })
        .collect()
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

/// Adaptive threshold using percentile-based approach.
fn adaptive_threshold(profile: &[f64]) -> f64 {
    let mut sorted: Vec<f64> = profile.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let dark_ref = sorted[sorted.len() * 40 / 100];
    let bright_ref = sorted[sorted.len() * 85 / 100];
    dark_ref + (bright_ref - dark_ref) * 0.35
}

/// Find contiguous bright segments above the threshold.
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

/// Given detected bright segments, normalize to exactly 5 shop cards.
fn normalize_to_five(segments: &[(usize, usize)], _frame_width: usize) -> Vec<(usize, usize)> {
    if segments.is_empty() {
        return Vec::new();
    }

    if segments.len() == 5 {
        return segments.to_vec();
    }

    // Use the full span and divide evenly into 5 cards
    let total_start = segments[0].0;
    let total_end = segments.last().unwrap().1;
    let total_width = total_end - total_start;
    let card_width = total_width / 5;
    let gap = (card_width as f64 * 0.03) as usize;

    (0..5)
        .map(|i| {
            let s = total_start + i * card_width + gap;
            let e = total_start + (i + 1) * card_width - gap;
            (s, e)
        })
        .collect()
}

/// Find the y coordinate where card content actually starts.
fn find_card_y_start(frame: &RgbaImage, hud_top: u32, cards: &[(usize, usize)]) -> u32 {
    if cards.is_empty() {
        return hud_top;
    }

    let h = frame.height();
    let mid_x = ((cards[0].0 + cards[0].1) / 2) as u32;

    for y in hud_top..h {
        let px = frame.get_pixel(mid_x, y);
        let b = (px[0] as f64 + px[1] as f64 + px[2] as f64) / 3.0;
        if b > 45.0 {
            return y;
        }
    }

    hud_top
}

/// Find the gold region by detecting the gold coin icon (yellow pixels) in the HUD bar.
/// Also looks for the gold text color (white/light text) near the coin.
fn find_gold_region(frame: &RgbaImage, bar_top: u32, bar_bottom: u32) -> Option<ScreenRegion> {
    let (w, h) = (frame.width(), frame.height());
    let wf = w as f64;
    let hf = h as f64;

    if bar_top >= bar_bottom || bar_bottom.saturating_sub(bar_top) < 3 {
        return None;
    }

    // Scan the HUD bar for yellow/gold pixels (coin icon)
    // Relaxed thresholds to catch various shades of gold
    let mut gold_xs: Vec<u32> = Vec::new();
    let mut gold_ys: Vec<u32> = Vec::new();

    for y in bar_top..bar_bottom {
        for x in (w / 5)..(w * 4 / 5) {
            let px = frame.get_pixel(x, y);
            let (r, g, b) = (px[0], px[1], px[2]);
            // Gold/yellow: R high, G moderate-high, B low
            if r > 160 && g > 120 && b < 120 && (r as i32 - b as i32) > 60 {
                gold_xs.push(x);
                gold_ys.push(y);
            }
        }
    }

    if gold_xs.is_empty() {
        debug!(
            "No gold coin pixels found in HUD bar (y={}..{})",
            bar_top, bar_bottom
        );
        return None;
    }

    gold_xs.sort();
    gold_ys.sort();
    let coin_x = gold_xs[gold_xs.len() / 2];
    let coin_y_min = gold_ys[0];
    let coin_y_max = gold_ys[gold_ys.len() - 1];

    // The gold number text is to the right of the coin
    let text_x = coin_x + 12;
    let text_width = 50.0;
    let region_y = coin_y_min.saturating_sub(3);
    let region_h = (coin_y_max - coin_y_min + 8).max(15);

    debug!(
        "Gold coin at x={}, y={}..{}, text region x={}..{}",
        coin_x,
        coin_y_min,
        coin_y_max,
        text_x,
        text_x as f64 + text_width
    );

    Some(ScreenRegion {
        x: text_x as f64 / wf,
        y: region_y as f64 / hf,
        width: text_width / wf,
        height: region_h as f64 / hf,
    })
}

/// Find the level region.
/// In TFT the level is shown as "Lv. X" on the far left, at the same height as shop cards.
/// We look to the LEFT of the first shop card for the level number.
fn find_level_region(
    frame: &RgbaImage,
    bar_top: u32,
    bar_bottom: u32,
) -> Option<ScreenRegion> {
    let (w, h) = (frame.width(), frame.height());
    let wf = w as f64;
    let hf = h as f64;

    if bar_top >= bar_bottom {
        return None;
    }

    // The level indicator sits in the HUD area between game board and shop cards.
    // It's typically around x=270-330, in the bar area.
    // Look for bright or yellow-ish text pixels in a focused area on the left side.
    let search_x_start = (w as f64 * 0.10) as u32;
    let search_x_end = (w as f64 * 0.22) as u32;

    // Collect bright pixels that could be text (white, yellow, or light colored)
    let mut text_xs: Vec<u32> = Vec::new();
    let mut text_ys: Vec<u32> = Vec::new();

    for y in bar_top..bar_bottom {
        for x in search_x_start..search_x_end {
            let px = frame.get_pixel(x, y);
            let (r, g, b) = (px[0] as u32, px[1] as u32, px[2] as u32);
            let brightness = (r + g + b) / 3;
            // Accept white text or yellow/gold text
            if brightness > 150 || (r > 180 && g > 150 && b < 120) {
                text_xs.push(x);
                text_ys.push(y);
            }
        }
    }

    if text_xs.len() < 5 {
        debug!(
            "No level text found (only {} bright pixels in x={}..{}, y={}..{})",
            text_xs.len(),
            search_x_start,
            search_x_end,
            bar_top,
            bar_bottom
        );
        return None;
    }

    text_xs.sort();
    text_ys.sort();
    let x_min = text_xs[text_xs.len() * 5 / 100]; // trim outliers
    let x_max = text_xs[text_xs.len() * 95 / 100];
    let y_min = text_ys[text_ys.len() * 5 / 100];
    let y_max = text_ys[text_ys.len() * 95 / 100];

    let pad = 4u32;
    let region_x = x_min.saturating_sub(pad);
    let region_y = y_min.saturating_sub(pad);
    let region_w = (x_max - x_min + pad * 2).max(20);
    let region_h = (y_max - y_min + pad * 2).max(15);

    debug!(
        "Level text: x={}..{}, y={}..{} ({} bright pixels)",
        x_min,
        x_max,
        y_min,
        y_max,
        text_xs.len()
    );

    Some(ScreenRegion {
        x: region_x as f64 / wf,
        y: region_y as f64 / hf,
        width: region_w as f64 / wf,
        height: region_h as f64 / hf,
    })
}

/// Find the stage indicator at the top center by looking for bright text.
fn find_stage_region(frame: &RgbaImage) -> Option<ScreenRegion> {
    let (w, h) = (frame.width(), frame.height());
    let wf = w as f64;
    let hf = h as f64;

    // First, find where game content starts (skip any titlebar)
    let game_top = find_game_top(frame);

    // Scan a narrow band at the top center for bright text
    // Use wider search area and lower brightness threshold
    let scan_bottom = (game_top + (h as f64 * 0.06) as u32).min(h);
    let center_start = w * 2 / 5;
    let center_end = w * 3 / 5;

    let mut min_x = w;
    let mut max_x = 0u32;
    let mut min_y = h;
    let mut max_y = 0u32;
    let mut found = false;

    for y in game_top..scan_bottom {
        for x in center_start..center_end {
            let px = frame.get_pixel(x, y);
            let brightness = (px[0] as u32 + px[1] as u32 + px[2] as u32) / 3;
            // Lower threshold to catch stage text which may not be pure white
            if brightness > 170 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    if !found || max_x - min_x < 5 {
        debug!("No stage text found at top center");
        return None;
    }

    // The stage text "X-Y" is typically compact. If the detected area is too wide,
    // it might include other UI elements. Constrain to center portion.
    let detected_width = max_x - min_x;
    let (final_min_x, final_max_x) = if detected_width > 200 {
        // Too wide, likely catching other elements. Take a narrow center strip.
        let center = (min_x + max_x) / 2;
        (center.saturating_sub(40), center + 40)
    } else {
        (min_x, max_x)
    };

    let pad = 5u32;
    debug!(
        "Stage text area: x={}..{}, y={}..{}",
        final_min_x, final_max_x, min_y, max_y
    );

    Some(ScreenRegion {
        x: final_min_x.saturating_sub(pad) as f64 / wf,
        y: min_y.saturating_sub(pad) as f64 / hf,
        width: (final_max_x - final_min_x + pad * 2) as f64 / wf,
        height: (max_y - min_y + pad * 2) as f64 / hf,
    })
}

/// Detect the top of the game area (skipping any window titlebar).
fn find_game_top(frame: &RgbaImage) -> u32 {
    let (w, h) = (frame.width(), frame.height());
    let step = (w / 50).max(1);
    let max_scan = (h / 10).min(60);

    for y in 0..max_scan {
        let mut sum = 0.0;
        let mut count = 0u32;
        let mut x = w / 4;
        while x < w * 3 / 4 {
            let px = frame.get_pixel(x, y);
            sum += (px[0] as f64 + px[1] as f64 + px[2] as f64) / 3.0;
            count += 1;
            x += step;
        }
        let avg = sum / count.max(1) as f64;
        if avg > 30.0 {
            return y;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_hud_top_with_cards() {
        // Simulate: bright game board, dark HUD bar, bright cards, dark edge
        let mut frame = RgbaImage::new(200, 100);
        // Game board (bright): y=0..70
        for y in 0..70 {
            for x in 0..200 {
                frame.put_pixel(x, y, image::Rgba([150, 150, 150, 255]));
            }
        }
        // HUD bar (dark): y=70..78
        for y in 70..78 {
            for x in 0..200 {
                frame.put_pixel(x, y, image::Rgba([15, 15, 15, 255]));
            }
        }
        // Shop cards (bright): y=78..95
        for y in 78..95 {
            for x in 0..200 {
                frame.put_pixel(x, y, image::Rgba([120, 120, 120, 255]));
            }
        }
        // Bottom edge (dark): y=95..100
        for y in 95..100 {
            for x in 0..200 {
                frame.put_pixel(x, y, image::Rgba([10, 10, 10, 255]));
            }
        }

        let hud_top = find_hud_top(&frame);
        assert!(
            hud_top >= 69 && hud_top <= 72,
            "HUD top should be ~70, got {}",
            hud_top
        );
    }

    #[test]
    fn test_smooth() {
        let data = vec![0.0, 0.0, 100.0, 100.0, 0.0, 0.0];
        let result = smooth(&data, 3);
        assert!(result[2] > 30.0 && result[2] < 100.0);
    }

    #[test]
    fn test_find_bright_segments() {
        let profile = vec![
            10.0, 10.0, 10.0, 80.0, 80.0, 80.0, 80.0, 10.0, 10.0, 80.0, 80.0, 80.0, 10.0,
        ];
        let segments = find_bright_segments(&profile, 50.0, 3);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], (3, 7));
        assert_eq!(segments[1], (9, 12));
    }
}
