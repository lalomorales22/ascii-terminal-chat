use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::thread;

use ffmpeg_next as ff;
use ff::util::frame::video::Video;

use crate::ascii::AsciiFrame;

pub struct WebcamCapture {
    frame_rx: Receiver<AsciiFrame>,
    shutdown_tx: Option<Sender<()>>,
}

impl WebcamCapture {
    pub fn new(width: u32, height: u32, fps: u32) -> Result<Self> {
        let (frame_tx, frame_rx) = bounded(10);
        let (shutdown_tx, shutdown_rx) = bounded(1);

        // Initialize FFmpeg
        ff::init().context("Failed to initialize FFmpeg")?;

        // For now, always use test pattern
        // Real webcam capture with FFmpeg requires complex platform-specific setup
        thread::spawn(move || {
            eprintln!("Note: Using test pattern for video. Real webcam support coming soon.");
            generate_test_pattern(frame_tx, shutdown_rx, width as u16, height as u16, fps);
        });

        Ok(Self {
            frame_rx,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub fn get_frame(&self) -> Option<AsciiFrame> {
        self.frame_rx.try_recv().ok()
    }

    #[allow(dead_code)]
    pub fn recv_frame(&self) -> Result<AsciiFrame> {
        self.frame_rx.recv().map_err(|e| anyhow::anyhow!("Failed to receive frame: {}", e))
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

// Simple test pattern as placeholder for real video
fn generate_test_pattern(
    frame_tx: Sender<AsciiFrame>,
    shutdown_rx: Receiver<()>,
    width: u16,
    height: u16,
    fps: u32,
) {
    let frame_delay = std::time::Duration::from_millis(1000 / fps as u64);
    let mut last_frame = std::time::Instant::now();
    let mut frame_count = 0u32;

    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        if last_frame.elapsed() < frame_delay {
            thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }

        // Create a more video-like test pattern with movement
        let mut rgb_data = Vec::new();
        let t = (frame_count as f32) * 0.1;
        
        for y in 0..height {
            for x in 0..width {
                // Create a moving gradient that looks more like a face/video
                let cx = width as f32 / 2.0;
                let cy = height as f32 / 2.0;
                let dx = (x as f32 - cx) / cx;
                let dy = (y as f32 - cy) / cy;
                let dist = (dx * dx + dy * dy).sqrt();
                
                // Create a circular gradient with some variation
                let intensity = ((1.0 - dist.min(1.0)) * 200.0) as u8;
                let variation = ((t.sin() * 20.0) as i16).abs() as u8;
                
                // Add some color variation to make it look more natural
                let r = intensity.saturating_add(variation);
                let g = intensity;
                let b = intensity.saturating_sub(variation);
                
                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        if let Ok(frame) = AsciiFrame::from_rgb_data(&rgb_data, width, height, false) {
            if frame_tx.send(frame).is_err() {
                break;
            }
        }

        frame_count = frame_count.wrapping_add(1);
        last_frame = std::time::Instant::now();
    }
}

// Helper function to convert FFmpeg Video frame to ASCII (for future use)
#[allow(dead_code)]
fn video_to_ascii_frame(rgb: &Video) -> Result<AsciiFrame> {
    let width = rgb.width() as u16;
    let height = rgb.height() as u16;
    let stride = rgb.stride(0);
    let data = rgb.data(0);

    let mut rgb_data = Vec::new();
    for y in 0..height {
        let row_start = (y as usize * stride);
        let row_end = row_start + (width as usize * 3);
        if row_end <= data.len() {
            let row = &data[row_start..row_end];
            
            for x in 0..width {
                let i = x as usize * 3;
                if i + 2 < row.len() {
                    rgb_data.push(row[i]);
                    rgb_data.push(row[i + 1]);
                    rgb_data.push(row[i + 2]);
                } else {
                    rgb_data.extend_from_slice(&[0, 0, 0]);
                }
            }
        } else {
            // Fill with black if we're past the data
            for _ in 0..width {
                rgb_data.extend_from_slice(&[0, 0, 0]);
            }
        }
    }

    AsciiFrame::from_rgb_data(&rgb_data, width, height, false)
}