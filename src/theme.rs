use ratatui::layout::Rect;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::palette::Scheme;

/// A home screen skin. Paint reads this and does not special-case the phone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinId {
    #[default]
    Minimal,
    Android,
    Iphone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeSize {
    Small,
    #[default]
    Regular,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSize {
    Small,
    #[default]
    Regular,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceStyle {
    #[default]
    Rounded,
    Blocky,
}

impl SurfaceStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rounded => "Rounded",
            Self::Blocky => "Blocky",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub label: &'static str,
    pub workspace: bool,
    pub stops: [(u16, u8, u8, u8); 5],
    pub ink: Rgb,
    pub muted: Rgb,
    pub card: Rgb,
    pub chip: Rgb,
    pub selected: Rgb,
    pub pill: Rgb,
    pub pill_ink: Rgb,
    pub nav: Rgb,
    pub nav_ink: Rgb,
    pub folder: Rgb,
    pub shell: Rgb,
    pub prompt: Rgb,
    pub dock: Option<Rgb>,
    pub show_clock: bool,
    /// Android keeps a search pill above the buttons. iPhone uses a top chip.
    pub search_bar: bool,
    /// iPhone draws a home pill instead of the three Android buttons.
    pub home_indicator: bool,
    pub marks: [&'static str; 3],
    pub search_label: &'static str,
}

const ANDROID_STOPS: [(u16, u8, u8, u8); 5] = [
    (0, 94, 70, 104),
    (275, 82, 59, 94),
    (550, 70, 48, 84),
    (820, 39, 28, 55),
    (1000, 22, 18, 32),
];

/// iOS 26 Sky. Pale glass at the top, deeper blue toward the dock.
const IPHONE_STOPS: [(u16, u8, u8, u8); 5] = [
    (0, 186, 222, 255),
    (220, 236, 246, 255),
    (480, 126, 186, 236),
    (760, 64, 122, 206),
    (1000, 24, 58, 128),
];

impl SkinId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Android => "Android",
            Self::Iphone => "iPhone",
        }
    }

    pub fn theme(self) -> Theme {
        match self {
            Self::Minimal => Theme {
                label: "Minimal",
                workspace: true,
                marks: ["Back", "Home", "Apps"],
                show_clock: false,
                search_bar: false,
                dock: None,
                ..Self::Android.theme()
            },
            Self::Android => Theme {
                label: "Android",
                workspace: false,
                stops: ANDROID_STOPS,
                ink: Rgb::new(255, 255, 255),
                muted: Rgb::new(198, 186, 210),
                card: Rgb::new(28, 16, 42),
                chip: Rgb::new(48, 28, 64),
                selected: Rgb::new(66, 48, 86),
                pill: Rgb::new(255, 255, 255),
                pill_ink: Rgb::new(80, 80, 80),
                nav: Rgb::new(18, 8, 28),
                nav_ink: Rgb::new(255, 255, 255),
                folder: Rgb::new(170, 210, 255),
                shell: Rgb::new(12, 10, 16),
                prompt: Rgb::new(255, 196, 214),
                dock: None,
                show_clock: true,
                search_bar: true,
                home_indicator: false,
                marks: ["◀", "●", "■"],
                search_label: "Search",
            },
            Self::Iphone => Theme {
                label: "iPhone",
                workspace: false,
                stops: IPHONE_STOPS,
                ink: Rgb::new(16, 28, 58),
                muted: Rgb::new(70, 96, 130),
                card: Rgb::new(255, 255, 255),
                chip: Rgb::new(220, 232, 244),
                selected: Rgb::new(186, 220, 250),
                pill: Rgb::new(255, 255, 255),
                pill_ink: Rgb::new(40, 50, 70),
                nav: Rgb::new(232, 242, 252),
                nav_ink: Rgb::new(16, 28, 58),
                folder: Rgb::new(16, 90, 180),
                shell: Rgb::new(12, 16, 28),
                prompt: Rgb::new(150, 200, 255),
                dock: Some(Rgb::new(232, 242, 252)),
                show_clock: false,
                search_bar: false,
                home_indicator: false,
                marks: ["◀", "●", "■"],
                search_label: "Search",
            },
        }
    }
}

