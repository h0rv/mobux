use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use font8x8::{UnicodeFonts, BASIC_FONTS, BLOCK_FONTS};
use image::{Rgb, RgbImage};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::Terminal;

use crate::paint::{layout, scene};
use crate::state::{State, CATALOG};
use crate::surface::{Background, CellSize};
use crate::theme::{self, SkinId};

const SCALE: u32 = 2;
const CELL_W: u32 = 8 * SCALE;
const CELL_H: u32 = 16 * SCALE;

pub struct HomeShot {
    pub name: &'static str,
    pub cols: u16,
    pub rows: u16,
}

pub const HOME_SHOTS: &[HomeShot] = &[
    HomeShot {
        name: "home-mobile",
        cols: 40,
        rows: 48,
    },
    HomeShot {
        name: "home-desktop",
        cols: 120,
        rows: 36,
    },
];

pub fn write_home(dir: &Path) -> io::Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut paths = Vec::with_capacity(HOME_SHOTS.len() + 18);
    for shot in HOME_SHOTS {
        let image = home_image(shot.cols, shot.rows)?;
        paths.push(save_png(dir, shot.name, &image)?);
    }
    let android = screen_image(40, 48, |state| state.skin = SkinId::Android)?;
    paths.push(save_png(dir, "android-mobile", &android)?);
    let active = screen_image(40, 48, |state| {
        session_fixture(state);
        state.screen = crate::state::Screen::Home;
    })?;
    paths.push(save_png(dir, "workspace-active-mobile", &active)?);
    let install = screen_image(40, 48, |state| {
        state.open_store();
        state.pending_install =
            crate::install::Plan::for_app(crate::catalog::by_bin("gh").unwrap(), true);
    })?;
    paths.push(save_png(dir, "install-mobile", &install)?);
    let cells = screen_image(40, 48, |state| state.image_icons = false)?;
    paths.push(save_png(dir, "home-cells-mobile", &cells)?);
    let one_dark = screen_image(40, 48, |state| {
        state.palette = crate::palette::Palette::OneDarkPro
    })?;
    paths.push(save_png(dir, "one-dark-mobile", &one_dark)?);
    let light = screen_image(40, 48, |state| {
        state.palette = crate::palette::Palette::GruvboxMaterialLight
    })?;
    paths.push(save_png(dir, "light-mobile", &light)?);
    for (name, palette) in [
        (
            "catppuccin-mocha-mobile",
            crate::palette::Palette::CatppuccinMocha,
        ),
        (
            "catppuccin-latte-mobile",
            crate::palette::Palette::CatppuccinLatte,
        ),
        (
            "tokyo-night-mobile",
            crate::palette::Palette::TokyoNightDark,
        ),
    ] {
        let image = screen_image(40, 48, |state| state.palette = palette)?;
        paths.push(save_png(dir, name, &image)?);
    }
    let blocky = screen_image(40, 48, |state| {
        state.surface_style = theme::SurfaceStyle::Blocky
    })?;
    paths.push(save_png(dir, "home-blocky-mobile", &blocky)?);
    let terminal = screen_image(40, 48, |state| state.wallpaper = false)?;
    paths.push(save_png(dir, "home-terminal-mobile", &terminal)?);
    let apps = screen_image(40, 48, |state| {
        state.open_store();
        state.installed = CATALOG
            .iter()
            .map(|app| matches!(app.id.as_str(), "opencode" | "micro"))
            .collect();
    })?;
    paths.push(save_png(dir, "apps-mobile", &apps)?);
    let search = screen_image(40, 48, |state| {
        state.screen = crate::state::Screen::Search;
        state.query = "app".to_string();
        state.hits = vec![crate::find::Hit {
            title: "src/app.rs".to_string(),
            detail: "file".to_string(),
            path: PathBuf::from("/home/you/projects/demo/src/app.rs"),
            is_dir: false,
        }];
    })?;
    paths.push(save_png(dir, "search-mobile", &search)?);
    let files = screen_image(40, 48, |state| {
        use crate::state::{Node, NodeKind, Screen};
        state.screen = Screen::Files;
        state.cwd = PathBuf::from("/home/you/projects/demo");
        state.files = [
            ("Up", NodeKind::Parent),
            ("src", NodeKind::Dir),
            ("assets", NodeKind::Dir),
            ("README.md", NodeKind::File),
            ("Cargo.toml", NodeKind::File),
        ]
        .into_iter()
        .map(|(name, kind)| Node {
            name: name.to_string(),
            path: state.cwd.join(name),
            kind,
        })
        .collect();
        let mut tree = crate::tree::Tree::new(state.cwd.clone(), &state.files);
        tree.rows.insert(
            2,
            crate::tree::Row {
                node: Node {
                    name: "main.rs".into(),
                    path: state.cwd.join("src/main.rs"),
                    kind: NodeKind::File,
                },
                depth: 1,
                expanded: false,
            },
        );
        tree.rows[1].expanded = true;
        state.tree = Some(tree);
    })?;
    paths.push(save_png(dir, "files-mobile", &files)?);
    let iphone = screen_image(40, 48, |state| {
        state.skin = SkinId::Iphone;
        state.palette = crate::palette::Palette::GruvboxMaterialLight;
    })?;
    paths.push(save_png(dir, "iphone-mobile", &iphone)?);
    let settings = screen_image(40, 48, |state| state.open_settings())?;
    paths.push(save_png(dir, "settings-mobile", &settings)?);
    let recents = screen_image(40, 48, session_fixture)?;
    paths.push(save_png(dir, "recents-mobile", &recents)?);
    let stop = screen_image(40, 48, |state| {
        session_fixture(state);
        state.pending_stop = Some(state.live_sessions[0].id.clone());
    })?;
    paths.push(save_png(dir, "stop-mobile", &stop)?);
    Ok(paths)
}

