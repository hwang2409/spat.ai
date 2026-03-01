use crate::CaptureStatus;
use image::RgbaImage;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tracing::{info, warn};

/// Decode a video file and send RGBA frames through the watch channel,
/// mimicking the same interface as `capture_loop`.
pub async fn video_loop(
    path: &Path,
    frame_tx: watch::Sender<Option<Arc<RgbaImage>>>,
    status_tx: watch::Sender<CaptureStatus>,
    frame_interval: Duration,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let window_title = format!("[Video] {}", filename);

    info!("Video loop starting: {}", path.display());

    let path_owned = path.to_path_buf();
    let (decode_tx, mut decode_rx) = tokio::sync::mpsc::channel::<RgbaImage>(2);

    // Spawn blocking decode thread
    let stop_decode = stop.clone();
    let decode_handle = tokio::task::spawn_blocking(move || {
        decode_video(&path_owned, decode_tx, stop_decode)
    });

    let mut frame_count = 0u64;
    let mut fps_timer = Instant::now();

    // Receive decoded frames and forward to watch channel at the desired interval
    loop {
        if stop.load(Ordering::Relaxed) {
            info!("Video loop stopping (stop signal)");
            break;
        }

        let tick_start = Instant::now();

        match decode_rx.recv().await {
            Some(frame) => {
                let resolution = (frame.width(), frame.height());
                frame_count += 1;

                let elapsed = fps_timer.elapsed().as_secs_f64();
                let fps = if elapsed > 0.0 {
                    frame_count as f64 / elapsed
                } else {
                    0.0
                };

                if elapsed > 5.0 {
                    frame_count = 0;
                    fps_timer = Instant::now();
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let _ = status_tx.send(CaptureStatus {
                    is_capturing: true,
                    window_found: true,
                    window_title: Some(window_title.clone()),
                    fps,
                    last_capture_time: Some(now),
                    resolution: Some(resolution),
                });

                let _ = frame_tx.send(Some(Arc::new(frame)));

                // Pace to frame_interval
                let decode_elapsed = tick_start.elapsed();
                if decode_elapsed < frame_interval {
                    tokio::time::sleep(frame_interval - decode_elapsed).await;
                }
            }
            None => {
                // Channel closed — video finished
                info!("Video decode complete");
                break;
            }
        }
    }

    // Wait for decode thread to finish
    match decode_handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!("Video decode error: {}", e),
        Err(e) => warn!("Video decode thread panicked: {}", e),
    }

    let _ = status_tx.send(CaptureStatus::default());
    info!("Video loop stopped");
    Ok(())
}

/// Blocking video decode using ffmpeg-next.
/// Sends decoded RGBA frames through the mpsc channel.
fn decode_video(
    path: &Path,
    tx: tokio::sync::mpsc::Sender<RgbaImage>,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    ffmpeg_next::init()?;

    let mut ictx = ffmpeg_next::format::input(path)?;

    let video_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .ok_or_else(|| anyhow::anyhow!("No video stream found"))?;

    let stream_index = video_stream.index();
    let decoder_ctx = ffmpeg_next::codec::context::Context::from_parameters(video_stream.parameters())?;
    let mut decoder = decoder_ctx.decoder().video()?;

    let mut scaler = ffmpeg_next::software::scaling::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg_next::format::Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        ffmpeg_next::software::scaling::Flags::BILINEAR,
    )?;

    info!(
        "Video opened: {}x{}, format {:?}",
        decoder.width(),
        decoder.height(),
        decoder.format()
    );

    let width = decoder.width();
    let height = decoder.height();

    for (stream, packet) in ictx.packets() {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        if stream.index() != stream_index {
            continue;
        }

        decoder.send_packet(&packet)?;

        let mut decoded_frame = ffmpeg_next::frame::Video::empty();
        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }

            let mut rgba_frame = ffmpeg_next::frame::Video::empty();
            scaler.run(&decoded_frame, &mut rgba_frame)?;

            let data = rgba_frame.data(0);
            let stride = rgba_frame.stride(0);

            // Copy row-by-row in case stride != width*4
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            for y in 0..height as usize {
                let row_start = y * stride;
                let row_end = row_start + (width as usize * 4);
                pixels.extend_from_slice(&data[row_start..row_end]);
            }

            if let Some(img) = RgbaImage::from_raw(width, height, pixels) {
                if tx.blocking_send(img).is_err() {
                    // Receiver dropped
                    return Ok(());
                }
            }
        }
    }

    // Flush decoder
    decoder.send_eof()?;
    let mut decoded_frame = ffmpeg_next::frame::Video::empty();
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut rgba_frame = ffmpeg_next::frame::Video::empty();
        scaler.run(&decoded_frame, &mut rgba_frame)?;

        let data = rgba_frame.data(0);
        let stride = rgba_frame.stride(0);

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let row_start = y * stride;
            let row_end = row_start + (width as usize * 4);
            pixels.extend_from_slice(&data[row_start..row_end]);
        }

        if let Some(img) = RgbaImage::from_raw(width, height, pixels) {
            if tx.blocking_send(img).is_err() {
                return Ok(());
            }
        }
    }

    Ok(())
}