impl Theme {
    /// Replace colors without changing the skin's layout or behavior.
    pub fn with_palette(mut self, scheme: &Scheme) -> Self {
        let colors = &scheme.colors;
        self.ink = colors[0x05];
        self.muted = colors[0x04];
        self.card = colors[0x01];
        self.chip = colors[0x02];
        self.selected = colors[0x03];
        // Base16 reverses its neutral ramp for light schemes, so this pair
        // keeps the search pill readable in either variant without fixed white.
        self.pill = colors[0x05];
        self.pill_ink = colors[0x00];
        self.nav = colors[0x00];
        self.nav_ink = colors[0x05];
        self.folder = colors[0x0D];
        self.shell = colors[0x00];
        self.prompt = colors[0x0B];
        self.dock = self.dock.map(|_| colors[0x01]);

        // Tint the background gently with the scheme's blue accent. Keep each
        // skin's stop positions while avoiding unrelated wallpaper colors.
        let weights = if self.workspace {
            [0; 5]
        } else {
            [12, 10, 7, 4, 0]
        };
        for (stop, weight) in self.stops.iter_mut().zip(weights) {
            let color = mix(colors[0x00], colors[0x0D], weight);
            *stop = (stop.0, color.r, color.g, color.b);
        }
        self
    }
}

fn mix(background: Rgb, accent: Rgb, percent: u16) -> Rgb {
    let channel = |back: u8, front: u8| {
        ((u16::from(back) * (100 - percent) + u16::from(front) * percent + 50) / 100) as u8
    };
    Rgb::new(
        channel(background.r, accent.r),
        channel(background.g, accent.g),
        channel(background.b, accent.b),
    )
}

impl TypeSize {
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Regular => "Regular",
            Self::Large => "Large",
        }
    }
}

impl AppSize {
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Regular => "Regular",
            Self::Large => "Large",
        }
    }
}

const SKINS: [SkinId; 3] = [SkinId::Minimal, SkinId::Android, SkinId::Iphone];
const TYPES: [TypeSize; 3] = [TypeSize::Small, TypeSize::Regular, TypeSize::Large];
const APPS: [AppSize; 3] = [AppSize::Small, AppSize::Regular, AppSize::Large];

pub fn step_skin(current: SkinId, dir: isize) -> SkinId {
    step(&SKINS, current, dir)
}

pub fn step_type(current: TypeSize, dir: isize) -> TypeSize {
    step(&TYPES, current, dir)
}

pub fn step_apps(current: AppSize, dir: isize) -> AppSize {
    step(&APPS, current, dir)
}

fn step<T: Copy + PartialEq>(items: &[T], current: T, dir: isize) -> T {
    let index = items.iter().position(|item| *item == current).unwrap_or(0);
    let len = items.len().max(1);
    let moved = if dir < 0 { index + len - 1 } else { index + 1 };
    items[moved % len]
}

/// How many cells a finger needs. The phone font stays the terminal font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub icon_w: u16,
    pub icon_body: u16,
    pub search_h: u16,
    pub nav_h: u16,
    pub row_h: u16,
}

impl Default for Metrics {
    fn default() -> Self {
        metrics(TypeSize::Regular, AppSize::Regular, Rect::new(0, 0, 80, 40))
    }
}

impl Metrics {
    pub fn icon_h(self) -> u16 {
        self.icon_body.saturating_add(1)
    }

    pub fn recent_stride(self) -> u16 {
        self.row_h.saturating_add(1)
    }
}

