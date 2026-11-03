use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Serialize};

use crate::theme::Rgb;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    Light,
    Dark,
}

/// A portable Base16 scheme. Colors are ordered from base00 through base0F.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scheme {
    pub name: String,
    pub author: String,
    pub variant: Variant,
    pub colors: [Rgb; 16],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonScheme {
    name: String,
    author: String,
    variant: Variant,
    palette: JsonPalette,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
struct JsonPalette {
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    base0A: String,
    base0B: String,
    base0C: String,
    base0D: String,
    base0E: String,
    base0F: String,
}

impl Scheme {
    /// HeliBoard 4.2 SaveThoseColors / Kotlin Pair JSON, not an Android backup.
    pub fn heliboard_theme(&self) -> String {
        let colors: serde_json::Map<String, serde_json::Value> = [
            ("background", 0),
            ("keys", 1),
            ("functional_keys", 2),
            ("spacebar", 1),
            ("text", 5),
            ("hint_text", 5),
            ("suggestion_text", 5),
            ("spacebar_text", 5),
            ("accent", 13),
            ("gesture", 13),
        ]
        .into_iter()
        .map(|(key, index)| {
            let c = self.colors[index];
            let argb = i32::from_be_bytes([255, c.r, c.g, c.b]);
            (
                key.to_owned(),
                serde_json::json!({"first":argb,"second":false}),
            )
        })
        .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "name":format!("Mobux {}",self.name),"moreColors":0,"colors":colors
        }))
        .expect("serializing a JSON value")
    }

    /// Termux's standard 16 ANSI slots, plus foreground, background and cursor.
    pub fn termux_colors(&self) -> String {
        use std::fmt::Write as _;
        let mut text = String::new();
        let mut line = |key: &str, index: usize| {
            let c = self.colors[index];
            writeln!(text, "{key}=#{:02x}{:02x}{:02x}", c.r, c.g, c.b).expect("writing to String");
        };
        line("background", 0);
        line("foreground", 5);
        line("cursor", 5);
        for (slot, index) in [0, 8, 11, 10, 13, 14, 12, 5, 3, 8, 11, 10, 13, 14, 12, 7]
            .into_iter()
            .enumerate()
        {
            line(&format!("color{slot}"), index);
        }
        text
    }

    pub fn parse_json(json: &str) -> Result<Self, String> {
        let raw: JsonScheme =
            serde_json::from_str(json).map_err(|error| format!("Invalid theme JSON: {error}"))?;
        if raw.name.trim().is_empty() {
            return Err("Theme name must not be empty".into());
        }
        let palette = raw.palette;
        let values = [
            palette.base00,
            palette.base01,
            palette.base02,
            palette.base03,
            palette.base04,
            palette.base05,
            palette.base06,
            palette.base07,
            palette.base08,
            palette.base09,
            palette.base0A,
            palette.base0B,
            palette.base0C,
            palette.base0D,
            palette.base0E,
            palette.base0F,
        ];
        let mut colors = [Rgb::new(0, 0, 0); 16];
        for (index, value) in values.iter().enumerate() {
            let hex = value.strip_prefix('#').unwrap_or(value);
            if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(format!(
                    "Invalid base{index:02X} color {value:?}: expected six hexadecimal digits, optionally prefixed with #"
                ));
            }
            let number = u32::from_str_radix(hex, 16)
                .map_err(|error| format!("Invalid base{index:02X} color: {error}"))?;
            colors[index] = Rgb::new(
                ((number >> 16) & 0xff) as u8,
                ((number >> 8) & 0xff) as u8,
                (number & 0xff) as u8,
            );
        }
        Ok(Self {
            name: raw.name,
            author: raw.author,
            variant: raw.variant,
            colors,
        })
    }
}

static GRUVBOX_MATERIAL_DARK: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/gruvbox-material-dark.json"))
        .expect("bundled Gruvbox Material Dark scheme must be valid")
});
static ONE_DARK_PRO: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/one-dark-pro.json"))
        .expect("bundled One Dark Pro scheme must be valid")
});
static GRUVBOX_MATERIAL_LIGHT: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/gruvbox-material-light.json"))
        .expect("bundled Gruvbox Material Light scheme must be valid")
});

