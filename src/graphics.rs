//! One shared pixel background underneath real terminal text.
//! Kitty's negative z-index keeps glyphs above the image. Surface geometry and
//! icon placement are resolved by the canvas, not reconstructed by this backend.
use std::io::{self, Cursor, Write};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::surface::Background;

const IMAGE_ID: u32 = 6_800;

pub fn supported() -> bool {
    let option = std::env::var("MOBUX_GRAPHICS").unwrap_or_default();
    enabled(
        &option,
        std::env::var("TERM_PROGRAM").as_deref().unwrap_or(""),
        std::env::var_os("KITTY_WINDOW_ID").is_some(),
        std::env::var_os("TMUX").is_some()
            || std::env::var_os("STY").is_some()
            || std::env::var("TERM")
                .is_ok_and(|term| term.starts_with("screen") || term.starts_with("tmux")),
    )
}

fn enabled(option: &str, program: &str, kitty: bool, multiplexed: bool) -> bool {
    match option {
        "kitty" => true,
        "cells" => false,
        _ => !multiplexed && (kitty || program == "ghostty"),
    }
}

#[derive(Default)]
pub struct Graphics {
    visible: Option<Background>,
}

impl Graphics {
    pub fn clear(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.visible.take().is_some() {
            write!(out, "\x1b_Ga=d,d=I,i={IMAGE_ID},q=2\x1b\\")?;
            out.flush()?;
        }
        Ok(())
    }

    pub fn draw(&mut self, out: &mut impl Write, background: Option<Background>) -> io::Result<()> {
        let Some(background) = background else {
            return self.clear(out);
        };
        if self.visible.as_ref() == Some(&background) {
            return Ok(());
        }
        let mut png = Cursor::new(Vec::new());
        background
            .pixels
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(io::Error::other)?;
        let data = STANDARD.encode(png.into_inner());
        let area = background.area;
        // Re-uploading the same ID replaces its old placement atomically after
        // the complete image arrives. Do not delete it first and cause a flash.
        self.visible = Some(background);
        write!(out, "\x1b7\x1b[{};{}H", area.y + 1, area.x + 1)?;
        write_upload(out, area.width, area.height, &data)?;
        write!(out, "\x1b8")?;
        out.flush()
    }
}

fn write_upload(out: &mut impl Write, cols: u16, rows: u16, data: &str) -> io::Result<()> {
    let chunks = data.as_bytes().chunks(4096);
    let count = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let more = u8::from(index + 1 < count);
        if index == 0 {
            write!(
                out,
                "\x1b_Ga=T,f=100,i={IMAGE_ID},p=1,c={cols},r={rows},C=1,z=-1,q=2,m={more};"
            )?;
        } else {
            write!(out, "\x1b_Gq=2,m={more};")?;
        }
        out.write_all(chunk)?;
        write!(out, "\x1b\\")?;
    }
    Ok(())
}

impl Drop for Graphics {
    fn drop(&mut self) {
        let _ = self.clear(&mut io::stdout());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn detection_is_conservative_and_can_be_overridden() {
        assert!(enabled("", "ghostty", false, false));
        assert!(enabled("", "", true, false));
        assert!(!enabled("", "", false, false));
        assert!(!enabled("", "ghostty", false, true));
        assert!(!enabled("cells", "ghostty", true, false));
        assert!(enabled("kitty", "", false, true));
    }

    #[test]
    fn chunks_are_bounded_quiet_and_under_text() {
        let mut out = Vec::new();
        write_upload(&mut out, 40, 48, &"A".repeat(5000)).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("p=1,c=40,r=48,C=1,z=-1,q=2,m=1;"));
        assert!(text.contains("\x1b_Gq=2,m=0;"));
        assert!(text.ends_with("\x1b\\"));
    }

    #[test]
    fn uploaded_png_is_the_exact_canvas_and_unchanged_frames_are_cached() {
        let frame = Background {
            area: Rect::new(0, 0, 2, 2),
            pixels: image::RgbaImage::from_pixel(16, 32, image::Rgba([23, 45, 67, 255])),
        };
        let mut graphics = Graphics::default();
        let mut out = Vec::new();
        graphics.draw(&mut out, Some(frame.clone())).unwrap();
        let wire = String::from_utf8(out).unwrap();
        let upload = wire.split_once("\x1b_G").unwrap().1;
        let payload = upload
            .split_once(';')
            .unwrap()
            .1
            .split("\x1b\\")
            .next()
            .unwrap();
        let decoded = STANDARD.decode(payload).unwrap();
        let image = image::load_from_memory(&decoded).unwrap().into_rgba8();
        assert_eq!(image, frame.pixels);
        let mut out = Vec::new();
        graphics.draw(&mut out, Some(frame)).unwrap();
        assert!(out.is_empty());
        graphics.draw(&mut out, None).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("a=d,d=I,i=6800"));
    }
}