pub fn metrics(type_size: TypeSize, app_size: AppSize, area: Rect) -> Metrics {
    let tall = area.height >= 28;
    let roomy = area.height >= 18;
    let (mut icon_w, mut icon_body): (u16, u16) = match app_size {
        AppSize::Small => (6, 3),
        AppSize::Regular => {
            if roomy {
                (8, 4)
            } else {
                (5, 3)
            }
        }
        AppSize::Large => {
            if area.width >= 70 {
                (10, 5)
            } else {
                (8, 4)
            }
        }
    };
    if matches!(type_size, TypeSize::Large) && roomy {
        icon_body = icon_body.saturating_add(1);
    }
    let count = 5u16;
    while icon_w > 4 && icon_w.saturating_mul(count) > area.width {
        icon_w = icon_w.saturating_sub(1);
    }
    if !roomy {
        icon_body = icon_body.min(3);
    }
    let search_h = if !roomy {
        1
    } else {
        match type_size {
            TypeSize::Small => 1,
            TypeSize::Regular => {
                if tall {
                    3
                } else {
                    1
                }
            }
            TypeSize::Large => 3,
        }
    };
    let nav_h = if tall { 2 } else { 1 };
    let row_h = if matches!(type_size, TypeSize::Large) && roomy {
        3
    } else {
        2
    };
    Metrics {
        icon_w,
        icon_body,
        search_h,
        nav_h,
        row_h,
    }
}

pub fn slot_count(height: u16, header: u16, footer: u16, stride: u16) -> usize {
    usize::from(height.saturating_sub(header).saturating_sub(footer) / stride.max(1))
}