fn session_fixture(state: &mut State) {
    state.registered_apps = vec!["opencode".into(), "hunk".into()];
    use crate::sessions::{LaunchSpec, ManagedSession, SessionStatus};
    state.session_backend = true;
    state.replace_live_sessions(vec![
        ManagedSession {
            id: "mobux-00000000-0000-4000-8000-000000000001"
                .parse()
                .unwrap(),
            spec: LaunchSpec {
                title: "OpenCode".to_string(),
                bin: "opencode".to_string(),
                args: vec![],
                cwd: state.cwd.clone(),
            },
            status: SessionStatus::Running,
        },
        ManagedSession {
            id: "mobux-00000000-0000-4000-8000-000000000002"
                .parse()
                .unwrap(),
            spec: LaunchSpec {
                title: "Hunk".to_string(),
                bin: "hunk".to_string(),
                args: vec!["diff".to_string()],
                cwd: state.cwd.clone(),
            },
            status: SessionStatus::Exited(Some(0)),
        },
    ]);
    state.open_recents();
}

fn save_png(dir: &Path, name: &str, image: &RgbImage) -> io::Result<PathBuf> {
    let path = dir.join(format!("{name}.png"));
    image.save(&path).map_err(io::Error::other)?;
    Ok(path)
}

pub fn home_image(cols: u16, rows: u16) -> io::Result<RgbImage> {
    screen_image(cols, rows, |_| {})
}

pub fn home_buffer(cols: u16, rows: u16) -> io::Result<Buffer> {
    screen_buffer(cols, rows, |_| {})
}

fn screen_image(cols: u16, rows: u16, setup: impl FnOnce(&mut State)) -> io::Result<RgbImage> {
    let (buffer, background) = screen_scene(cols, rows, |state| {
        state.image_icons = true;
        setup(state);
    })?;
    if let Some(background) = background {
        // A preview needs a concrete backdrop for transparent terminal pixels.
        // The live terminal uses its own configured background instead.
        let mut image = RgbImage::from_fn(
            background.pixels.width(),
            background.pixels.height(),
            |x, y| {
                let p = background.pixels.get_pixel(x, y);
                let a = u32::from(p[3]);
                Rgb([0, 1, 2].map(|i| ((u32::from(p[i]) * a + 24 * (255 - a)) / 255) as u8))
            },
        );
        paint_text_layer(&mut image, &buffer);
        Ok(image)
    } else {
        Ok(buffer_image(&buffer))
    }
}