static CATPPUCCIN_MOCHA: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/catppuccin-mocha.json"))
        .expect("bundled Catppuccin Mocha scheme must be valid")
});
static CATPPUCCIN_LATTE: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/catppuccin-latte.json"))
        .expect("bundled Catppuccin Latte scheme must be valid")
});
static TOKYO_NIGHT_DARK: LazyLock<Scheme> = LazyLock::new(|| {
    Scheme::parse_json(include_str!("../themes/tokyo-night-dark.json"))
        .expect("bundled Tokyo Night Dark scheme must be valid")
});

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Palette {
    #[default]
    GruvboxMaterialDark,
    OneDarkPro,
    GruvboxMaterialLight,
    CatppuccinMocha,
    CatppuccinLatte,
    TokyoNightDark,
    Custom(Arc<Scheme>),
}

impl Palette {
    pub fn presets() -> [Self; 6] {
        [
            Self::GruvboxMaterialDark,
            Self::OneDarkPro,
            Self::GruvboxMaterialLight,
            Self::CatppuccinMocha,
            Self::CatppuccinLatte,
            Self::TokyoNightDark,
        ]
    }
    pub fn label(&self) -> &str {
        match self {
            Self::GruvboxMaterialDark => "Gruvbox Material Dark",
            Self::OneDarkPro => "One Dark Pro",
            Self::GruvboxMaterialLight => "Gruvbox Material Light",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::TokyoNightDark => "Tokyo Night Dark",
            Self::Custom(scheme) => &scheme.name,
        }
    }

    pub fn scheme(&self) -> &Scheme {
        match self {
            Self::GruvboxMaterialDark => &GRUVBOX_MATERIAL_DARK,
            Self::OneDarkPro => &ONE_DARK_PRO,
            Self::GruvboxMaterialLight => &GRUVBOX_MATERIAL_LIGHT,
            Self::CatppuccinMocha => &CATPPUCCIN_MOCHA,
            Self::CatppuccinLatte => &CATPPUCCIN_LATTE,
            Self::TokyoNightDark => &TOKYO_NIGHT_DARK,
            Self::Custom(scheme) => scheme,
        }
    }

    /// Move one preset in either direction. Custom schemes enter at either end.
    pub fn step(&mut self, dir: isize) {
        if dir == 0 {
            return;
        }
        *self = match (&*self, dir > 0) {
            (Self::GruvboxMaterialDark, true) => Self::OneDarkPro,
            (Self::OneDarkPro, true) => Self::GruvboxMaterialLight,
            (Self::GruvboxMaterialLight, true) => Self::CatppuccinMocha,
            (Self::CatppuccinMocha, true) => Self::CatppuccinLatte,
            (Self::CatppuccinLatte, true) => Self::TokyoNightDark,
            (Self::TokyoNightDark | Self::Custom(_), true) => Self::GruvboxMaterialDark,
            (Self::GruvboxMaterialDark | Self::Custom(_), false) => Self::TokyoNightDark,
            (Self::OneDarkPro, false) => Self::GruvboxMaterialDark,
            (Self::GruvboxMaterialLight, false) => Self::OneDarkPro,
            (Self::CatppuccinMocha, false) => Self::GruvboxMaterialLight,
            (Self::CatppuccinLatte, false) => Self::CatppuccinMocha,
            (Self::TokyoNightDark, false) => Self::CatppuccinLatte,
        };
    }