pub fn slot_at(origin_y: u16, header: u16, row: u16, stride: u16, row_h: u16) -> Option<usize> {
    let start = origin_y.saturating_add(header);
    if row < start {
        return None;
    }
    let delta = row - start;
    let stride = stride.max(1);
    if delta % stride >= row_h {
        return None;
    }
    Some(usize::from(delta / stride))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_wallpaper_starts_plum_and_iphone_starts_blue() {
        let android = SkinId::Android.theme().stops[0];
        let iphone = SkinId::Iphone.theme().stops[0];
        assert!(
            android.1 > android.2 && android.3 > android.2,
            "android top should be plum"
        );
        assert!(iphone.3 > iphone.1, "iphone top should be blue");
    }

    #[test]
    fn palette_keeps_skin_geometry_and_flags() {
        use crate::palette::Palette;

        fn is_copy<T: Copy>() {}
        is_copy::<Theme>();
        for skin in [SkinId::Android, SkinId::Iphone] {
            let original = skin.theme();
            for palette in [
                Palette::GruvboxMaterialDark,
                Palette::OneDarkPro,
                Palette::GruvboxMaterialLight,
            ] {
                let themed = original.with_palette(palette.scheme());
                assert_eq!(themed.label, original.label);
                assert_eq!(themed.show_clock, original.show_clock);
                assert_eq!(themed.search_bar, original.search_bar);
                assert_eq!(themed.home_indicator, original.home_indicator);
                assert_eq!(themed.marks, original.marks);
                assert_eq!(themed.search_label, original.search_label);
                assert_eq!(themed.dock.is_some(), original.dock.is_some());
                assert_eq!(
                    themed.stops.map(|stop| stop.0),
                    original.stops.map(|stop| stop.0)
                );
            }
        }
    }

    #[test]
    fn every_surface_color_comes_from_the_scheme() {
        use crate::palette::Palette;

        for skin in [SkinId::Android, SkinId::Iphone] {
            for palette in [
                Palette::GruvboxMaterialDark,
                Palette::OneDarkPro,
                Palette::GruvboxMaterialLight,
            ] {
                let scheme = palette.scheme();
                let theme = skin.theme().with_palette(scheme);
                for color in [
                    theme.ink,
                    theme.muted,
                    theme.card,
                    theme.chip,
                    theme.selected,
                    theme.pill,
                    theme.pill_ink,
                    theme.nav,
                    theme.nav_ink,
                    theme.folder,
                    theme.shell,
                    theme.prompt,
                ] {
                    assert!(scheme.colors.contains(&color));
                }
                assert_eq!(theme.ink, scheme.colors[0x05]);
                assert_eq!(theme.muted, scheme.colors[0x04]);
                assert_eq!(theme.card, scheme.colors[0x01]);
                assert_eq!(theme.chip, scheme.colors[0x02]);
                assert_eq!(theme.selected, scheme.colors[0x03]);
                assert_eq!(theme.folder, scheme.colors[0x0D]);
                assert_eq!(theme.nav, scheme.colors[0x00]);
                assert_eq!(theme.shell, scheme.colors[0x00]);
                if let Some(dock) = theme.dock {
                    assert_eq!(dock, scheme.colors[0x01]);
                }
            }
        }
    }

    fn luminance(color: Rgb) -> f64 {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    #[test]
    fn pills_have_readable_contrast_in_light_and_dark_palettes() {
        use crate::palette::{Palette, Variant};

        for palette in [
            Palette::GruvboxMaterialDark,
            Palette::OneDarkPro,
            Palette::GruvboxMaterialLight,
        ] {
            for skin in [SkinId::Android, SkinId::Iphone] {
                let scheme = palette.scheme();
                let theme = skin.theme().with_palette(scheme);
                let background = luminance(theme.pill);
                let foreground = luminance(theme.pill_ink);
                let contrast =
                    (background.max(foreground) + 0.05) / (background.min(foreground) + 0.05);
                assert!(contrast >= 4.5, "{}: {contrast}", palette.label());
                match scheme.variant {
                    Variant::Light => assert!(foreground > background),
                    Variant::Dark => assert!(foreground < background),
                }
            }
        }
    }

    #[test]
    fn wallpaper_is_a_restrained_mix_of_background_and_accent() {
        use crate::palette::Palette;

        for palette in [
            Palette::GruvboxMaterialDark,
            Palette::OneDarkPro,
            Palette::GruvboxMaterialLight,
        ] {
            let scheme = palette.scheme();
            let base = scheme.colors[0];
            let accent = scheme.colors[0x0D];
            for skin in [SkinId::Android, SkinId::Iphone] {
                let theme = skin.theme().with_palette(scheme);
                for ((_, r, g, b), weight) in theme.stops.into_iter().zip([12, 10, 7, 4, 0]) {
                    assert_eq!(Rgb::new(r, g, b), mix(base, accent, weight));
                    assert!(r.abs_diff(base.r) <= 31);
                    assert!(g.abs_diff(base.g) <= 31);
                    assert!(b.abs_diff(base.b) <= 31);
                }
                assert_eq!(theme.stops[4], (1000, base.r, base.g, base.b));
            }
        }
    }

    #[test]
    fn large_type_grows_rows_and_large_apps_grow_icons() {
        let area = Rect::new(0, 0, 80, 40);
        let regular = metrics(TypeSize::Regular, AppSize::Regular, area);
        let large_type = metrics(TypeSize::Large, AppSize::Regular, area);
        let large_apps = metrics(TypeSize::Regular, AppSize::Large, area);
        assert!(large_type.row_h > regular.row_h);
        assert!(large_type.nav_h >= regular.nav_h);
        assert!(large_apps.icon_w > regular.icon_w);
    }

    #[test]
    fn skins_cycle_and_return() {
        assert_eq!(step_skin(SkinId::Android, 1), SkinId::Iphone);
        assert_eq!(step_skin(SkinId::Iphone, 1), SkinId::Minimal);
        assert_eq!(step_skin(SkinId::Minimal, 1), SkinId::Android);
        assert_eq!(step_type(TypeSize::Large, 1), TypeSize::Small);
        assert_eq!(step_apps(AppSize::Small, -1), AppSize::Large);
    }
}