// Preview typography approximates a modern terminal font. Live text remains native.
fn paint_text_layer(image: &mut RgbImage, buffer: &Buffer) {
    static FONT: std::sync::LazyLock<fontdue::Font> = std::sync::LazyLock::new(|| {
        fontdue::Font::from_bytes(
            include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .expect("bundled font")
    });
    let size = CELL_H as f32 * 0.75;
    let line = FONT
        .horizontal_line_metrics(size)
        .expect("font line metrics");
    let baseline = ((CELL_H as f32 - line.new_line_size) / 2.0 + line.ascent).round() as i32;
    for row in 0..buffer.area.height {
        for col in 0..buffer.area.width {
            let Some(cell) = buffer.cell(Position::new(col, row)) else {
                continue;
            };
            if cell.symbol().trim().is_empty() {
                continue;
            }
            let mut mask = RgbImage::new(CELL_W, CELL_H);
            let symbol = cell.symbol().chars().next().unwrap_or(' ');
            if FONT.lookup_glyph_index(symbol) != 0 {
                let (metrics, bitmap) = FONT.rasterize(symbol, size);
                let left =
                    ((CELL_W as f32 - metrics.advance_width) / 2.0).floor() as i32 + metrics.xmin;
                let top = baseline - metrics.height as i32 - metrics.ymin;
                for y in 0..metrics.height {
                    for x in 0..metrics.width {
                        let px = left + x as i32;
                        let py = top + y as i32;
                        if px >= 0 && py >= 0 && px < CELL_W as i32 && py < CELL_H as i32 {
                            mask.put_pixel(
                                px as u32,
                                py as u32,
                                Rgb([bitmap[y * metrics.width + x]; 3]),
                            );
                        }
                    }
                }
            } else {
                paint_cell(&mut mask, 0, 0, [255, 255, 255], [0, 0, 0], cell.symbol());
            }
            for (x, y, pixel) in mask.enumerate_pixels() {
                if pixel[0] != 0 {
                    image.put_pixel(
                        u32::from(col) * CELL_W + x,
                        u32::from(row) * CELL_H + y,
                        Rgb({
                            let fg = if cell.fg == Color::Reset {
                                [230, 230, 230]
                            } else {
                                rgb_of(cell.fg)
                            };
                            let old = image
                                .get_pixel(u32::from(col) * CELL_W + x, u32::from(row) * CELL_H + y)
                                .0;
                            std::array::from_fn(|i| {
                                ((u32::from(fg[i]) * u32::from(pixel[0])
                                    + u32::from(old[i]) * (255 - u32::from(pixel[0])))
                                    / 255) as u8
                            })
                        }),
                    );
                }
            }
        }
    }
}

fn screen_buffer(cols: u16, rows: u16, setup: impl FnOnce(&mut State)) -> io::Result<Buffer> {
    Ok(screen_scene(cols, rows, setup)?.0)
}

fn screen_scene(
    cols: u16,
    rows: u16,
    setup: impl FnOnce(&mut State),
) -> io::Result<(Buffer, Option<Background>)> {
    let backend = TestBackend::new(cols, rows);
    let mut terminal = Terminal::new(backend)?;
    let mut state = State::new();
    state.cwd = PathBuf::from("/home/you/projects/demo");
    state.session_backend = true;
    state.clock_override = Some(("12:00".to_string(), "Fri, Sep 22".to_string()));
    setup(&mut state);
    let area = Rect::new(0, 0, cols, rows);
    state.metrics = theme::metrics(state.type_size, state.app_size, area);
    state.places = layout(area, &state.theme(), state.metrics);
    let mut background = None;
    terminal.draw(|frame| {
        background = scene(
            frame,
            &state,
            CellSize {
                width: CELL_W as u16,
                height: CELL_H as u16,
            },
        )
    })?;
    Ok((terminal.backend().buffer().clone(), background))
}

pub fn buffer_image(buffer: &Buffer) -> RgbImage {
    let width = u32::from(buffer.area.width) * CELL_W;
    let height = u32::from(buffer.area.height) * CELL_H;
    let mut image = RgbImage::from_pixel(width, height, Rgb([0, 0, 0]));
    for row in 0..buffer.area.height {
        for col in 0..buffer.area.width {
            let Some(cell) = buffer.cell(Position::new(col, row)) else {
                continue;
            };
            let origin_x = u32::from(col) * CELL_W;
            let origin_y = u32::from(row) * CELL_H;
            paint_cell(
                &mut image,
                origin_x,
                origin_y,
                rgb_of(cell.fg),
                rgb_of(cell.bg),
                cell.symbol(),
            );
        }
    }
    image
}

fn paint_cell(image: &mut RgbImage, x: u32, y: u32, fg: [u8; 3], bg: [u8; 3], symbol: &str) {
    let ch = symbol.chars().next().unwrap_or(' ');
    match ch {
        ' ' | '\0' => fill(image, x, y, CELL_W, CELL_H, bg),
        '●' => fill_circle(image, x, y, fg, bg),
        '█' | '■' => fill(image, x, y, CELL_W, CELL_H, fg),
        '─' => {
            fill(image, x, y, CELL_W, CELL_H, bg);
            fill(image, x, y + CELL_H / 2, CELL_W, 2, fg);
        }
        '│' => {
            fill(image, x, y, CELL_W, CELL_H, bg);
            fill(image, x + CELL_W / 2, y, 2, CELL_H, fg);
        }
        '◀' => triangle(image, x, y, fg, bg),
        other => {
            if let Some(mask) = quadrant_mask(other) {
                paint_quadrants(image, x, y, fg, bg, mask);
            } else {
                blit_glyph(image, x, y, fg, bg, glyph(other));
            }
        }
    }
}

fn quadrant_mask(ch: char) -> Option<u8> {
    Some(match ch {
        '▘' => 0b0001,
        '▝' => 0b0010,
        '▖' => 0b0100,
        '▗' => 0b1000,
        '▀' => 0b0011,
        '▄' => 0b1100,
        '▌' => 0b0101,
        '▐' => 0b1010,
        '▛' => 0b0111,
        '▜' => 0b1011,
        '▙' => 0b1101,
        '▟' => 0b1110,
        '▚' => 0b1001,
        '▞' => 0b0110,
        _ => return None,
    })
}

fn paint_quadrants(image: &mut RgbImage, x: u32, y: u32, fg: [u8; 3], bg: [u8; 3], mask: u8) {
    fill(image, x, y, CELL_W, CELL_H, bg);
    let half_w = CELL_W / 2;
    let half_h = CELL_H / 2;
    if mask & 0b0001 != 0 {
        fill(image, x, y, half_w, half_h, fg);
    }
    if mask & 0b0010 != 0 {
        fill(image, x + half_w, y, CELL_W - half_w, half_h, fg);
    }
    if mask & 0b0100 != 0 {
        fill(image, x, y + half_h, half_w, CELL_H - half_h, fg);
    }
    if mask & 0b1000 != 0 {
        fill(
            image,
            x + half_w,
            y + half_h,
            CELL_W - half_w,
            CELL_H - half_h,
            fg,
        );
    }
}

fn glyph(ch: char) -> Option<[u8; 8]> {
    BASIC_FONTS
        .get(ch)
        .or_else(|| BLOCK_FONTS.get(ch))
        .or_else(|| BASIC_FONTS.get('?'))
}

fn blit_glyph(
    image: &mut RgbImage,
    x: u32,
    y: u32,
    fg: [u8; 3],
    bg: [u8; 3],
    rows: Option<[u8; 8]>,
) {
    fill(image, x, y, CELL_W, CELL_H, bg);
    let Some(rows) = rows else {
        return;
    };
    let top = (CELL_H - 8 * SCALE) / 2;
    for (row_index, row) in rows.iter().enumerate() {
        for bit in 0..8u32 {
            // font8x8 stores the leftmost pixel in the least significant bit.
            let on = (*row & (1_u8 << bit)) != 0;
            if !on {
                continue;
            }
            let px = x + bit * SCALE;
            let py = y + top + u32::try_from(row_index).unwrap_or(0) * SCALE;
            fill(image, px, py, SCALE, SCALE, fg);
        }
    }
}

fn triangle(image: &mut RgbImage, x: u32, y: u32, fg: [u8; 3], bg: [u8; 3]) {
    fill(image, x, y, CELL_W, CELL_H, bg);
    let width = CELL_W - 2 * SCALE;
    let height = CELL_W - SCALE;
    let top = (CELL_H - height) / 2;
    let right = x + (CELL_W + width) / 2;
    for dy in 0..height {
        let reach = dy.min(height - 1 - dy) + 1;
        let span = (reach * width / (height / 2)).max(1);
        fill(image, right - span, y + top + dy, span, 1, fg);
    }
}

fn fill_circle(image: &mut RgbImage, x: u32, y: u32, fg: [u8; 3], bg: [u8; 3]) {
    // Doubled pixel coordinates keep the circle centered in an even-sized cell.
    let radius = i32::try_from(CELL_W.min(CELL_H) - SCALE).unwrap_or(0);
    let cx = i32::try_from(CELL_W).unwrap_or(0);
    let cy = i32::try_from(CELL_H).unwrap_or(0);
    for dy in 0..CELL_H {
        for dx in 0..CELL_W {
            let ox = 2 * i32::try_from(dx).unwrap_or(0) + 1 - cx;
            let oy = 2 * i32::try_from(dy).unwrap_or(0) + 1 - cy;
            let color = if ox * ox + oy * oy <= radius * radius {
                fg
            } else {
                bg
            };
            put(image, x + dx, y + dy, color);
        }
    }
}

fn fill(image: &mut RgbImage, x: u32, y: u32, width: u32, height: u32, color: [u8; 3]) {
    for dy in 0..height {
        for dx in 0..width {
            put(image, x + dx, y + dy, color);
        }
    }
}

fn put(image: &mut RgbImage, x: u32, y: u32, color: [u8; 3]) {
    if x < image.width() && y < image.height() {
        image.put_pixel(x, y, Rgb(color));
    }
}

fn rgb_of(color: Color) -> [u8; 3] {
    match color {
        Color::Reset | Color::Black => [0, 0, 0],
        Color::Red => [205, 49, 49],
        Color::Green => [13, 188, 121],
        Color::Yellow => [229, 229, 16],
        Color::Blue => [36, 114, 200],
        Color::Magenta => [188, 63, 188],
        Color::Cyan => [17, 168, 205],
        Color::Gray => [168, 168, 168],
        Color::DarkGray => [128, 128, 128],
        Color::LightRed => [255, 85, 85],
        Color::LightGreen => [80, 250, 123],
        Color::LightYellow => [255, 255, 85],
        Color::LightBlue => [114, 159, 207],
        Color::LightMagenta => [255, 119, 255],
        Color::LightCyan => [128, 255, 255],
        Color::White => [255, 255, 255],
        Color::Rgb(red, green, blue) => [red, green, blue],
        Color::Indexed(index) => {
            let value = index.saturating_mul(2);
            [value, value, value]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol_image(symbol: &str) -> RgbImage {
        let mut image = RgbImage::new(CELL_W, CELL_H);
        paint_cell(&mut image, 0, 0, [255; 3], [0; 3], symbol);
        image
    }

    #[test]
    fn screenshot_cells_have_terminal_aspect_ratio() {
        let buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        let image = buffer_image(&buffer);
        assert_eq!(image.dimensions(), (3 * 16, 2 * 32));
    }

    #[test]
    fn asymmetric_f_glyph_is_not_mirrored_and_is_vertically_centered() {
        let image = symbol_image("F");
        // Known font8x8 F, written in visual left-to-right order.
        let expected = [
            "#######.", ".##...#.", ".##.#...", ".####...", ".##.#...", ".##.....", "####....",
            "........",
        ];
        let top = (CELL_H - 8 * SCALE) / 2;
        for (x, y, pixel) in image.enumerate_pixels() {
            let on = (top..top + 8 * SCALE).contains(&y)
                && expected[((y - top) / SCALE) as usize].as_bytes()[(x / SCALE) as usize] == b'#';
            assert_eq!(pixel.0, if on { [255; 3] } else { [0; 3] }, "({x}, {y})");
        }
    }

    #[test]
    fn navigation_circle_is_round_centered_and_fills_the_cell_width() {
        let image = symbol_image("●");
        let horizontal_diameter = (0..CELL_W)
            .filter(|&x| image.get_pixel(x, CELL_H / 2).0 == [255; 3])
            .count();
        let vertical_diameter = (0..CELL_H)
            .filter(|&y| image.get_pixel(CELL_W / 2, y).0 == [255; 3])
            .count();
        assert_eq!(horizontal_diameter, 14);
        assert_eq!(vertical_diameter, horizontal_diameter);
        for (x, y, pixel) in image.enumerate_pixels() {
            assert_eq!(pixel, image.get_pixel(CELL_W - 1 - x, y));
            assert_eq!(pixel, image.get_pixel(x, CELL_H - 1 - y));
        }
        assert_eq!(image.get_pixel(1, 9).0, [0; 3]);
        assert_eq!(image.get_pixel(1, CELL_H / 2).0, [255; 3]);
    }

    #[test]
    fn navigation_triangle_points_left_and_is_centered() {
        let image = symbol_image("◀");
        assert_eq!(image.get_pixel(2, 15).0, [255; 3]);
        assert_eq!(image.get_pixel(2, 9).0, [0; 3]);
        assert_eq!(image.get_pixel(13, 9).0, [255; 3]);
        assert_eq!(image.get_pixel(13, 22).0, [255; 3]);
        assert_eq!(image.get_pixel(13, 8).0, [0; 3]);
        assert_eq!(image.get_pixel(1, 15).0, [0; 3]);
        assert_eq!(image.get_pixel(14, 15).0, [0; 3]);
        for (x, y, pixel) in image.enumerate_pixels() {
            assert_eq!(pixel, image.get_pixel(x, CELL_H - 1 - y));
        }
    }

    #[test]
    fn mobile_home_uses_the_selected_palette_for_wallpaper_and_search() {
        let mut state = State::new();
        let area = Rect::new(0, 0, 40, 48);
        state.metrics = theme::metrics(state.type_size, state.app_size, area);
        state.places = layout(area, &state.theme(), state.metrics);
        let theme = state.theme();
        let image = home_image(40, 48).unwrap();
        assert_eq!(image.dimensions(), (40 * CELL_W, 48 * CELL_H));
        let (_, r, g, b) = theme.stops[0];
        assert_eq!(image.get_pixel(image.width() / 2, 2).0, [r, g, b]);
        let pill = state.places.search;
        let pixel = image.get_pixel(
            u32::from(pill.x + pill.width / 2) * CELL_W + CELL_W / 2,
            u32::from(pill.y + pill.height / 2) * CELL_H + CELL_H / 2,
        );
        assert_eq!(pixel.0, [theme.card.r, theme.card.g, theme.card.b]);
    }

    #[test]
    fn write_home_creates_all_previews() {
        let dir = std::env::temp_dir().join(format!("mobux-shots-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = write_home(&dir).unwrap();
        assert_eq!(paths.len(), 20);
        for path in paths {
            let bytes = fs::read(&path).unwrap();
            assert!(bytes.starts_with(b"\x89PNG"));
            assert!(bytes.len() > 1_000);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn iphone_uses_the_chosen_light_palette_instead_of_a_separate_color_system() {
        let mut state = State::new();
        state.skin = SkinId::Iphone;
        state.palette = crate::palette::Palette::GruvboxMaterialLight;
        let (_, r, g, b) = state.theme().stops[0];
        let image = screen_image(40, 48, |target| {
            target.skin = state.skin;
            target.palette = state.palette;
        })
        .unwrap();
        assert_eq!(image.get_pixel(image.width() / 2, 2).0, [r, g, b]);
    }
}
