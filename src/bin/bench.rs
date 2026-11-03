//! Reproducible CPU-side frame benchmark. Does not measure terminal display or SSH.
use std::io::Cursor;
use std::time::Instant;

use mobux::graphics::Graphics;
use mobux::paint;
use mobux::state::State;
use mobux::surface::CellSize;
use mobux::theme;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for (cols, rows) in [(40, 48), (100, 40)] {
        for cells in [
            CellSize {
                width: 8,
                height: 16,
            },
            CellSize {
                width: 16,
                height: 32,
            },
        ] {
            let mut terminal = Terminal::new(TestBackend::new(cols, rows))?;
            let mut state = State::new();
            state.image_icons = true;
            state.clock_override = Some(("12:00".to_string(), "Fri, Sep 22".to_string()));
            state.metrics =
                theme::metrics(state.type_size, state.app_size, Rect::new(0, 0, cols, rows));
            state.places =
                paint::layout(Rect::new(0, 0, cols, rows), &state.theme(), state.metrics);
            let mut scene_times = Vec::new();
            let mut png_times = Vec::new();
            let mut last = None;
            for index in 0..101 {
                let mut background = None;
                let mut elapsed = 0.0;
                terminal.draw(|frame| {
                    let start = Instant::now();
                    background = paint::scene(frame, &state, cells);
                    elapsed = start.elapsed().as_secs_f64() * 1000.0;
                })?;
                let frame = background.ok_or("native canvas unavailable")?;
                let start = Instant::now();
                frame
                    .pixels
                    .write_to(&mut Cursor::new(Vec::new()), image::ImageFormat::Png)?;
                let png = start.elapsed().as_secs_f64() * 1000.0;
                if index > 0 {
                    scene_times.push(elapsed);
                    png_times.push(png);
                }
                last = Some(frame);
            }
            let frame = last.ok_or("no rendered frames")?;
            let mut graphics = Graphics::default();
            let mut out = Vec::new();
            graphics.draw(&mut out, Some(frame.clone()))?;
            out.clear();
            graphics.draw(&mut out, Some(frame))?;
            let unchanged = out.len();
            graphics.clear(&mut out)?;
            println!("layout={} {cols}x{rows} cells={}x{} paint_median_ms={:.3} png_median_ms={:.3} unchanged_upload_bytes={unchanged}", state.skin.label(),cells.width,cells.height,median(&mut scene_times),median(&mut png_times));
        }
    }
    Ok(())
}
