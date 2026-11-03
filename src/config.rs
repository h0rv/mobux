use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::palette::Palette;
use crate::theme::{AppSize, SkinId, SurfaceStyle, TypeSize};

const VERSION: u8 = 1;
const HISTORY_LIMIT: usize = 8;
const MAX_BYTES: usize = 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorChoice {
    #[default]
    Auto,
    Micro,
    Nano,
    Neovim,
    Helix,
}

impl EditorChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Micro => "micro",
            Self::Nano => "nano",
            Self::Neovim => "Neovim",
            Self::Helix => "Helix",
        }
    }

    pub fn command(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Micro => Some("micro"),
            Self::Nano => Some("nano"),
            Self::Neovim => Some("nvim"),
            Self::Helix => Some("hx"),
        }
    }

    /// Move one choice in the requested direction, wrapping at either end.
    pub fn step(&mut self, dir: isize) {
        if dir == 0 {
            return;
        }
        *self = match (*self, dir > 0) {
            (Self::Auto, true) => Self::Micro,
            (Self::Micro, true) => Self::Nano,
            (Self::Nano, true) => Self::Neovim,
            (Self::Neovim, true) => Self::Helix,
            (Self::Helix, true) => Self::Auto,
            (Self::Auto, false) => Self::Helix,
            (Self::Micro, false) => Self::Auto,
            (Self::Nano, false) => Self::Micro,
            (Self::Neovim, false) => Self::Nano,
            (Self::Helix, false) => Self::Neovim,
        };
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub skin: SkinId,
    pub type_size: TypeSize,
    pub app_size: AppSize,
    pub surface_style: SurfaceStyle,
    pub wallpaper: bool,
    pub palette: Palette,
    pub editor: EditorChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            skin: SkinId::default(),
            type_size: TypeSize::default(),
            app_size: AppSize::default(),
            surface_style: SurfaceStyle::default(),
            wallpaper: true,
            palette: Palette::default(),
            editor: EditorChoice::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedLaunch {
    pub title: String,
    pub bin: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u8,
    pub settings: Settings,
    pub history: Vec<SavedLaunch>,
    #[serde(default)]
    pub apps: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: VERSION,
            settings: Settings::default(),
            history: Vec::new(),
            apps: Vec::new(),
        }
    }
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err(format!(
                "Unsupported config version {} (expected {VERSION})",
                self.version
            ));
        }
        if self.apps.len() > 64
            || self.apps.iter().any(|id| {
                id.is_empty()
                    || id.len() > 64
                    || !id.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            })
            || self
                .apps
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.apps.len()
        {
            return Err("Invalid registered app IDs".into());
        }
        if self.history.len() > HISTORY_LIMIT {
            return Err(format!(
                "Launch history must contain at most {HISTORY_LIMIT} entries"
            ));
        }
        if let Palette::Custom(scheme) = &self.settings.palette {
            if scheme.name.trim().is_empty() {
                return Err("Custom theme name must not be empty".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn discover() -> Result<Self, String> {
        discover_path(
            env::var_os("MOBUX_CONFIG_DIR").map(PathBuf::from),
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("HOME").map(PathBuf::from),
        )
        .map(Self::at)
    }

    /// Use an exact file path without consulting the environment.
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Config, String> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(error) => return Err(self.error("read", error)),
        };
        // Read one extra byte to distinguish an exact-size file from an oversized one.
        let mut bytes = Vec::new();
        file.take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| self.error("read", error))?;
        if bytes.len() > MAX_BYTES {
            return Err(self.error("read", "Config exceeds the 1 MiB size limit"));
        }
        let config: Config =
            serde_json::from_slice(&bytes).map_err(|error| self.error("parse", error))?;
        config
            .validate()
            .map_err(|error| self.error("load", error))?;
        Ok(config)
    }

    pub fn save(&self, config: &Config) -> Result<(), String> {
        config
            .validate()
            .map_err(|error| self.error("save", error))?;
        let bytes =
            serde_json::to_vec_pretty(config).map_err(|error| self.error("serialize", error))?;
        if bytes.len() > MAX_BYTES {
            return Err(self.error("save", "Config exceeds the 1 MiB size limit"));
        }
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(parent)
            .map_err(|error| self.error("create directory for", error))?;

        let (mut file, mut pending) = self.create_temp(parent)?;
        file.write_all(&bytes)
            .map_err(|error| self.error("write", error))?;
        file.sync_all().map_err(|error| self.error("sync", error))?;
        drop(file);
        fs::rename(&pending.path, &self.path).map_err(|error| self.error("replace", error))?;
        pending.committed = true;
        Ok(())
    }

    fn create_temp(&self, parent: &Path) -> Result<(File, PendingWrite), String> {
        for _ in 0..128 {
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".mobux-state-{}-{sequence}.tmp",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => {
                    return Ok((
                        file,
                        PendingWrite {
                            path,
                            committed: false,
                        },
                    ))
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(self.error("create temporary file for", error)),
            }
        }
        Err(self.error("save", "Could not reserve a unique temporary file"))
    }

    fn error(&self, action: &str, error: impl std::fmt::Display) -> String {
        format!("Cannot {action} config {}: {error}", self.path.display())
    }
}

