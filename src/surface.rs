use std::ops::{Deref, DerefMut};

use image::RgbaImage;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Transform};

use crate::theme::SurfaceStyle;

const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const SAMPLES: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellSize {
    pub width: u16,
    pub height: u16,
}

impl Default for CellSize {
    fn default() -> Self {
        Self {
            width: 8,
            height: 16,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Background {
    pub area: Rect,
    pub pixels: RgbaImage,
}

pub struct Canvas<'a> {
    buffer: &'a mut Buffer,
    pixels: Option<Pixmap>,
    cells: CellSize,
    style: SurfaceStyle,
    transparent: bool,
}

impl<'a> Canvas<'a> {
    pub fn new(buffer: &'a mut Buffer, cells: CellSize, native: bool, style: SurfaceStyle) -> Self {
        let cells = CellSize {
            width: cells.width.clamp(1, 64),
            height: cells.height.clamp(1, 128),
        };
        let width = u32::from(buffer.area.width) * u32::from(cells.width);
        let height = u32::from(buffer.area.height) * u32::from(cells.height);
        let pixels = if native && u64::from(width) * u64::from(height) <= MAX_PIXELS {
            Pixmap::new(width, height).map(|mut pixels| {
                pixels.fill(tiny_skia::Color::BLACK);
                pixels
            })
        } else {
            None
        };
        Self {
            buffer,
            pixels,
            cells,
            style,
            transparent: false,
        }
    }

    pub fn clear_background(&mut self) {
        self.transparent = true;
        if let Some(pixels) = self.pixels.as_mut() {
            pixels.fill(tiny_skia::Color::TRANSPARENT);
        }
        for cell in &mut self.buffer.content {
            cell.reset();
        }
    }

    pub fn native(&self) -> bool {
        self.pixels.is_some()
    }

    pub fn finish(self) -> Option<Background> {
        let pixels = self.pixels?;
        let (width, height) = (pixels.width(), pixels.height());
        let mut data = pixels.take();
        // PNG takes straight alpha; tiny-skia stores premultiplied channels.
        // Only translucent pixels need conversion (wallpaper-off mode).
        if self.transparent {
            for pixel in data.as_chunks_mut::<4>().0 {
                let alpha = u32::from(pixel[3]);
                if alpha > 0 && alpha < 255 {
                    for channel in &mut pixel[..3] {
                        *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                    }
                }
            }
        }
        let pixels = RgbaImage::from_raw(width, height, data)?;
        for cell in &mut self.buffer.content {
            cell.set_bg(Color::Reset);
            if background_symbol(cell.symbol()) {
                cell.set_symbol(" ");
            }
        }
        Some(Background {
            area: self.buffer.area,
            pixels,
        })
    }

    /// Paint complete integer pixel rows. Per-cell quadrant rectangles leave
    /// seams on terminals with odd pixel-sized cells and are much more costly.
    pub fn background_row(&mut self, y: u16, upper: Color, lower: Color) {
        if y < self.buffer.area.y || y >= self.buffer.area.bottom() {
            return;
        }
        if let Some(pixels) = self.pixels.as_mut() {
            let width = pixels.width() as usize;
            let start = usize::from(y - self.buffer.area.y) * usize::from(self.cells.height);
            for row in 0..usize::from(self.cells.height) {
                let [r, g, b] = rgb(if row < usize::from(self.cells.height) / 2 {
                    upper
                } else {
                    lower
                });
                let offset = (start + row) * width * 4;
                for pixel in pixels.data_mut()[offset..offset + width * 4]
                    .as_chunks_mut::<4>()
                    .0
                {
                    pixel.copy_from_slice(&[r, g, b, 255]);
                }
            }
        }
        for x in self.buffer.area.x..self.buffer.area.right() {
            if let Some(cell) = self.buffer.cell_mut(Position::new(x, y)) {
                cell.set_symbol("▀").set_fg(upper).set_bg(lower);
            }
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color) {
        let rect = rect.intersection(self.buffer.area);
        let bounds = self.pixel_rect(rect);
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };
        if let Some(rect) = tiny_skia::Rect::from_xywh(bounds.x, bounds.y, bounds.w, bounds.h) {
            pixels.fill_rect(rect, &paint(color), Transform::identity(), None);
        }
    }

    pub fn block(&mut self, x: u16, y: u16, symbol: &str, style: &Style) {
        if !self.buffer.area.contains(Position::new(x, y)) || !background_symbol(symbol) {
            return;
        }
        let bounds = self.pixel_rect(Rect::new(x, y, 1, 1));
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };
        let fg = style.fg.unwrap_or(Color::Reset);
        let bg = style.bg.unwrap_or(Color::Reset);
        let quadrants = foreground_quadrants(symbol);
        for (index, foreground) in quadrants.into_iter().enumerate() {
            let left = bounds.x + (index % 2) as f32 * bounds.w / 2.0;
            let top = bounds.y + (index / 2) as f32 * bounds.h / 2.0;
            if let Some(rect) =
                tiny_skia::Rect::from_xywh(left, top, bounds.w / 2.0, bounds.h / 2.0)
            {
                let mut ink = paint(if foreground { fg } else { bg });
                ink.anti_alias = false;
                pixels.fill_rect(rect, &ink, Transform::identity(), None);
            }
        }
    }

    pub fn rounded(&mut self, rect: Rect, color: Color, radius: f32) {
        let clipped = rect.intersection(self.buffer.area);
        if clipped.is_empty() {
            return;
        }
        let mut shape = self.pixel_rect(rect);
        shape.radius = match self.style {
            SurfaceStyle::Blocky => 0.0,
            SurfaceStyle::Rounded => {
                let requested = if radius.is_nan() {
                    0.0
                } else {
                    radius.max(0.0)
                };
                (requested * f32::from(self.cells.width))
                    .min(shape.w / 2.0)
                    .min(shape.h / 2.0)
            }
        };
        if let (Some(pixels), Some(path)) = (self.pixels.as_mut(), shape.path()) {
            pixels.fill_path(
                &path,
                &paint(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        if self.native() {
            // The native background already contains the exact curve. Do not
            // also supersample a discarded text-cell version of every surface.
            for y in clipped.y..clipped.bottom() {
                for x in clipped.x..clipped.right() {
                    if let Some(cell) = self.buffer.cell_mut(Position::new(x, y)) {
                        cell.reset();
                        cell.set_fg(color).set_bg(color);
                    }
                }
            }
            return;
        }
        for y in clipped.y..clipped.bottom() {
            for x in clipped.x..clipped.right() {
                let left = f32::from(x - rect.x) * f32::from(self.cells.width);
                let top = f32::from(y - rect.y) * f32::from(self.cells.height);
                let coverage = [
                    shape.coverage(left, top, self.cells, 0),
                    shape.coverage(left, top, self.cells, 1),
                ];
                if coverage == [0, 0] {
                    continue;
                }
                if let Some(cell) = self.buffer.cell_mut(Position::new(x, y)) {
                    let underlying = half_colors(cell);
                    let upper = blend(color, underlying[0], coverage[0], SAMPLES * SAMPLES);
                    let lower = blend(color, underlying[1], coverage[1], SAMPLES * SAMPLES);
                    cell.reset();
                    cell.set_symbol("▀").set_fg(upper).set_bg(lower);
                }
            }
        }
    }

    /// A quiet, one-pixel edge, not a character-cell border.
    pub fn panel(&mut self, rect: Rect, color: Color, edge: Color, radius: f32) {
        self.rounded(rect, color, radius);
        let mut shape = self.pixel_rect(rect);
        shape.radius = if self.style == SurfaceStyle::Blocky {
            0.0
        } else {
            (radius * f32::from(self.cells.width))
                .min(shape.w / 2.0)
                .min(shape.h / 2.0)
        };
        shape.x += 0.5;
        shape.y += 0.5;
        shape.w = (shape.w - 1.0).max(0.0);
        shape.h = (shape.h - 1.0).max(0.0);
        if let (Some(pixels), Some(path)) = (self.pixels.as_mut(), shape.path()) {
            pixels.stroke_path(
                &path,
                &paint(edge),
                &tiny_skia::Stroke {
                    width: 1.0,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
    }

    pub fn icon(&mut self, rect: Rect, rgba: &RgbaImage) {
        if !self.native()
            || rect.intersection(self.buffer.area).is_empty()
            || rgba.width() == 0
            || rgba.height() == 0
            || u64::from(rgba.width()) * u64::from(rgba.height()) > MAX_PIXELS
        {
            return;
        }
        let bounds = self.pixel_rect(rect);
        let scale = (bounds.w / rgba.width() as f32).min(bounds.h / rgba.height() as f32);
        let x = bounds.x + (bounds.w - rgba.width() as f32 * scale) / 2.0;
        let y = bounds.y + (bounds.h - rgba.height() as f32 * scale) / 2.0;
        let Some(mut source) = Pixmap::new(rgba.width(), rgba.height()) else {
            return;
        };
        for (target, input) in source
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(rgba.pixels())
        {
            let alpha = u16::from(input[3]);
            for channel in 0..3 {
                target[channel] = ((u16::from(input[channel]) * alpha + 127) / 255) as u8;
            }
            target[3] = input[3];
        }
        let ink = PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        if let Some(pixels) = self.pixels.as_mut() {
            pixels.draw_pixmap(
                0,
                0,
                source.as_ref(),
                &ink,
                Transform::from_row(scale, 0.0, 0.0, scale, x, y),
                None,
            );
        }
    }

    fn pixel_rect(&self, rect: Rect) -> RoundedRect {
        RoundedRect {
            x: (i32::from(rect.x) - i32::from(self.buffer.area.x)) as f32
                * f32::from(self.cells.width),
            y: (i32::from(rect.y) - i32::from(self.buffer.area.y)) as f32
                * f32::from(self.cells.height),
            w: f32::from(rect.width) * f32::from(self.cells.width),
            h: f32::from(rect.height) * f32::from(self.cells.height),
            radius: 0.0,
        }
    }
}

impl Deref for Canvas<'_> {
    type Target = Buffer;

    fn deref(&self) -> &Buffer {
        self.buffer
    }
}

impl DerefMut for Canvas<'_> {
    fn deref_mut(&mut self) -> &mut Buffer {
        self.buffer
    }
}

struct RoundedRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
}

impl RoundedRect {
    fn path(&self) -> Option<Path> {
        let (x, y, w, h, r) = (self.x, self.y, self.w, self.h, self.radius);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let mut path = PathBuilder::new();
        if r == 0.0 {
            path.push_rect(tiny_skia::Rect::from_xywh(x, y, w, h)?);
        } else {
            let k = r * 0.552_284_8;
            path.move_to(x + r, y);
            path.line_to(x + w - r, y);
            path.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
            path.line_to(x + w, y + h - r);
            path.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
            path.line_to(x + r, y + h);
            path.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
            path.line_to(x, y + r);
            path.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
            path.close();
        }
        path.finish()
    }

    fn coverage(&self, left: f32, top: f32, cells: CellSize, half: u32) -> u32 {
        if self.radius == 0.0 {
            return SAMPLES * SAMPLES;
        }
        let mut covered = 0;
        for sy in 0..SAMPLES {
            for sx in 0..SAMPLES {
                let x = left + (sx as f32 + 0.5) * f32::from(cells.width) / SAMPLES as f32;
                let y = top
                    + (half as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * f32::from(cells.height)
                        / 2.0;
                let cx = x.clamp(self.radius, self.w - self.radius);
                let cy = y.clamp(self.radius, self.h - self.radius);
                if (x - cx).powi(2) + (y - cy).powi(2) <= self.radius.powi(2) {
                    covered += 1;
                }
            }
        }
        covered
    }
}

fn paint(color: Color) -> Paint<'static> {
    let [r, g, b] = rgb(color);
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, 255);
    paint.anti_alias = true;
    paint
}

fn background_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "▀" | "▄" | "█" | "▖" | "▗" | "▘" | "▙" | "▚" | "▛" | "▜" | "▝" | "▞" | "▟"
    )
}

fn foreground_quadrants(symbol: &str) -> [bool; 4] {
    match symbol {
        "▀" => [true, true, false, false],
        "▄" => [false, false, true, true],
        "█" => [true; 4],
        "▖" => [false, false, true, false],
        "▗" => [false, false, false, true],
        "▘" => [true, false, false, false],
        "▙" => [true, false, true, true],
        "▚" => [true, false, false, true],
        "▛" => [true, true, true, false],
        "▜" => [true, true, false, true],
        "▝" => [false, true, false, false],
        "▞" => [false, true, true, false],
        "▟" => [false, true, true, true],
        _ => [false; 4],
    }
}

fn half_colors(cell: &Cell) -> [Color; 2] {
    let quadrants = foreground_quadrants(cell.symbol());
    let half = |offset| {
        let count = u32::from(quadrants[offset]) + u32::from(quadrants[offset + 1]);
        blend(cell.fg, cell.bg, count, 2)
    };
    [half(0), half(2)]
}

fn blend(fg: Color, bg: Color, covered: u32, total: u32) -> Color {
    if covered == 0 {
        return bg;
    }
    if covered == total {
        return fg;
    }
    let fg = rgb(fg);
    let bg = rgb(bg);
    let channel = |index| {
        ((u32::from(fg[index]) * covered + u32::from(bg[index]) * (total - covered) + total / 2)
            / total) as u8
    };
    Color::Rgb(channel(0), channel(1), channel(2))
}

fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(index) => match index {
            0..=15 => ANSI[usize::from(index)],
            16..=231 => {
                let value = index - 16;
                let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
                [level(value / 36), level(value / 6 % 6), level(value % 6)]
            }
            232..=255 => [8 + (index - 232) * 10; 3],
        },
        Color::Reset | Color::Black => ANSI[0],
        Color::Red => ANSI[1],
        Color::Green => ANSI[2],
        Color::Yellow => ANSI[3],
        Color::Blue => ANSI[4],
        Color::Magenta => ANSI[5],
        Color::Cyan => ANSI[6],
        Color::Gray => ANSI[7],
        Color::DarkGray => ANSI[8],
        Color::LightRed => ANSI[9],
        Color::LightGreen => ANSI[10],
        Color::LightYellow => ANSI[11],
        Color::LightBlue => ANSI[12],
        Color::LightMagenta => ANSI[13],
        Color::LightCyan => ANSI[14],
        Color::White => ANSI[15],
    }
}

