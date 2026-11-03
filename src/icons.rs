//! Bundled alpha masks on adaptive badges. See assets/THIRD_PARTY.md.
//! No SVG parser, special font, downloads, or per-frame raster generation.
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use image::RgbaImage;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    Git,
    OpenCode,
    Folder,
    Bag,
    Settings,
    Terminal,
}

impl Glyph {
    pub fn from_catalog(name: &str) -> Option<Self> {
        Some(match name {
            "git" => Self::Git,
            "opencode" => Self::OpenCode,
            "folder" => Self::Folder,
            "apps" => Self::Bag,
            "settings" => Self::Settings,
            "terminal" | "editor" | "monitor" => Self::Terminal,
            _ => return None,
        })
    }

    fn mask(self) -> &'static RgbaImage {
        static MASKS: LazyLock<[RgbaImage; 6]> = LazyLock::new(|| {
            [
                include_bytes!("../assets/icons/account_tree.png").as_slice(),
                include_bytes!("../assets/icons/opencode.png").as_slice(),
                include_bytes!("../assets/icons/folder.png").as_slice(),
                include_bytes!("../assets/icons/apps.png").as_slice(),
                include_bytes!("../assets/icons/settings.png").as_slice(),
                include_bytes!("../assets/icons/terminal.png").as_slice(),
            ]
            .map(|png| {
                image::load_from_memory(png)
                    .expect("bundled icon must be a valid PNG")
                    .into_rgba8()
            })
        });
        &MASKS[self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconShape {
    Circle,
    Rounded,
    Square,
}

type AssetKey = (Glyph, (u8, u8, u8), IconShape);
static ASSETS: LazyLock<Mutex<HashMap<AssetKey, Arc<RgbaImage>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn raster(glyph: Glyph, accent: (u8, u8, u8), shape: IconShape) -> Arc<RgbaImage> {
    let mut assets = ASSETS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Arc::clone(
        assets
            .entry((glyph, accent, shape))
            .or_insert_with(|| Arc::new(draw_raster(glyph, accent, shape))),
    )
}

fn draw_raster(glyph: Glyph, accent: (u8, u8, u8), shape: IconShape) -> RgbaImage {
    let mask = glyph.mask();
    RgbaImage::from_fn(96, 96, |px, py| {
        let mut covered = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                let x = (px as f32 + (sx as f32 + 0.5) / 4.0) / 96.0;
                let y = (py as f32 + (sy as f32 + 0.5) / 4.0) / 96.0;
                covered += u32::from(covers(x, y, shape));
            }
        }
        let alpha = u32::from(mask.get_pixel(px, py)[3]);
        let color = [accent.0, accent.1, accent.2];
        let badge = [244u32, 239, 250];
        image::Rgba([
            ((u32::from(color[0]) * alpha + badge[0] * (255 - alpha)) / 255) as u8,
            ((u32::from(color[1]) * alpha + badge[1] * (255 - alpha)) / 255) as u8,
            ((u32::from(color[2]) * alpha + badge[2] * (255 - alpha)) / 255) as u8,
            (covered * 255 / 16) as u8,
        ])
    })
}

fn covers(x: f32, y: f32, shape: IconShape) -> bool {
    match shape {
        IconShape::Square => true,
        IconShape::Circle => (x - 0.5).powi(2) + (y - 0.5).powi(2) <= 0.25,
        IconShape::Rounded => {
            let dx = (x - 0.5).abs() - 0.28;
            let dy = (y - 0.5).abs() - 0.28;
            dx.max(0.0).powi(2) + dy.max(0.0).powi(2) <= 0.22f32.powi(2)
        }
    }
}

pub fn paint(
    buf: &mut Buffer,
    rect: Rect,
    glyph: Glyph,
    accent: (u8, u8, u8),
    shape: IconShape,
    backdrop: impl Fn(u16) -> Color,
) {
    let asset = raster(glyph, accent, shape);
    for row in 0..rect.height {
        let y = rect.y + row;
        for col in 0..rect.width {
            let sample = |half: u16| {
                let paper = match backdrop(y) {
                    Color::Rgb(r, g, b) => [r, g, b],
                    _ => [0; 3],
                };
                let mut sum = [0u32; 3];
                for sy in 0..6 {
                    for sx in 0..6 {
                        let x = ((f32::from(col) + (sx as f32 + 0.5) / 6.0) * 96.0
                            / f32::from(rect.width)) as u32;
                        let y = ((f32::from(row * 2 + half) + (sy as f32 + 0.5) / 6.0) * 96.0
                            / f32::from(rect.height * 2)) as u32;
                        let pixel = asset.get_pixel(x.min(95), y.min(95));
                        let a = u32::from(pixel[3]);
                        for i in 0..3 {
                            sum[i] +=
                                (u32::from(pixel[i]) * a + u32::from(paper[i]) * (255 - a)) / 255;
                        }
                    }
                }
                Color::Rgb(
                    (sum[0] / 36) as u8,
                    (sum[1] / 36) as u8,
                    (sum[2] / 36) as u8,
                )
            };
            if let Some(cell) = buf.cell_mut(Position::new(rect.x + col, y)) {
                cell.set_symbol("▀")
                    .set_style(Style::default().fg(sample(0)).bg(sample(1)));
            }
        }
    }
}

pub fn body(rect: Rect) -> Rect {
    let width = rect.width.saturating_sub(2).max(1);
    let height = width.div_ceil(2).min(rect.height.saturating_sub(1));
    let width = width.min(height.saturating_mul(2));
    Rect::new(
        rect.x + rect.width.saturating_sub(width) / 2,
        rect.y,
        width,
        height,
    )
}

pub fn visible_body(slot: Rect, nav_y: u16, right: u16) -> Option<Rect> {
    if slot.width < 3 || slot.height < 3 || slot.bottom() > nav_y || slot.right() > right {
        return None;
    }
    let rect = body(slot);
    (rect.width > 0 && rect.height > 0).then_some(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_assets_are_reused_and_shapes_are_distinct() {
        let first = raster(Glyph::OpenCode, (124, 92, 255), IconShape::Circle);
        let second = raster(Glyph::OpenCode, (124, 92, 255), IconShape::Circle);
        assert!(Arc::ptr_eq(&first, &second));
        let square = raster(Glyph::OpenCode, (124, 92, 255), IconShape::Square);
        assert_ne!(first, square);
        assert_eq!(first.get_pixel(0, 0)[3], 0);
        assert_eq!(square.get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn all_bundled_masks_are_centered_transparent_and_have_visible_glyphs() {
        for glyph in [
            Glyph::Git,
            Glyph::OpenCode,
            Glyph::Folder,
            Glyph::Bag,
            Glyph::Settings,
            Glyph::Terminal,
        ] {
            let mask = glyph.mask();
            assert_eq!(mask.dimensions(), (96, 96));
            assert_eq!(mask.get_pixel(0, 0)[3], 0);
            assert!(mask.pixels().any(|p| p[3] > 0));
        }
        assert_ne!(Glyph::OpenCode.mask(), Glyph::Terminal.mask());
    }

    #[test]
    fn icons_must_fit_above_navigation_and_have_room_for_labels() {
        assert!(visible_body(Rect::new(0, 0, 2, 5), 10, 40).is_none());
        assert!(visible_body(Rect::new(0, 8, 8, 5), 10, 40).is_none());
        assert!(visible_body(Rect::new(38, 0, 8, 5), 10, 40).is_none());
        assert_eq!(
            visible_body(Rect::new(0, 0, 8, 5), 10, 40),
            Some(Rect::new(1, 0, 6, 3))
        );
    }
}