fn discover_path(
    override_dir: Option<PathBuf>,
    xdg: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(dir) = override_dir {
        if !dir.is_absolute() {
            return Err(format!(
                "MOBUX_CONFIG_DIR must be absolute: {}",
                dir.display()
            ));
        }
        return Ok(dir.join("state.json"));
    }
    let base = match xdg.filter(|path| path.is_absolute()) {
        Some(path) => path,
        None => home.filter(|path| path.is_absolute())
            .ok_or_else(|| "Cannot locate mobux/state.json: set an absolute MOBUX_CONFIG_DIR, XDG_CONFIG_HOME, or HOME".to_string())?
            .join(".config"),
    };
    Ok(base.join("mobux/state.json"))
}

struct PendingWrite {
    path: PathBuf,
    committed: bool,
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{json, Value};

    use super::*;
    use crate::palette::{Scheme, Variant};
    use crate::theme::Rgb;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            loop {
                let path = env::temp_dir().join(format!(
                    "mobux-config-test-{}-{}",
                    std::process::id(),
                    NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create test directory: {error}"),
                }
            }
        }

        fn store(&self) -> Store {
            Store::at(self.0.join("mobux/state.json"))
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn custom_palette() -> Palette {
        Palette::Custom(Arc::new(Scheme {
            name: "A custom theme".into(),
            author: "Test author".into(),
            variant: Variant::Light,
            colors: [Rgb::new(12, 34, 56); 16],
        }))
    }

    fn launch() -> SavedLaunch {
        SavedLaunch {
            title: "An editor with spaces".into(),
            bin: "my editor".into(),
            args: vec![
                "file with spaces.rs".into(),
                "--flag=two words".into(),
                "".into(),
            ],
            cwd: PathBuf::from("/a directory/with spaces"),
        }
    }

    fn write_json(store: &Store, value: &Value) {
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), serde_json::to_vec(value).unwrap()).unwrap();
    }

    #[test]
    fn app_registration_roundtrips_and_rejects_duplicate_or_invalid_ids() {
        let mut config = Config {
            apps: vec!["herdr".into(), "github".into()],
            ..Config::default()
        };
        config.validate().unwrap();
        let decoded: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(decoded, config);
        config.apps.push("herdr".into());
        assert!(config.validate().is_err());
        config.apps = vec!["not an id".into()];
        assert!(config.validate().is_err());
        assert!(Config::default().apps.is_empty());
    }

    #[test]
    fn defaults_match_current_preferences() {
        let settings = Settings::default();
        assert_eq!(settings.skin, SkinId::Minimal);
        assert_eq!(settings.type_size, TypeSize::Regular);
        assert_eq!(settings.app_size, AppSize::Regular);
        assert_eq!(settings.surface_style, SurfaceStyle::Rounded);
        assert!(settings.wallpaper);
        assert_eq!(settings.palette, Palette::GruvboxMaterialDark);
        assert_eq!(settings.editor, EditorChoice::Auto);
        assert_eq!(Config::default().version, 1);
    }

    #[test]
    fn editor_labels_commands_and_steps() {
        let mut editor = EditorChoice::default();
        for (choice, label, command) in [
            (EditorChoice::Auto, "Auto", None),
            (EditorChoice::Micro, "micro", Some("micro")),
            (EditorChoice::Nano, "nano", Some("nano")),
            (EditorChoice::Neovim, "Neovim", Some("nvim")),
            (EditorChoice::Helix, "Helix", Some("hx")),
        ] {
            assert_eq!(editor, choice);
            assert_eq!(editor.label(), label);
            assert_eq!(editor.command(), command);
            editor.step(0);
            assert_eq!(editor, choice);
            editor.step(isize::MAX);
        }
        assert_eq!(editor, EditorChoice::Auto);
        editor.step(isize::MIN);
        assert_eq!(editor, EditorChoice::Helix);
        for expected in [
            EditorChoice::Neovim,
            EditorChoice::Nano,
            EditorChoice::Micro,
            EditorChoice::Auto,
        ] {
            editor.step(-1);
            assert_eq!(editor, expected);
        }
    }

    #[test]
    fn roundtrips_every_preference_custom_arc_and_history() {
        let dir = TestDir::new();
        let store = dir.store();
        let mut config = Config {
            history: vec![launch(); HISTORY_LIMIT],
            ..Config::default()
        };
        for skin in [SkinId::Android, SkinId::Iphone] {
            for type_size in [TypeSize::Small, TypeSize::Regular, TypeSize::Large] {
                for app_size in [AppSize::Small, AppSize::Regular, AppSize::Large] {
                    for surface_style in [SurfaceStyle::Rounded, SurfaceStyle::Blocky] {
                        for wallpaper in [false, true] {
                            for palette in [
                                Palette::GruvboxMaterialDark,
                                Palette::OneDarkPro,
                                Palette::GruvboxMaterialLight,
                                custom_palette(),
                            ] {
                                for editor in [
                                    EditorChoice::Auto,
                                    EditorChoice::Micro,
                                    EditorChoice::Nano,
                                    EditorChoice::Neovim,
                                    EditorChoice::Helix,
                                ] {
                                    config.settings = Settings {
                                        skin,
                                        type_size,
                                        app_size,
                                        surface_style,
                                        wallpaper,
                                        palette: palette.clone(),
                                        editor,
                                    };
                                    let bytes = serde_json::to_vec(&config).unwrap();
                                    assert_eq!(
                                        serde_json::from_slice::<Config>(&bytes).unwrap(),
                                        config
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        store.save(&config).unwrap();
        assert_eq!(store.load().unwrap(), config);
        assert_eq!(
            fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn missing_file_is_default_and_partial_settings_use_defaults() {
        let dir = TestDir::new();
        let store = dir.store();
        assert_eq!(store.load().unwrap(), Config::default());
        assert!(!store.path().exists());
        write_json(
            &store,
            &json!({"version": 1, "settings": {"skin": "iphone"}, "history": []}),
        );
        assert_eq!(
            store.load().unwrap().settings,
            Settings {
                skin: SkinId::Iphone,
                ..Settings::default()
            }
        );
        write_json(
            &store,
            &json!({"version": 1, "settings": {}, "history": []}),
        );
        assert_eq!(store.load().unwrap(), Config::default());
    }

    #[test]
    fn malformed_and_unsupported_configs_report_path_without_changing_file() {
        let dir = TestDir::new();
        let store = dir.store();
        let mut unsupported = serde_json::to_value(Config::default()).unwrap();
        unsupported["version"] = json!(2);
        for (bytes, expected) in [
            (
                serde_json::to_vec(&unsupported).unwrap(),
                "Unsupported config version 2",
            ),
            (b"{ broken JSON".to_vec(), "parse"),
            (vec![0xff], "parse"),
            (b"".to_vec(), "parse"),
        ] {
            fs::create_dir_all(store.path().parent().unwrap()).unwrap();
            fs::write(store.path(), &bytes).unwrap();
            let error = store.load().unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert!(
                error.contains(&store.path().display().to_string()),
                "{error}"
            );
            assert_eq!(fs::read(store.path()).unwrap(), bytes);
        }
    }

    #[test]
    fn rejects_invalid_enums_types_and_unknown_fields() {
        let dir = TestDir::new();
        let store = dir.store();
        for field in [
            "skin",
            "type_size",
            "app_size",
            "surface_style",
            "palette",
            "editor",
        ] {
            let mut value = serde_json::to_value(Config::default()).unwrap();
            value["settings"][field] = json!("not-a-valid-value");
            write_json(&store, &value);
            assert!(store.load().unwrap_err().contains("unknown variant"));
        }
        for (field, invalid) in [
            ("wallpaper", json!("true")),
            ("editor", Value::Null),
            ("app_size", json!(42)),
        ] {
            let mut value = serde_json::to_value(Config::default()).unwrap();
            value["settings"][field] = invalid;
            write_json(&store, &value);
            assert!(store.load().is_err());
        }
        for pointer in [
            "",
            "/settings",
            "/history/0",
            "/settings/palette/custom",
            "/settings/palette/custom/colors/0",
        ] {
            let mut config = Config {
                history: vec![launch()],
                ..Config::default()
            };
            config.settings.palette = custom_palette();
            let mut value = serde_json::to_value(config).unwrap();
            value.pointer_mut(pointer).unwrap()["unexpected"] = json!(true);
            write_json(&store, &value);
            assert!(store.load().unwrap_err().contains("unknown field"));
        }
    }

    #[test]
    fn validates_history_custom_name_and_typed_colors() {
        let dir = TestDir::new();
        let store = dir.store();
        let mut config = Config {
            history: vec![launch(); HISTORY_LIMIT + 1],
            ..Config::default()
        };
        write_json(&store, &serde_json::to_value(&config).unwrap());
        assert!(store.load().unwrap_err().contains("at most 8"));
        assert!(store.save(&config).unwrap_err().contains("at most 8"));
        config.history.clear();
        config.settings.palette = custom_palette();
        for name in ["", " \t\n"] {
            let mut value = serde_json::to_value(&config).unwrap();
            value["settings"]["palette"]["custom"]["name"] = json!(name);
            write_json(&store, &value);
            assert!(store.load().unwrap_err().contains("name must not be empty"));
        }
        for colors in [
            json!([]),
            json!(vec![json!({"r": 256, "g": 0, "b": 0}); 16]),
        ] {
            let mut value = serde_json::to_value(&config).unwrap();
            value["settings"]["palette"]["custom"]["colors"] = colors;
            write_json(&store, &value);
            assert!(store.load().is_err());
        }
    }

    #[test]
    fn enforces_read_and_write_size_bounds() {
        let dir = TestDir::new();
        let store = dir.store();
        store.save(&Config::default()).unwrap();
        let original = fs::read(store.path()).unwrap();
        let mut config = Config {
            history: vec![launch()],
            ..Config::default()
        };
        config.history[0].title = "x".repeat(MAX_BYTES);
        assert!(store.save(&config).unwrap_err().contains("1 MiB"));
        assert_eq!(fs::read(store.path()).unwrap(), original);
        let mut exact = serde_json::to_vec(&Config::default()).unwrap();
        exact.resize(MAX_BYTES, b' ');
        fs::write(store.path(), &exact).unwrap();
        assert_eq!(store.load().unwrap(), Config::default());
        exact.push(b' ');
        fs::write(store.path(), &exact).unwrap();
        assert!(store.load().unwrap_err().contains("1 MiB"));
        assert_eq!(
            fs::metadata(store.path()).unwrap().len(),
            (MAX_BYTES + 1) as u64
        );
    }

    #[test]
    fn atomically_replaces_without_truncating_existing_inode_or_leaving_temps() {
        let dir = TestDir::new();
        let store = dir.store();
        store.save(&Config::default()).unwrap();
        let mut old_file = File::open(store.path()).unwrap();
        let mut config = Config::default();
        config.settings.wallpaper = false;
        store.save(&config).unwrap();
        let mut previous = Vec::new();
        old_file.read_to_end(&mut previous).unwrap();
        assert_eq!(
            serde_json::from_slice::<Config>(&previous).unwrap(),
            Config::default()
        );
        assert_eq!(store.load().unwrap(), config);
        assert_eq!(
            fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn failed_rename_cleans_temporary_file_and_preserves_destination() {
        let dir = TestDir::new();
        let store = dir.store();
        fs::create_dir_all(store.path()).unwrap();
        fs::write(store.path().join("keep"), "untouched").unwrap();
        assert!(store.save(&Config::default()).is_err());
        assert_eq!(fs::read(store.path().join("keep")).unwrap(), b"untouched");
        assert_eq!(
            fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn invalid_save_preserves_existing_file_and_creates_no_temps() {
        let dir = TestDir::new();
        let store = dir.store();
        store.save(&Config::default()).unwrap();
        let bytes = fs::read(store.path()).unwrap();
        let config = Config {
            version: 2,
            ..Config::default()
        };
        assert!(store.save(&config).unwrap_err().contains("version"));
        assert_eq!(fs::read(store.path()).unwrap(), bytes);
        assert_eq!(
            fs::read_dir(store.path().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn creates_private_files_and_directories_without_changing_existing_permissions() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let dir = TestDir::new();
        fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).unwrap();
        let store = Store::at(dir.0.join("new/nested/state.json"));
        store.save(&Config::default()).unwrap();
        assert_eq!(fs::metadata(store.path()).unwrap().mode() & 0o777, 0o600);
        for path in [dir.0.join("new"), dir.0.join("new/nested")] {
            assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o700);
        }
        assert_eq!(fs::metadata(&dir.0).unwrap().mode() & 0o777, 0o755);
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o644)).unwrap();
        store.save(&Config::default()).unwrap();
        assert_eq!(fs::metadata(store.path()).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn discovery_respects_override_xdg_and_home_without_mutating_environment() {
        let home = Some(PathBuf::from("/home/test"));
        assert_eq!(
            discover_path(None, None, home.clone()).unwrap(),
            PathBuf::from("/home/test/.config/mobux/state.json")
        );
        assert_eq!(
            discover_path(None, Some("relative".into()), home.clone()).unwrap(),
            PathBuf::from("/home/test/.config/mobux/state.json")
        );
        assert_eq!(
            discover_path(None, Some("/xdg".into()), None).unwrap(),
            PathBuf::from("/xdg/mobux/state.json")
        );
        assert_eq!(
            discover_path(Some("/override".into()), Some("/xdg".into()), home).unwrap(),
            PathBuf::from("/override/state.json")
        );
        assert!(discover_path(Some("relative".into()), None, None)
            .unwrap_err()
            .contains("MOBUX_CONFIG_DIR"));
        assert!(discover_path(None, None, None).is_err());
        assert!(discover_path(None, None, Some("relative".into())).is_err());
    }
}