const ANSI: [[u8; 3]; 16] = [
    [0, 0, 0],
    [128, 0, 0],
    [0, 128, 0],
    [128, 128, 0],
    [0, 0, 128],
    [128, 0, 128],
    [0, 128, 128],
    [192, 192, 192],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [0, 0, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn canvas(buffer: &mut Buffer, native: bool, style: SurfaceStyle) -> Canvas<'_> {
        Canvas::new(buffer, CellSize::default(), native, style)
    }

    #[test]
    fn terminal_background_is_transparent_and_rounded_edges_keep_straight_alpha() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
        let mut canvas = Canvas::new(
            &mut buffer,
            CellSize::default(),
            true,
            SurfaceStyle::Rounded,
        );
        canvas.clear_background();
        canvas.rounded(Rect::new(1, 1, 8, 3), Color::Rgb(240, 240, 240), 2.0);
        let background = canvas.finish().unwrap();
        assert_eq!(background.pixels.get_pixel(0, 0)[3], 0);
        assert!(background
            .pixels
            .pixels()
            .any(|p| p[3] > 0 && p[3] < 255 && p[0] > 220));
        assert!(buffer.content.iter().all(|c| c.bg == Color::Reset));
    }

    #[test]
    fn odd_sized_cells_have_no_black_seams_between_wallpaper_tiles() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 3));
        let mut canvas = Canvas::new(
            &mut buffer,
            CellSize {
                width: 17,
                height: 37,
            },
            true,
            SurfaceStyle::Rounded,
        );
        for y in 0..3 {
            canvas.background_row(y, Color::Rgb(80, 60, 90), Color::Rgb(70, 50, 80));
        }
        let background = canvas.finish().unwrap();
        assert!(background
            .pixels
            .pixels()
            .all(|p| p[0] > 0 && p[1] > 0 && p[2] > 0 && p[3] == 255));
    }

    #[test]
    fn rounded_and_blocky_have_different_corners() {
        let area = Rect::new(0, 0, 6, 4);
        let mut rounded_buffer = Buffer::empty(area);
        let mut rounded = canvas(&mut rounded_buffer, true, SurfaceStyle::Rounded);
        rounded.rounded(area, Color::White, 2.0);
        let rounded_image = rounded.finish().unwrap().pixels;
        let mut block_buffer = Buffer::empty(area);
        let mut block = canvas(&mut block_buffer, true, SurfaceStyle::Blocky);
        block.rounded(area, Color::White, 2.0);
        let block_image = block.finish().unwrap().pixels;
        assert_eq!(rounded_image.get_pixel(0, 0), &Rgba([0, 0, 0, 255]));
        assert_eq!(block_image.get_pixel(0, 0), &Rgba([255; 4]));
        assert_eq!(rounded_image.get_pixel(24, 32), &Rgba([255; 4]));
        assert!(rounded_image
            .pixels()
            .any(|pixel| pixel[0] > 0 && pixel[0] < 255));
    }

    #[test]
    fn tall_card_fills_straight_sides_and_middle() {
        let area = Rect::new(0, 0, 8, 20);
        let mut buffer = Buffer::empty(area);
        let mut canvas = canvas(&mut buffer, true, SurfaceStyle::Rounded);
        canvas.rounded(area, Color::Rgb(70, 90, 110), 2.0);
        assert_eq!(canvas[(0, 10)].fg, Color::Rgb(70, 90, 110));
        let image = canvas.finish().unwrap().pixels;
        assert_eq!(image.get_pixel(0, 160), &Rgba([70, 90, 110, 255]));
        assert_eq!(image.get_pixel(32, 319), &Rgba([70, 90, 110, 255]));
    }

    #[test]
    fn fallback_preserves_both_wallpaper_halves_outside_corners() {
        let area = Rect::new(0, 0, 12, 6);
        let mut buffer = Buffer::empty(area);
        for cell in &mut buffer.content {
            cell.set_symbol("▀").set_fg(Color::Red).set_bg(Color::Blue);
        }
        let mut canvas = canvas(&mut buffer, false, SurfaceStyle::Rounded);
        canvas.rounded(area, Color::White, f32::INFINITY);
        assert_eq!(canvas[(0, 0)].fg, Color::Red);
        assert_eq!(canvas[(0, 0)].bg, Color::Blue);
        assert_eq!(canvas[(6, 2)].fg, Color::White);
        assert!(canvas.finish().is_none());
        assert_eq!(buffer[(0, 0)].symbol(), "▀");
    }

    #[test]
    fn fallback_blends_partial_corners_with_the_existing_surface() {
        let area = Rect::new(0, 0, 8, 4);
        let render = |background| {
            let mut buffer = Buffer::empty(area);
            for cell in &mut buffer.content {
                cell.set_symbol("▀").set_fg(background).set_bg(background);
            }
            let mut canvas = canvas(&mut buffer, false, SurfaceStyle::Rounded);
            canvas.rounded(area, Color::White, 2.0);
            drop(canvas);
            buffer
        };
        let dark = render(Color::Black);
        let red = render(Color::Rgb(200, 0, 0));
        assert!(dark.content.iter().zip(&red.content).any(|(dark, red)| {
            let [r, g, _] = rgb(red.fg);
            dark.fg != Color::White && r > g && dark.fg != red.fg
        }));
    }

    #[test]
    fn finish_keeps_text_and_navigation_but_removes_background_blocks() {
        let area = Rect::new(3, 5, 10, 1);
        let mut buffer = Buffer::empty(area);
        for (cell, symbol) in buffer
            .content
            .iter_mut()
            .zip(["▀", "▄", "█", "▙", "A", "◀", "●", "■", "╭", "▌"])
        {
            cell.set_symbol(symbol)
                .set_fg(Color::White)
                .set_bg(Color::Blue);
        }
        let background = canvas(&mut buffer, true, SurfaceStyle::Rounded)
            .finish()
            .unwrap();
        assert_eq!(background.area, area);
        assert_eq!(background.pixels.dimensions(), (80, 16));
        assert!(background.pixels.pixels().all(|pixel| pixel[3] == 255));
        let symbols: Vec<_> = buffer.content.iter().map(Cell::symbol).collect();
        assert_eq!(symbols, [" ", " ", " ", " ", "A", "◀", "●", "■", "╭", "▌"]);
        assert!(buffer.content.iter().all(|cell| cell.bg == Color::Reset));
    }

    #[test]
    fn icons_keep_their_aspect_ratio_and_are_centered() {
        let area = Rect::new(5, 8, 8, 4);
        let mut buffer = Buffer::empty(area);
        let mut canvas = canvas(&mut buffer, true, SurfaceStyle::Rounded);
        canvas.fill(area, Color::Blue);
        let icon = RgbaImage::from_pixel(20, 10, Rgba([255, 0, 0, 255]));
        canvas.icon(area, &icon);
        let image = canvas.finish().unwrap().pixels;
        assert_eq!(image.get_pixel(32, 4), &Rgba([0, 0, 128, 255]));
        assert_eq!(image.get_pixel(32, 20), &Rgba([255, 0, 0, 255]));
        assert_eq!(image.get_pixel(32, 59), &Rgba([0, 0, 128, 255]));
        let red_rows: Vec<_> = (0..64)
            .filter(|y| image.get_pixel(32, *y)[0] > 128)
            .collect();
        assert_eq!(red_rows, (16..48).collect::<Vec<_>>());
    }

    #[test]
    fn drawing_is_clipped_and_relative_to_the_buffer_origin() {
        let area = Rect::new(10, 20, 3, 2);
        let mut buffer = Buffer::empty(area);
        let mut canvas = canvas(&mut buffer, true, SurfaceStyle::Rounded);
        canvas.fill(Rect::new(0, 0, 11, 21), Color::Red);
        canvas.block(
            12,
            21,
            "▀",
            &Style::default().fg(Color::Green).bg(Color::Blue),
        );
        let image = canvas.finish().unwrap().pixels;
        assert_eq!(image.get_pixel(0, 0), &Rgba([128, 0, 0, 255]));
        assert_eq!(image.get_pixel(8, 0), &Rgba([0, 0, 0, 255]));
        assert_eq!(image.get_pixel(16, 16), &Rgba([0, 128, 0, 255]));
        assert_eq!(image.get_pixel(16, 24), &Rgba([0, 0, 128, 255]));
    }

    #[test]
    fn oversized_viewports_fall_back_without_clearing_cells() {
        let area = Rect::new(0, 0, 1, u16::MAX);
        let mut buffer = Buffer::empty(area);
        buffer[(0, 0)].set_symbol("▀").set_bg(Color::Blue);
        let canvas = Canvas::new(
            &mut buffer,
            CellSize {
                width: 64,
                height: 128,
            },
            true,
            SurfaceStyle::Rounded,
        );
        assert!(!canvas.native());
        assert!(canvas.finish().is_none());
        assert_eq!(buffer[(0, 0)].symbol(), "▀");
        assert_eq!(buffer[(0, 0)].bg, Color::Blue);
    }

    #[test]
    fn empty_rectangles_and_untrusted_cell_sizes_are_safe() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 2));
        let mut canvas = Canvas::new(
            &mut buffer,
            CellSize {
                width: 0,
                height: u16::MAX,
            },
            true,
            SurfaceStyle::Rounded,
        );
        canvas.rounded(Rect::default(), Color::Red, f32::NAN);
        canvas.fill(Rect::default(), Color::Blue);
        canvas.icon(Rect::default(), &RgbaImage::new(0, 0));
        assert_eq!(canvas.finish().unwrap().pixels.dimensions(), (2, 256));
        let mut empty = Buffer::empty(Rect::default());
        assert!(!self::canvas(&mut empty, true, SurfaceStyle::Rounded).native());
    }
}