    pub fn from_name_or_file(name: &str) -> Result<Self, String> {
        match name {
            "gruvbox-material-dark" => Ok(Self::GruvboxMaterialDark),
            "one-dark-pro" => Ok(Self::OneDarkPro),
            "gruvbox-material-light" => Ok(Self::GruvboxMaterialLight),
            "catppuccin-mocha" => Ok(Self::CatppuccinMocha),
            "catppuccin-latte" => Ok(Self::CatppuccinLatte),
            "tokyo-night-dark" => Ok(Self::TokyoNightDark),
            _ => {
                let json = std::fs::read_to_string(name).map_err(|error| {
                    format!(
                        "Cannot read theme {name:?}: {error}. Use gruvbox-material-dark, one-dark-pro, gruvbox-material-light, catppuccin-mocha, catppuccin-latte, tokyo-night-dark, or a JSON file path"
                    )
                })?;
                let scheme = Scheme::parse_json(&json)
                    .map_err(|error| format!("Cannot load theme {name:?}: {error}"))?;
                Ok(Self::Custom(Arc::new(scheme)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: &str = include_str!("../themes/gruvbox-material-dark.json");

    #[test]
    fn parses_all_sixteen_colors_and_optional_hash() {
        let scheme = Scheme::parse_json(DARK).unwrap();
        let hashed = DARK.replace("292828", "#A0b1C2");
        let hashed = Scheme::parse_json(&hashed).unwrap();
        assert_eq!(scheme.colors.len(), 16);
        assert_eq!(scheme.colors[0], Rgb::new(0x29, 0x28, 0x28));
        assert_eq!(scheme.colors[15], Rgb::new(0xbd, 0x6f, 0x3e));
        assert_eq!(hashed.colors[0], Rgb::new(0xa0, 0xb1, 0xc2));
        assert_eq!(scheme.variant, Variant::Dark);
        assert!(!scheme.name.is_empty());
        assert!(!scheme.author.is_empty());
    }

    #[test]
    fn rejects_invalid_hex_without_panicking() {
        for value in [
            "", "123", "12345678", "gg0000", "##123456", "é1234", " 123456",
        ] {
            let json = DARK.replace("292828", value);
            let error = Scheme::parse_json(&json).unwrap_err();
            assert!(error.contains("base00"), "{error}");
        }
    }

    #[test]
    fn rejects_missing_unknown_duplicate_and_wrongly_typed_fields() {
        for (json, expected) in [
            (
                DARK.replace("\"base0F\": \"bd6f3e\"", "\"base10\": \"bd6f3e\""),
                "unknown field",
            ),
            (
                DARK.replace("\"base0F\": \"bd6f3e\"", "\"base0E\": \"bd6f3e\""),
                "duplicate field",
            ),
            (
                DARK.replace(",\n    \"base0F\": \"bd6f3e\"", ""),
                "missing field",
            ),
            (DARK.replace("\"292828\"", "42"), "string"),
            (
                DARK.replace("\"name\":", "\"unexpected\":"),
                "unknown field",
            ),
        ] {
            let error = Scheme::parse_json(&json).unwrap_err();
            assert!(error.contains(expected), "{error}");
        }
        assert!(Scheme::parse_json("{}").is_err());
        assert!(Scheme::parse_json(&format!("{DARK} trailing")).is_err());
    }

    #[test]
    fn validates_name_and_variant() {
        let mut json: serde_json::Value = serde_json::from_str(DARK).unwrap();
        for name in ["", " \t\n"] {
            json["name"] = name.into();
            assert!(Scheme::parse_json(&json.to_string())
                .unwrap_err()
                .contains("name"));
        }
        json["name"] = "Custom theme".into();
        for variant in ["", "Dark", "night"] {
            json["variant"] = variant.into();
            let error = Scheme::parse_json(&json.to_string()).unwrap_err();
            assert!(error.contains("light") && error.contains("dark"));
        }
        json["variant"] = "light".into();
        assert_eq!(
            Scheme::parse_json(&json.to_string()).unwrap().variant,
            Variant::Light
        );
    }

    #[test]
    fn presets_have_names_variants_and_cached_schemes() {
        for (name, preset, variant) in [
            (
                "gruvbox-material-dark",
                Palette::GruvboxMaterialDark,
                Variant::Dark,
            ),
            ("one-dark-pro", Palette::OneDarkPro, Variant::Dark),
            (
                "gruvbox-material-light",
                Palette::GruvboxMaterialLight,
                Variant::Light,
            ),
            ("catppuccin-mocha", Palette::CatppuccinMocha, Variant::Dark),
            ("catppuccin-latte", Palette::CatppuccinLatte, Variant::Light),
            ("tokyo-night-dark", Palette::TokyoNightDark, Variant::Dark),
        ] {
            assert_eq!(Palette::from_name_or_file(name).unwrap(), preset);
            assert_eq!(preset.scheme().variant, variant);
            assert!(!preset.label().is_empty());
            assert!(std::ptr::eq(preset.scheme(), preset.clone().scheme()));
        }
        assert_eq!(Palette::default(), Palette::GruvboxMaterialDark);
        assert!(Palette::from_name_or_file("missing-theme-that-does-not-exist.json").is_err());
    }

    #[test]
    fn theme_exports_have_complete_slots_and_signed_opaque_heliboard_colors() {
        for preset in Palette::presets() {
            let scheme = preset.scheme();
            let termux = scheme.termux_colors();
            assert_eq!(termux.lines().count(), 19);
            assert!(termux.lines().all(|line| line
                .split_once("=#")
                .is_some_and(
                    |(_, hex)| hex.len() == 6 && hex.bytes().all(|c| c.is_ascii_hexdigit())
                )));
            let json: serde_json::Value = serde_json::from_str(&scheme.heliboard_theme()).unwrap();
            assert_eq!(json["moreColors"], 0);
            let colors = json["colors"].as_object().unwrap();
            assert_eq!(colors.len(), 10);
            for value in colors.values() {
                assert_eq!(value["second"], false);
                let color = i32::try_from(value["first"].as_i64().unwrap()).unwrap();
                assert_eq!(color.to_be_bytes()[0], 255);
            }
            let c = scheme.colors[0];
            assert_eq!(
                colors["background"]["first"],
                i32::from_be_bytes([255, c.r, c.g, c.b])
            );
        }
    }

    #[test]
    fn loads_custom_file_and_shares_scheme_on_clone() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("themes/gruvbox-material-light.json");
        let palette = Palette::from_name_or_file(path.to_str().unwrap()).unwrap();
        let Palette::Custom(scheme) = &palette else {
            panic!("a file must produce a custom palette");
        };
        assert_eq!(palette.label(), scheme.name);
        assert_eq!(scheme.variant, Variant::Light);
        let Palette::Custom(cloned) = palette.clone() else {
            panic!("cloning must preserve a custom palette");
        };
        assert!(Arc::ptr_eq(scheme, &cloned));
    }

    #[test]
    fn cycles_presets_in_both_directions_and_exits_custom() {
        let mut palette = Palette::default();
        for expected in [
            Palette::OneDarkPro,
            Palette::GruvboxMaterialLight,
            Palette::CatppuccinMocha,
            Palette::CatppuccinLatte,
            Palette::TokyoNightDark,
            Palette::GruvboxMaterialDark,
        ] {
            palette.step(1);
            assert_eq!(palette, expected);
        }
        for expected in [
            Palette::TokyoNightDark,
            Palette::CatppuccinLatte,
            Palette::CatppuccinMocha,
            Palette::GruvboxMaterialLight,
            Palette::OneDarkPro,
            Palette::GruvboxMaterialDark,
        ] {
            palette.step(-1);
            assert_eq!(palette, expected);
        }
        let custom = Palette::Custom(Arc::new(Scheme::parse_json(DARK).unwrap()));
        palette = custom.clone();
        palette.step(0);
        assert_eq!(palette, custom);
        palette.step(1);
        assert_eq!(palette, Palette::GruvboxMaterialDark);
        palette = custom;
        palette.step(-1);
        assert_eq!(palette, Palette::TokyoNightDark);
    }

    #[test]
    fn new_presets_preserve_upstream_names_authors_and_all_base16_roles() {
        for (preset, name, author, colors) in [
            (
                Palette::CatppuccinMocha,
                "Catppuccin Mocha",
                "https://github.com/catppuccin/catppuccin",
                [
                    0x1e1e2e, 0x181825, 0x313244, 0x45475a, 0x585b70, 0xcdd6f4, 0xf5e0dc, 0xb4befe,
                    0xf38ba8, 0xfab387, 0xf9e2af, 0xa6e3a1, 0x94e2d5, 0x89b4fa, 0xcba6f7, 0xf2cdcd,
                ],
            ),
            (
                Palette::CatppuccinLatte,
                "Catppuccin Latte",
                "https://github.com/catppuccin/catppuccin",
                [
                    0xeff1f5, 0xe6e9ef, 0xccd0da, 0xbcc0cc, 0xacb0be, 0x4c4f69, 0xdc8a78, 0x7287fd,
                    0xd20f39, 0xfe640b, 0xdf8e1d, 0x40a02b, 0x179299, 0x1e66f5, 0x8839ef, 0xdd7878,
                ],
            ),
            (
                Palette::TokyoNightDark,
                "Tokyo Night Dark",
                "Michaël Ball",
                [
                    0x1a1b26, 0x16161e, 0x2f3549, 0x444b6a, 0x787c99, 0xa9b1d6, 0xcbccd1, 0xd5d6db,
                    0xc0caf5, 0xa9b1d6, 0x0db9d7, 0x9ece6a, 0xb4f9f8, 0x2ac3de, 0xbb9af7, 0xf7768e,
                ],
            ),
        ] {
            let scheme = preset.scheme();
            assert_eq!(preset.label(), name);
            assert_eq!(scheme.name, name);
            assert_eq!(scheme.author, author);
            let expected = colors
                .map(|color: u32| Rgb::new((color >> 16) as u8, (color >> 8) as u8, color as u8));
            assert_eq!(scheme.colors, expected, "{name}");
        }
    }

    #[test]
    fn stepping_uses_direction_only_and_zero_preserves_each_preset() {
        let mut palette = Palette::default();
        for _ in 0..6 {
            let original = palette.clone();
            palette.step(0);
            assert_eq!(palette, original);
            palette.step(isize::MAX);
            palette.step(isize::MIN);
            assert_eq!(palette, original);
            palette.step(1);
        }
        assert_eq!(palette, Palette::default());
    }
}
