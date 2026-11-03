use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ratatui::layout::Rect;

use crate::config::{Config, EditorChoice, SavedLaunch, Settings};
use crate::find::{self, FindMode, Hit, Indexed};
use crate::palette::Palette;
use crate::sessions::{ManagedSession, SessionId, SessionStatus};
use crate::theme::{self, slot_at, AppSize, Metrics, SkinId, SurfaceStyle, TypeSize};

/// Launcher pages. Home and Shell are the two pages a sideways swipe moves between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Shell,
    Files,
    Store,
    Search,
    Settings,
    Recents,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    ResumeSession {
        id: SessionId,
    },
    RestartSession {
        id: SessionId,
    },
    StopSession {
        id: SessionId,
    },
    Launch {
        title: String,
        bin: String,
        args: Vec<String>,
        cwd: PathBuf,
    },
}

/// One app the switcher can open again. The program itself is not kept running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub title: String,
    pub bin: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Parent,
    Dir,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
}

pub use crate::catalog::{CatalogItem, CATALOG};
pub const SETTINGS_COUNT: usize = 7;

#[derive(Clone, Copy, Debug)]
pub struct HomeIcon {
    pub name: &'static str,
    pub glyph: crate::icons::Glyph,
    pub rgb: (u8, u8, u8),
    pub target: IconTarget,
}

#[derive(Clone, Copy, Debug)]
pub enum IconTarget {
    App(usize),
    Shell,
    Files,
    Store,
    Settings,
}

pub const ICONS: [HomeIcon; 5] = [
    HomeIcon {
        name: "Add app",
        glyph: crate::icons::Glyph::Bag,
        rgb: (52, 168, 83),
        target: IconTarget::App(0),
    },
    HomeIcon {
        name: "Terminal",
        glyph: crate::icons::Glyph::Terminal,
        rgb: (96, 104, 120),
        target: IconTarget::Shell,
    },
    HomeIcon {
        name: "Files",
        glyph: crate::icons::Glyph::Folder,
        rgb: (66, 133, 244),
        target: IconTarget::Files,
    },
    HomeIcon {
        name: "Apps",
        glyph: crate::icons::Glyph::Bag,
        rgb: (52, 168, 83),
        target: IconTarget::Store,
    },
    HomeIcon {
        name: "Settings",
        glyph: crate::icons::Glyph::Settings,
        rgb: (96, 104, 120),
        target: IconTarget::Settings,
    },
];

impl HomeIcon {
    pub fn display_glyph(self) -> crate::icons::Glyph {
        self.glyph
    }

    pub fn display_label(self) -> &'static str {
        self.name
    }
}

/// Rects painted last frame. Clicks use these, so hit testing matches the picture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Places {
    pub icons: [Rect; 5],
    pub search: Rect,
    pub back: Rect,
    pub home: Rect,
    pub recent: Rect,
    pub list: Rect,
    pub files_sheet: Rect,
    pub search_sheet: Rect,
    pub file_rows: Rect,
    pub file_query: Rect,
    pub file_submit: Rect,
    pub file_names: Rect,
    pub file_text: Rect,
    pub file_open: Rect,
    pub file_use: Rect,
    pub project: Rect,
    pub workspace_agent: Rect,
    pub workspace_review: Rect,
    pub workspace_files: Rect,
    pub workspace_terminal: Rect,
    pub workspace_github: Rect,
    pub workspace_apps: Rect,
    pub workspace_live: Rect,
    pub terminal_open: Rect,
    pub notice: Rect,
    pub session_dialog: Rect,
    pub install_cancel: Rect,
    pub install_confirm: Rect,
    pub session_cancel: Rect,
    pub session_stop: Rect,
    pub names_chip: Rect,
    pub text_chip: Rect,
    pub dot_home: Rect,
    pub dot_shell: Rect,
    pub gear: Rect,
    pub settings_sheet: Rect,
}

#[derive(Clone, Debug)]
struct Pointer {
    col: u16,
    row: u16,
    moved: bool,
}

pub struct State {
    pub screen: Screen,
    /// The native pixel background is enabled by the terminal backend, not the palette.
    pub image_icons: bool,
    pub cwd: PathBuf,
    pub files: Vec<Node>,
    pub tree: Option<crate::tree::Tree>,
    pub files_input_focused: bool,
    pub search_job: Option<find::TextJob>,
    pub text_search_ready: bool,
    pub file_cursor: usize,
    pub file_scroll: usize,
    pub store_cursor: usize,
    pub store_scroll: usize,
    pub installed: Vec<bool>,
    pub registered_apps: Vec<String>,
    pub notice: String,
    pub places: Places,
    /// When set, the home screen draws this time and date instead of the clock.
    pub clock_override: Option<(String, String)>,
    pub query: String,
    pub find_mode: FindMode,
    pub name_index: Vec<Indexed>,
    pub hits: Vec<Hit>,
    pub hit_cursor: usize,
    pub hit_scroll: usize,
    pub skin: SkinId,
    pub palette: Palette,
    pub type_size: TypeSize,
    pub app_size: AppSize,
    pub surface_style: SurfaceStyle,
    pub wallpaper: bool,
    pub preferred_editor: EditorChoice,
    pub metrics: Metrics,
    pub sessions: Vec<Session>,
    pub live_sessions: Vec<ManagedSession>,
    pub session_backend: bool,
    pub pending_install: Option<crate::install::Plan>,
    pub pending_stop: Option<SessionId>,
    pub session_cursor: usize,
    pub session_scroll: usize,
    pub settings_cursor: usize,
    pub settings_scroll: usize,
    /// Shown for one frame before the terminal gives the screen to an app.
    pub opening: Option<String>,
    pointer: Option<Pointer>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            screen: Screen::Home,
            image_icons: false,
            cwd,
            files: Vec::new(),
            tree: None,
            files_input_focused: false,
            search_job: None,
            text_search_ready: false,
            file_cursor: 0,
            file_scroll: 0,
            store_cursor: 0,
            store_scroll: 0,
            installed: vec![false; CATALOG.len()],
            registered_apps: Vec::new(),
            notice: String::new(),
            places: Places::default(),
            clock_override: None,
            query: String::new(),
            find_mode: FindMode::Names,
            name_index: Vec::new(),
            hits: Vec::new(),
            hit_cursor: 0,
            hit_scroll: 0,
            skin: SkinId::Minimal,
            palette: Palette::default(),
            type_size: TypeSize::Regular,
            app_size: AppSize::Regular,
            surface_style: SurfaceStyle::Rounded,
            wallpaper: true,
            preferred_editor: EditorChoice::default(),
            metrics: Metrics::default(),
            sessions: Vec::new(),
            live_sessions: Vec::new(),
            session_backend: false,
            pending_install: None,
            pending_stop: None,
            session_cursor: 0,
            session_scroll: 0,
            settings_cursor: 0,
            settings_scroll: 0,
            opening: None,
            pointer: None,
        }
    }

    pub fn restore_config(&mut self, config: Config) {
        self.registered_apps = config.apps;
        let settings = config.settings;
        self.skin = settings.skin;
        self.type_size = settings.type_size;
        self.app_size = settings.app_size;
        self.surface_style = settings.surface_style;
        self.wallpaper = settings.wallpaper;
        self.palette = settings.palette;
        self.preferred_editor = settings.editor;
        self.sessions = config
            .history
            .into_iter()
            .map(|saved| Session {
                title: saved.title,
                bin: saved.bin,
                args: saved.args,
                cwd: saved.cwd,
            })
            .collect();
    }

    pub fn saved_config(&self) -> Config {
        Config {
            apps: self.registered_apps.clone(),
            settings: Settings {
                skin: self.skin,
                type_size: self.type_size,
                app_size: self.app_size,
                surface_style: self.surface_style,
                wallpaper: self.wallpaper,
                palette: self.palette.clone(),
                editor: self.preferred_editor,
            },
            history: self
                .sessions
                .iter()
                .take(8)
                .map(|session| SavedLaunch {
                    title: session.title.clone(),
                    bin: session.bin.clone(),
                    args: session.args.clone(),
                    cwd: session.cwd.clone(),
                })
                .collect(),
            ..Config::default()
        }
    }

    pub fn theme(&self) -> theme::Theme {
        self.skin.theme().with_palette(self.palette.scheme())
    }

    pub fn open_search(&mut self) {
        self.search_job = None;
        self.text_search_ready = false;
        self.screen = Screen::Search;
        self.query.clear();
        self.find_mode = FindMode::Names;
        self.notice.clear();
        self.name_index = find::index_names(&self.cwd);
        self.refresh_hits();
    }

    pub fn set_find_mode(&mut self, mode: FindMode) {
        self.find_mode = mode;
        self.notice.clear();
        self.query_changed();
    }

    pub fn refresh_hits(&mut self) {
        match self.find_mode {
            FindMode::Names => {
                self.hits = find::filter_names(&self.cwd, &self.name_index, &self.query);
            }
            FindMode::Text => {
                self.begin_text_search();
            }
        }
        self.hit_cursor = 0;
        self.hit_scroll = 0;
    }

    pub fn files_filtering(&self) -> bool {
        self.screen == Screen::Search || self.find_mode == FindMode::Text || !self.query.is_empty()
    }
    pub fn file_node(&self, index: usize) -> Option<&Node> {
        self.tree.as_ref().map_or_else(
            || self.files.get(index),
            |tree| tree.rows.get(index).map(|row| &row.node),
        )
    }
    pub fn file_count(&self) -> usize {
        self.tree
            .as_ref()
            .map_or(self.files.len(), |tree| tree.rows.len())
    }
    fn focus_file_filter(&mut self) {
        self.files_input_focused = true;
        if self.name_index.is_empty() {
            self.name_index = find::index_names(&self.cwd);
        }
    }
    fn query_changed(&mut self) {
        self.search_job = None;
        self.text_search_ready = false;
        self.notice.clear();
        self.hit_cursor = 0;
        self.hit_scroll = 0;
        if self.find_mode == FindMode::Names {
            self.hits = find::filter_names(&self.cwd, &self.name_index, &self.query);
        } else {
            self.hits.clear();
        }
    }
    fn begin_text_search(&mut self) {
        if self.search_job.is_some() {
            return;
        }
        if self.query.trim().chars().count() < 2 {
            self.notice = "Enter at least two characters".into();
            return;
        }
        self.hits.clear();
        self.notice.clear();
        self.text_search_ready = false;
        self.search_job = Some(find::TextJob::spawn(self.cwd.clone(), self.query.clone()));
    }
    pub fn poll_search(&mut self) -> bool {
        let Some(result) = self.search_job.as_mut().and_then(|job| job.poll()) else {
            return false;
        };
        self.search_job = None;
        self.text_search_ready = true;
        match result {
            Ok(hits) => self.hits = hits,
            Err(error) => {
                self.notice = error;
                self.hits.clear();
            }
        }
        self.hit_cursor = 0;
        self.hit_scroll = 0;
        true
    }

    pub fn open_files(&mut self) {
        if self.screen != Screen::Files {
            self.query.clear();
            self.find_mode = FindMode::Names;
            self.search_job = None;
            self.hits.clear();
            self.files_input_focused = false;
        }
        self.screen = Screen::Files;
        if self.tree.as_ref().is_none_or(|tree| tree.root != self.cwd) {
            self.reload_files();
        }
    }

    pub fn open_store(&mut self) {
        self.screen = Screen::Store;
        self.refresh_installed();
    }

    pub fn refresh_installed(&mut self) {
        self.installed = CATALOG.iter().map(|item| on_path(&item.bin)).collect();
    }

    pub fn change_directory(&mut self, path: PathBuf) -> Result<(), String> {
        let path = std::path::absolute(&path).map_err(|err| {
            let message = format!("Cannot resolve {}: {err}", path.display());
            self.notice = message.clone();
            message
        })?;
        let nodes = load_dir(&path).map_err(|err| {
            let message = format!("Cannot open {}: {err}", path.display());
            self.notice = message.clone();
            message
        })?;
        let cursor = if self.cwd.parent() == Some(path.as_path()) {
            nodes.iter().position(|node| node.path == self.cwd)
        } else {
            None
        };
        self.tree = Some(crate::tree::Tree::new(path.clone(), &nodes));
        self.query.clear();
        self.search_job = None;
        self.hits.clear();
        self.name_index.clear();
        self.files_input_focused = false;
        self.cwd = path;
        self.files = nodes;
        self.file_cursor = cursor.unwrap_or(0);
        self.file_scroll = 0;
        self.notice.clear();
        Ok(())
    }

    pub fn reload_files(&mut self) {
        match load_dir(&self.cwd) {
            Ok(nodes) => {
                self.tree = Some(crate::tree::Tree::new(self.cwd.clone(), &nodes));
                self.files = nodes;
                if self.file_cursor >= self.files.len() {
                    self.file_cursor = self.files.len().saturating_sub(1);
                }
            }
            Err(err) => {
                self.notice = err.to_string();
                // Keep the last usable tree and selection on refresh failure.
            }
        }
    }

    pub fn go_home(&mut self) {
        self.search_job = None;
        self.files_input_focused = false;
        self.pending_install = None;
        self.pending_stop = None;
        self.screen = Screen::Home;
        self.notice.clear();
    }

    pub fn go_shell(&mut self) {
        self.screen = Screen::Shell;
        self.notice.clear();
    }

    pub fn open_settings(&mut self) {
        self.screen = Screen::Settings;
        self.settings_cursor = 0;
        self.settings_scroll = 0;
        self.notice.clear();
    }

    pub fn open_recents(&mut self) {
        self.pending_stop = None;
        self.screen = Screen::Recents;
        self.session_cursor = 0;
        self.session_scroll = 0;
        self.notice.clear();
    }

    pub fn note_session(&mut self, title: &str, bin: &str, args: &[String], cwd: PathBuf) {
        self.sessions.retain(|session| {
            session.bin != bin || session.args.as_slice() != args || session.cwd != cwd
        });
        self.sessions.insert(
            0,
            Session {
                title: title.to_string(),
                bin: bin.to_string(),
                args: args.to_vec(),
                cwd,
            },
        );
        self.sessions.truncate(8);
        self.session_cursor = 0;
        self.session_scroll = 0;
    }

    fn start(&mut self, title: &str, bin: &str, args: Vec<String>, cwd: PathBuf) -> Action {
        Action::Launch {
            title: title.to_string(),
            bin: bin.to_string(),
            args,
            cwd,
        }
    }

    fn launch_named(&mut self, title: &str, bin: &str) -> Action {
        if let Some(missing) = crate::catalog::by_bin(bin)
            .and_then(|app| app.requires.iter().find(|dependency| !on_path(dependency)))
        {
            return self.offer_missing(missing);
        }
        if on_path(bin) {
            if bin == "gh" {
                return self.launch_github();
            }
            self.notice.clear();
            self.start(
                title,
                bin,
                crate::catalog::by_bin(bin).map_or_else(Vec::new, |app| app.args.clone()),
                self.cwd.clone(),
            )
        } else {
            self.offer_missing(bin)
        }
    }

    fn offer_missing(&mut self, bin: &str) -> Action {
        self.open_store();
        self.notice = format!("{bin} is not installed");
        if let Some(index) = CATALOG.iter().position(|app| app.bin == bin) {
            self.store_cursor = index;
            self.pending_install =
                crate::install::Plan::for_app(&CATALOG[index], crate::install::is_termux());
            if self.pending_install.is_some() {
                self.notice.clear();
            }
        }
        Action::None
    }

    fn activate_icon(&mut self, index: usize) -> Action {
        let Some(icon) = ICONS.get(index) else {
            return Action::None;
        };
        match icon.target {
            IconTarget::App(index) => {
                if let Some(app) = self
                    .registered_apps
                    .get(index)
                    .and_then(|id| CATALOG.iter().find(|app| app.id == *id))
                {
                    self.launch_named(&app.label, &app.bin)
                } else {
                    self.open_store();
                    Action::None
                }
            }
            IconTarget::Shell => self.launch_shell(),
            IconTarget::Files => {
                self.open_files();
                Action::None
            }
            IconTarget::Store => {
                self.open_store();
                Action::None
            }
            IconTarget::Settings => {
                self.open_settings();
                Action::None
            }
        }
    }

    fn back(&mut self) {
        if self.pending_install.take().is_some() {
            return;
        }
        if self.pending_stop.take().is_some() {
            return;
        }
        match self.screen {
            Screen::Files => {
                let parent = self.cwd.parent().map(Path::to_path_buf);
                let parent = parent.filter(|parent| parent != &self.cwd);
                if let Some(parent) = parent {
                    let _ = self.change_directory(parent);
                    return;
                }
                self.go_home();
            }
            Screen::Home => {}
            Screen::Shell | Screen::Store | Screen::Search | Screen::Settings | Screen::Recents => {
                self.go_home()
            }
        }
    }

    fn open_selected_file(&mut self) {
        let Some(node) = self.file_node(self.file_cursor).cloned() else {
            return;
        };
        match node.kind {
            NodeKind::Parent | NodeKind::Dir => {
                let _ = self.change_directory(node.path);
            }
            NodeKind::File => {
                self.notice = node.path.display().to_string();
            }
        }
    }

    fn open_selected_store(&mut self) -> Action {
        let Some(item) = CATALOG.get(self.store_cursor) else {
            return Action::None;
        };
        if !self.registered_apps.contains(&item.id) {
            self.toggle_app(self.store_cursor);
            return Action::None;
        }
        self.launch_named(&item.label, &item.bin)
    }

    pub fn toggle_app(&mut self, index: usize) {
        let Some(item) = CATALOG.get(index) else {
            return;
        };
        if self.registered_apps.contains(&item.id) {
            self.registered_apps.retain(|id| id != &item.id);
            self.notice = format!("Removed {} from workspace", item.label);
        } else if self.registered_apps.len() < 64 {
            self.registered_apps.push(item.id.clone());
            self.notice = format!("Added {}", item.label);
        }
    }

    pub fn primary_app(&self) -> Option<&'static CatalogItem> {
        self.registered_apps
            .iter()
            .find_map(|id| CATALOG.iter().find(|item| item.id == *id))
    }

    fn confirm_install(&mut self) -> Action {
        let Some(plan) = self.pending_install.take() else {
            return Action::None;
        };
        if !crate::install::is_termux() {
            self.notice = "Installation is available on Termux only.".into();
            return Action::None;
        }
        self.start(
            &format!("Install {}", plan.title),
            "pkg",
            plan.args(),
            self.cwd.clone(),
        )
    }

    fn launch_github(&mut self) -> Action {
        if !on_path("gh") {
            return self.launch_named("GitHub CLI", "gh");
        }
        self.notice.clear();
        // Static shell program, never interpolated data. Keep the overview visible
        // in an interactive shell so a one-shot CLI does not immediately disappear.
        self.start("GitHub",&crate::host::posix_shell().to_string_lossy(),vec!["-c".into(),
            "gh status; printf '\\nTry gh pr list, gh issue list, or gh auth login.\\n'; exec \"$1\" -i".into(),
            "mobux-github".into(),user_shell()],self.cwd.clone())
    }

    fn launch_shell(&mut self) -> Action {
        self.notice.clear();
        self.start("Terminal", &user_shell(), Vec::new(), self.cwd.clone())
    }

    pub fn on_char(&mut self, ch: char) -> Action {
        if self.screen == Screen::Files && !self.files_input_focused && ch == '/' {
            self.focus_file_filter();
            return Action::None;
        }
        if self.pending_install.is_some() {
            if ch == 'y' {
                return self.confirm_install();
            }
            if ch == 'n' {
                self.pending_install = None;
            }
            return Action::None;
        }
        if self.screen == Screen::Recents && self.pending_stop.is_some() {
            if ch == 'y' {
                return Action::StopSession {
                    id: self.pending_stop.take().expect("confirmation has an id"),
                };
            }
            if ch == 'n' {
                self.pending_stop = None;
            }
            return Action::None;
        }
        if self.screen == Screen::Search
            || (self.screen == Screen::Files && self.files_input_focused)
        {
            if !ch.is_control() && self.query.len() + ch.len_utf8() <= 512 {
                self.query.push(ch);
                self.query_changed();
            }
            return Action::None;
        }
        if self.screen == Screen::Shell {
            return Action::None;
        }
        if self.screen == Screen::Home {
            match ch {
                '/' => {
                    self.open_search();
                    return Action::None;
                }
                'a' => {
                    self.open_store();
                    return Action::None;
                }
                'g' => return self.launch_github(),
                'r' => {
                    self.open_recents();
                    return Action::None;
                }
                'd' => return self.launch_named("Hunk", "hunk"),
                '1' => return self.activate_icon(0),
                '2' => return self.activate_icon(1),
                '3' => return self.activate_icon(2),
                '4' => return self.activate_icon(3),
                '5' => return self.activate_icon(4),
                't' => {
                    self.go_shell();
                    return Action::None;
                }
                'p' => {
                    self.open_files();
                    return Action::None;
                }
                'q' => return Action::Quit,
                _ => {}
            }
        } else if ch == 'q' {
            return Action::Quit;
        }
        Action::None
    }

    pub fn paste_query(&mut self, text: &str) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            if self.query.len() + ch.len_utf8() > 512 {
                break;
            }
            self.query.push(ch);
        }
        self.query_changed();
    }

    pub fn on_backspace(&mut self) {
        if self.screen == Screen::Search
            || (self.screen == Screen::Files && self.files_input_focused)
        {
            self.query.pop();
            self.query_changed();
        }
    }

    pub fn on_enter(&mut self) -> Action {
        if self.pending_stop.is_some() || self.pending_install.is_some() {
            return Action::None;
        }
        if self.find_mode == FindMode::Text
            && !self.text_search_ready
            && matches!(self.screen, Screen::Search | Screen::Files)
        {
            self.begin_text_search();
            return Action::None;
        }
        if self.screen == Screen::Files && self.files_input_focused {
            self.files_input_focused = false;
            return Action::None;
        }
        match self.screen {
            Screen::Shell => self.launch_shell(),
            Screen::Files => self.open_button(),
            Screen::Store => self.open_selected_store(),
            Screen::Search => self.open_selected_hit(),
            Screen::Settings => {
                self.cycle_setting(1);
                Action::None
            }
            Screen::Recents => self.open_session(),
            Screen::Home => Action::None,
        }
    }

    pub fn on_esc(&mut self) {
        if self.pending_install.take().is_some() {
            return;
        }
        if self.pending_stop.take().is_some() {
            return;
        }
        if self.screen == Screen::Files
            && (!self.query.is_empty()
                || self.files_input_focused
                || self.find_mode == FindMode::Text)
        {
            self.query.clear();
            self.search_job = None;
            self.hits.clear();
            self.files_input_focused = false;
            self.find_mode = FindMode::Names;
            return;
        }
        if self.screen == Screen::Search && !self.query.is_empty() {
            self.query.clear();
            self.query_changed();
            return;
        }
        self.go_home();
    }

    pub fn on_left(&mut self) {
        match self.screen {
            Screen::Shell => self.go_home(),
            Screen::Settings => self.cycle_setting(-1),
            _ => {}
        }
    }

    pub fn on_right(&mut self) {
        match self.screen {
            Screen::Home => self.go_shell(),
            Screen::Settings => self.cycle_setting(1),
            _ => {}
        }
    }

    pub fn on_up(&mut self) {
        match self.screen {
            Screen::Shell => {}
            Screen::Files => self.move_file(-1),
            Screen::Store => self.move_store(-1),
            Screen::Search => self.move_hit(-1),
            Screen::Settings => self.move_setting(-1),
            Screen::Recents => self.move_session(-1),
            Screen::Home => {}
        }
    }

    pub fn on_down(&mut self) {
        match self.screen {
            Screen::Shell => {}
            Screen::Files => self.move_file(1),
            Screen::Store => self.move_store(1),
            Screen::Search => self.move_hit(1),
            Screen::Settings => self.move_setting(1),
            Screen::Recents => self.move_session(1),
            Screen::Home => {}
        }
    }

    fn move_file(&mut self, delta: isize) {
        if self.files_filtering() {
            self.move_hit(delta);
        } else {
            self.file_cursor = move_index(self.file_cursor, self.file_count(), delta);
        }
    }

    fn move_store(&mut self, delta: isize) {
        self.store_cursor = move_index(self.store_cursor, CATALOG.len(), delta);
    }

    fn move_hit(&mut self, delta: isize) {
        self.hit_cursor = move_index(self.hit_cursor, self.hits.len(), delta);
    }

    fn move_session(&mut self, delta: isize) {
        self.session_cursor = move_index(self.session_cursor, self.session_count(), delta);
    }

    fn move_setting(&mut self, delta: isize) {
        self.settings_cursor = move_index(self.settings_cursor, SETTINGS_COUNT, delta);
    }

    fn cycle_setting(&mut self, dir: isize) {
        match self.settings_cursor {
            0 => self.skin = theme::step_skin(self.skin, dir),
            1 => self.type_size = theme::step_type(self.type_size, dir),
            2 => self.app_size = theme::step_apps(self.app_size, dir),
            3 => {
                self.surface_style = match self.surface_style {
                    SurfaceStyle::Rounded => SurfaceStyle::Blocky,
                    SurfaceStyle::Blocky => SurfaceStyle::Rounded,
                };
            }
            4 => self.wallpaper = !self.wallpaper,
            5 => self.palette.step(dir),
            6 => self.preferred_editor.step(dir),
            _ => {}
        }
    }

    pub fn on_scroll(&mut self, down: bool) {
        match self.screen {
            Screen::Home if down => self.go_shell(),
            Screen::Home => {}
            Screen::Shell if !down => self.go_home(),
            Screen::Shell => {}
            Screen::Files => self.move_file(if down { 1 } else { -1 }),
            Screen::Store => self.move_store(if down { 1 } else { -1 }),
            Screen::Search => self.move_hit(if down { 1 } else { -1 }),
            Screen::Settings => self.move_setting(if down { 1 } else { -1 }),
            Screen::Recents => self.move_session(if down { 1 } else { -1 }),
        }
    }

    pub fn pointer_down(&mut self, col: u16, row: u16) {
        self.pointer = Some(Pointer {
            col,
            row,
            moved: false,
        });
    }

    pub fn pointer_drag(&mut self, col: u16, row: u16) {
        if let Some(pointer) = self.pointer.as_mut() {
            let dx = col.abs_diff(pointer.col);
            let dy = row.abs_diff(pointer.row);
            if dx.saturating_add(dy) >= 2 {
                pointer.moved = true;
            }
        }
    }

    pub fn pointer_up(&mut self, col: u16, row: u16) -> Action {
        let Some(pointer) = self.pointer.take() else {
            return self.on_click(col, row);
        };
        let dx = i32::from(col) - i32::from(pointer.col);
        if pointer.moved && dx.abs() >= 3 && matches!(self.screen, Screen::Home | Screen::Shell) {
            if dx < 0 {
                self.go_shell();
            } else {
                self.go_home();
            }
            return Action::None;
        }
        if pointer.moved {
            return Action::None;
        }
        self.on_click(pointer.col, pointer.row)
    }

    pub fn on_click(&mut self, col: u16, row: u16) -> Action {
        let places = self.places;
        if contains(places.back, col, row) {
            self.back();
            return Action::None;
        }
        if !self.notice.is_empty() && contains(places.notice, col, row) {
            self.notice.clear();
            return Action::None;
        }
        if contains(places.home, col, row) {
            self.go_home();
            return Action::None;
        }
        if contains(places.recent, col, row) {
            if self.screen == Screen::Recents {
                self.go_home();
            } else {
                self.open_recents();
            }
            return Action::None;
        }
        if self.screen == Screen::Home && contains(places.dot_home, col, row) {
            self.go_home();
            return Action::None;
        }
        if self.screen == Screen::Home && contains(places.dot_shell, col, row) {
            self.go_shell();
            return Action::None;
        }
        if self.pending_install.is_some() {
            if contains(places.install_cancel, col, row) {
                self.pending_install = None;
            } else if contains(places.install_confirm, col, row) {
                return self.confirm_install();
            }
            return Action::None;
        }
        match self.screen {
            Screen::Home => {
                if contains(places.workspace_agent, col, row) {
                    if let Some(session) = self.workspace_agent_session() {
                        return Action::ResumeSession {
                            id: session.id.clone(),
                        };
                    }
                    if let Some(app) = self.primary_app() {
                        return self.launch_named(&app.label, &app.bin);
                    }
                    self.open_store();
                    return Action::None;
                }
                if contains(places.workspace_review, col, row) {
                    self.open_recents();
                    return Action::None;
                }
                if contains(places.workspace_files, col, row) {
                    self.open_files();
                    return Action::None;
                }
                if contains(places.workspace_terminal, col, row) {
                    return self.launch_shell();
                }
                if contains(places.workspace_github, col, row) {
                    self.open_search();
                    return Action::None;
                }
                if contains(places.workspace_apps, col, row) {
                    self.open_store();
                    return Action::None;
                }
                if contains(places.workspace_live, col, row) {
                    self.open_recents();
                    return Action::None;
                }
                if contains(places.project, col, row) {
                    self.open_files();
                    return Action::None;
                }
                if contains(places.gear, col, row) {
                    self.open_settings();
                    return Action::None;
                }
                if contains(places.search, col, row) {
                    self.open_search();
                    return Action::None;
                }
                for (index, rect) in places.icons.iter().enumerate() {
                    if contains(*rect, col, row) {
                        return self.activate_icon(index);
                    }
                }
                Action::None
            }
            Screen::Files => self.click_files(col, row),
            Screen::Store => self.click_store(col, row),
            Screen::Search => self.click_search(col, row),
            Screen::Settings => self.click_settings(col, row),
            Screen::Recents => self.click_recents(col, row),
            Screen::Shell => {
                if contains(places.terminal_open, col, row) {
                    self.launch_shell()
                } else {
                    Action::None
                }
            }
        }
    }

    fn click_store(&mut self, col: u16, row: u16) -> Action {
        let sheet = self.places.list;
        if !contains(sheet, col, row) {
            return Action::None;
        }
        let Some(offset) = fat_row(sheet, 1, row, self.metrics) else {
            return Action::None;
        };
        let rows = theme::slot_count(sheet.height, 1, 1, self.metrics.row_h);
        if offset >= rows {
            return Action::None;
        }
        let index = self.store_scroll + offset;
        if index >= CATALOG.len() {
            return Action::None;
        }
        self.store_cursor = index;
        if col >= sheet.right().saturating_sub(8) {
            self.toggle_app(index);
            return Action::None;
        }
        self.open_selected_store()
    }

    pub fn keep_file_visible(&mut self, rows: usize) {
        if self.files_filtering() {
            self.keep_hit_visible(rows);
        } else {
            self.file_scroll = keep_visible(self.file_cursor, self.file_scroll, rows);
        }
    }

    pub fn keep_hit_visible(&mut self, rows: usize) {
        self.hit_scroll = keep_visible(self.hit_cursor, self.hit_scroll, rows);
    }

    fn click_files(&mut self, col: u16, row: u16) -> Action {
        let p = self.places;
        if !contains(p.files_sheet, col, row) {
            self.go_home();
            return Action::None;
        }
        if contains(p.file_query, col, row) {
            self.focus_file_filter();
            return Action::None;
        }
        if contains(p.file_submit, col, row) {
            if self.find_mode == FindMode::Text {
                self.begin_text_search();
            } else if self.query.is_empty() {
                self.reload_files();
                self.name_index.clear();
            } else {
                self.query.clear();
                self.query_changed();
            }
            return Action::None;
        }
        if contains(p.file_names, col, row) {
            self.focus_file_filter();
            self.set_find_mode(FindMode::Names);
            return Action::None;
        }
        if contains(p.file_text, col, row) {
            self.focus_file_filter();
            self.set_find_mode(FindMode::Text);
            return Action::None;
        }
        if contains(p.file_use, col, row) {
            let directory = if self.files_filtering() {
                self.hits
                    .get(self.hit_cursor)
                    .filter(|hit| hit.is_dir)
                    .map(|hit| hit.path.clone())
            } else {
                self.file_node(self.file_cursor)
                    .filter(|node| node.kind == NodeKind::Dir)
                    .map(|node| node.path.clone())
            };
            if let Some(directory) = directory {
                if self.change_directory(directory).is_err() {
                    return Action::None;
                }
            }
            self.go_home();
            return Action::None;
        }
        if contains(p.file_open, col, row) {
            return self.open_button();
        }
        if !contains(p.file_rows, col, row) {
            return Action::None;
        }
        let stride = self.metrics.row_h.max(1);
        let offset = usize::from((row - p.file_rows.y) / stride);
        if offset >= usize::from(p.file_rows.height / stride) {
            return Action::None;
        }
        self.files_input_focused = false;
        if self.files_filtering() {
            let index = self.hit_scroll + offset;
            if index < self.hits.len() {
                self.hit_cursor = index;
                return self.open_selected_hit();
            }
            return Action::None;
        }
        let index = self.file_scroll + offset;
        let Some(node) = self.file_node(index).cloned() else {
            return Action::None;
        };
        let selected = self.file_cursor == index;
        self.file_cursor = index;
        match node.kind {
            NodeKind::Parent => self.open_selected_file(),
            NodeKind::Dir => {
                if let Some(tree) = self.tree.as_mut() {
                    match tree.toggle(index) {
                        Ok(selected) => self.file_cursor = selected,
                        Err(error) => self.notice = error,
                    }
                }
            }
            NodeKind::File if selected => return self.open_button(),
            NodeKind::File => {}
        }
        Action::None
    }

    fn click_search(&mut self, col: u16, row: u16) -> Action {
        self.click_files(col, row)
    }

    fn open_button(&mut self) -> Action {
        if self.files_filtering() {
            return self.open_selected_hit();
        }
        let Some(node) = self.file_node(self.file_cursor).cloned() else {
            return Action::None;
        };
        match node.kind {
            NodeKind::Parent | NodeKind::Dir => {
                let _ = self.change_directory(node.path);
                Action::None
            }
            NodeKind::File => self.launch_editor(node.path),
        }
    }

    fn open_selected_hit(&mut self) -> Action {
        let Some(hit) = self.hits.get(self.hit_cursor).cloned() else {
            return Action::None;
        };
        if hit.is_dir {
            if self.change_directory(hit.path).is_ok() {
                self.screen = Screen::Files;
            }
            Action::None
        } else {
            self.launch_editor(hit.path)
        }
    }

    fn launch_editor(&mut self, path: PathBuf) -> Action {
        let Some(bin) = self.preferred_editor.command().or_else(|| {
            self.registered_apps
                .iter()
                .filter_map(|id| CATALOG.iter().find(|app| app.id == *id))
                .find(|app| app.kind == crate::catalog::AppKind::Editor)
                .map(|app| app.bin.as_str())
        }) else {
            self.open_store();
            self.notice = "Add an editor, or choose one in Settings".to_string();
            return Action::None;
        };
        if !on_path(bin) {
            return self.offer_missing(bin);
        }
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(bin)
            .to_string();
        self.start(
            &title,
            bin,
            vec![path.display().to_string()],
            self.cwd.clone(),
        )
    }

    fn click_settings(&mut self, col: u16, row: u16) -> Action {
        let sheet = self.places.settings_sheet;
        if !contains(sheet, col, row) {
            self.go_home();
            return Action::None;
        }
        let Some(offset) = fat_row(sheet, 1, row, self.metrics) else {
            return Action::None;
        };
        let rows = theme::slot_count(sheet.height, 1, 1, self.metrics.row_h);
        if offset < rows && self.settings_scroll + offset < SETTINGS_COUNT {
            self.settings_cursor = self.settings_scroll + offset;
            self.cycle_setting(1);
        }
        Action::None
    }

    fn click_recents(&mut self, col: u16, row: u16) -> Action {
        if let Some(id) = self.pending_stop.clone() {
            if contains(self.places.session_cancel, col, row) {
                self.pending_stop = None;
            } else if contains(self.places.session_stop, col, row) {
                self.pending_stop = None;
                return Action::StopSession { id };
            }
            return Action::None;
        }
        let list = self.places.list;
        if !contains(list, col, row) {
            self.go_home();
            return Action::None;
        }
        let Some(offset) = slot_at(
            list.y,
            1,
            row,
            self.metrics.recent_stride(),
            self.metrics.row_h,
        ) else {
            return Action::None;
        };
        let rows = theme::slot_count(list.height, 1, 0, self.metrics.recent_stride());
        if offset >= rows {
            return Action::None;
        }
        let index = self.session_scroll + offset;
        if index < self.session_count() {
            self.session_cursor = index;
            if self.session_backend && col >= list.right().saturating_sub(7) {
                self.pending_stop = Some(self.live_sessions[index].id.clone());
                return Action::None;
            }
            return self.open_session();
        }
        Action::None
    }

    /// Only advertise a live agent that can actually be resumed in this project.
    pub fn workspace_agent_session(&self) -> Option<&ManagedSession> {
        self.live_sessions.iter().find(|session| {
            session.spec.cwd == self.cwd
                && self
                    .primary_app()
                    .is_some_and(|app| app.bin == session.spec.bin)
                && session.status == SessionStatus::Running
        })
    }

    pub fn session_count(&self) -> usize {
        if self.session_backend {
            self.live_sessions.len()
        } else {
            self.sessions.len()
        }
    }

    pub fn replace_live_sessions(&mut self, mut sessions: Vec<ManagedSession>) {
        let selected = self
            .live_sessions
            .get(self.session_cursor)
            .map(|session| session.id.clone());
        sessions.sort_by_cached_key(|live| {
            self.sessions
                .iter()
                .position(|old| {
                    old.bin == live.spec.bin
                        && old.args == live.spec.args
                        && old.cwd == live.spec.cwd
                })
                .unwrap_or(usize::MAX)
        });
        self.live_sessions = sessions;
        self.session_cursor = selected
            .and_then(|id| {
                self.live_sessions
                    .iter()
                    .position(|session| session.id == id)
            })
            .unwrap_or(self.session_cursor)
            .min(self.live_sessions.len().saturating_sub(1));
    }

    fn open_session(&mut self) -> Action {
        if self.session_backend {
            let Some(session) = self.live_sessions.get(self.session_cursor) else {
                return Action::None;
            };
            return match session.status {
                SessionStatus::Running => Action::ResumeSession {
                    id: session.id.clone(),
                },
                SessionStatus::Exited(_) => Action::RestartSession {
                    id: session.id.clone(),
                },
            };
        }
        let Some(session) = self.sessions.get(self.session_cursor).cloned() else {
            return Action::None;
        };
        if on_path(&session.bin) {
            let Session {
                title,
                bin,
                args,
                cwd,
            } = session;
            self.start(&title, &bin, args, cwd)
        } else {
            let bin = session.bin.as_str();
            self.notice = format!("{bin} is not installed");
            Action::None
        }
    }

    pub fn keep_session_visible(&mut self, rows: usize) {
        self.session_scroll = keep_visible(self.session_cursor, self.session_scroll, rows);
    }

    pub fn keep_settings_visible(&mut self, rows: usize) {
        self.settings_scroll = keep_visible(self.settings_cursor, self.settings_scroll, rows);
    }

    pub fn keep_store_visible(&mut self, rows: usize) {
        self.store_scroll = keep_visible(self.store_cursor, self.store_scroll, rows);
    }
}

fn fat_row(sheet: Rect, header: u16, row: u16, metrics: Metrics) -> Option<usize> {
    slot_at(sheet.y, header, row, metrics.row_h, metrics.row_h)
}

fn move_index(cursor: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let cur = isize::try_from(cursor).unwrap_or(0);
    let last = isize::try_from(len - 1).unwrap_or(0);
    usize::try_from((cur + delta).clamp(0, last)).unwrap_or(0)
}

fn keep_visible(cursor: usize, scroll: usize, rows: usize) -> usize {
    if rows == 0 {
        return scroll;
    }
    if cursor < scroll {
        cursor
    } else if cursor >= scroll + rows {
        cursor + 1 - rows
    } else {
        scroll
    }
}

pub fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

pub fn on_path(bin: &str) -> bool {
    let requested = Path::new(bin);
    if requested.components().count() > 1 {
        return is_executable(requested);
    }
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| is_executable(&dir.join(bin)))
}

fn is_executable(path: &Path) -> bool {
    crate::host::executable(path)
}

pub fn user_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}

pub fn display_path(path: &Path) -> String {
    let Ok(home) = env::var("HOME") else {
        return path.display().to_string();
    };
    let Ok(rest) = path.strip_prefix(&home) else {
        return path.display().to_string();
    };
    if rest.as_os_str().is_empty() {
        "~".to_string()
    } else {
        format!("~/{}", rest.display())
    }
}

pub fn load_dir(path: &Path) -> io::Result<Vec<Node>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let entry_path = entry.path();
        // Follow directory links, but keep broken or vanished entries as files.
        let is_dir = fs::metadata(&entry_path).is_ok_and(|metadata| metadata.is_dir());
        let node = Node {
            path: entry_path,
            name,
            kind: if is_dir {
                NodeKind::Dir
            } else {
                NodeKind::File
            },
        };
        if is_dir {
            dirs.push(node);
        } else {
            files.push(node);
        }
    }
    dirs.sort_by_cached_key(|node| node.name.to_lowercase());
    files.sort_by_cached_key(|node| node.name.to_lowercase());
    let mut nodes = Vec::with_capacity(dirs.len() + files.len() + 1);
    if let Some(parent) = path.parent() {
        nodes.push(Node {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            kind: NodeKind::Parent,
        });
    }
    nodes.extend(dirs);
    nodes.extend(files);
    Ok(nodes)
}

#[cfg(test)]
mod directory_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = env::temp_dir().join(format!(
                "mobux navigation {} {}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn browsing(path: &Path) -> State {
        let mut state = State::new();
        state.change_directory(path.to_path_buf()).unwrap();
        state.screen = Screen::Files;
        state
    }

    #[test]
    fn filtering_preserves_expansion_and_focused_q_is_text() {
        let dir = TestDir::new();
        fs::create_dir_all(dir.0.join("src/deep")).unwrap();
        fs::write(dir.0.join("src/deep/needle.rs"), "a content marker").unwrap();
        let mut state = browsing(&dir.0);
        let index = state
            .tree
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .position(|row| row.node.name == "src")
            .unwrap();
        state.tree.as_mut().unwrap().toggle(index).unwrap();
        let before = state.tree.clone();
        state.on_char('/');
        assert_eq!(state.on_char('q'), Action::None);
        assert_eq!(state.query, "q");
        state.on_esc();
        assert_eq!(state.tree, before);
        state.on_char('/');
        state.paste_query("needle");
        assert_eq!(state.hits[0].path, dir.0.join("src/deep/needle.rs"));
        assert!(state.files_filtering());
        state.on_esc();
        assert!(!state.files_filtering());
        assert_eq!(state.tree, before);
    }

    #[test]
    fn content_search_requires_submission_and_edits_cancel_it() {
        let dir = TestDir::new();
        fs::write(dir.0.join("needle.rs"), "content marker").unwrap();
        let mut state = browsing(&dir.0);
        state.on_char('/');
        state.set_find_mode(FindMode::Text);
        state.paste_query("content");
        assert!(state.search_job.is_none());
        state.on_enter();
        assert!(state.search_job.is_some());
        state.on_char('x');
        assert!(state.search_job.is_none());
        assert!(!state.text_search_ready);
        assert!(state.hits.is_empty());
        state.paste_query(&"x".repeat(1000));
        assert!(state.query.len() <= 512);
    }

    #[test]
    fn changing_directory_commits_the_listing_and_resets_selection() {
        let dir = TestDir::new();
        let child = dir.0.join("folder with spaces");
        fs::create_dir(&child).unwrap();
        fs::write(child.join("file with spaces.txt"), "contents").unwrap();
        let mut state = browsing(&dir.0);
        state.file_cursor = 1;
        state.file_scroll = 1;
        state.notice = "old error".to_string();

        state.change_directory(child.clone()).unwrap();

        assert_eq!(state.cwd, child);
        assert_eq!(state.files, load_dir(&child).unwrap());
        assert_eq!((state.file_cursor, state.file_scroll), (0, 0));
        assert!(state.notice.is_empty());
    }

    #[test]
    fn failed_directory_changes_preserve_the_entire_browser_state() {
        let dir = TestDir::new();
        let file = dir.0.join("not a folder");
        fs::write(&file, "contents").unwrap();
        let vanished = dir.0.join("vanished folder");
        fs::create_dir(&vanished).unwrap();
        fs::remove_dir(&vanished).unwrap();

        for target in [file, vanished] {
            for route in ["direct", "selected", "button", "search"] {
                let mut state = browsing(&dir.0);
                // Simulate a folder that disappeared or became a file after listing.
                state.files.push(Node {
                    name: "stale folder".to_string(),
                    path: target.clone(),
                    kind: NodeKind::Dir,
                });
                state.tree = Some(crate::tree::Tree::new(state.cwd.clone(), &state.files));
                state.file_cursor = state.files.len() - 1;
                state.file_scroll = 1;
                let files = state.files.clone();
                let cursor = state.file_cursor;
                if route == "search" {
                    state.screen = Screen::Search;
                    state.hits = vec![Hit {
                        title: "stale folder".to_string(),
                        detail: "folder".to_string(),
                        path: target.clone(),
                        is_dir: true,
                    }];
                }
                let screen = state.screen;

                match route {
                    "direct" => {
                        let error = state.change_directory(target.clone()).unwrap_err();
                        assert_eq!(state.notice, error);
                    }
                    "selected" => state.open_selected_file(),
                    "button" => assert_eq!(state.open_button(), Action::None),
                    "search" => assert_eq!(state.open_selected_hit(), Action::None),
                    _ => unreachable!(),
                }

                assert_eq!(state.cwd, dir.0, "{route}");
                assert_eq!(state.files, files, "{route}");
                assert_eq!((state.file_cursor, state.file_scroll), (cursor, 1));
                assert_eq!(state.screen, screen);
                assert!(state.notice.contains(&target.display().to_string()));
            }
        }
    }

    #[test]
    fn upward_navigation_selects_the_child_folder() {
        let dir = TestDir::new();
        fs::create_dir(dir.0.join("a folder")).unwrap();
        let child = dir.0.join("z folder");
        fs::create_dir(&child).unwrap();

        for route in ["back", "selected", "button"] {
            let mut state = browsing(&child);
            match route {
                "back" => state.back(),
                "selected" => state.open_selected_file(),
                "button" => assert_eq!(state.open_button(), Action::None),
                _ => unreachable!(),
            }
            assert_eq!(state.cwd, dir.0);
            assert_eq!(state.files[state.file_cursor].path, child);
            assert_eq!(state.file_scroll, 0);
        }
    }

    #[test]
    fn back_to_a_vanished_parent_preserves_the_listing() {
        let dir = TestDir::new();
        let parent = dir.0.join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        let mut state = browsing(&child);
        let files = state.files.clone();
        fs::remove_dir_all(&parent).unwrap();

        state.back();

        assert_eq!(state.cwd, child);
        assert_eq!(state.files, files);
        assert_eq!((state.file_cursor, state.file_scroll), (0, 0));
        assert_eq!(state.screen, Screen::Files);
        assert!(!state.notice.is_empty());
    }

    #[test]
    fn opening_a_search_folder_resets_file_cursor_and_scroll() {
        let dir = TestDir::new();
        let child = dir.0.join("search folder with spaces");
        fs::create_dir(&child).unwrap();
        let mut state = browsing(&dir.0);
        state.file_cursor = 1;
        state.file_scroll = 1;
        state.open_search();
        state.query = "search folder".to_string();
        state.refresh_hits();
        assert_eq!(state.hits[0].path, child);

        assert_eq!(state.open_selected_hit(), Action::None);

        assert_eq!(state.cwd, child);
        assert_eq!(state.files, load_dir(&child).unwrap());
        assert_eq!((state.file_cursor, state.file_scroll), (0, 0));
        assert_eq!(state.screen, Screen::Files);
    }

    #[cfg(unix)]
    #[test]
    fn linked_folders_expand_on_click_and_broken_links_remain_files() {
        use std::os::unix::fs::symlink;

        let dir = TestDir::new();
        let target = dir.0.join("target folder");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("child.txt"), "contents").unwrap();
        let link = dir.0.join("linked folder");
        symlink(&target, &link).unwrap();
        let broken = dir.0.join("broken link");
        symlink(dir.0.join("missing target"), &broken).unwrap();
        let mut state = browsing(&dir.0);
        let index = state
            .files
            .iter()
            .position(|node| node.path == link)
            .unwrap();
        assert_eq!(state.files[index].kind, NodeKind::Dir);
        assert_eq!(
            state
                .files
                .iter()
                .find(|node| node.path == broken)
                .unwrap()
                .kind,
            NodeKind::File
        );
        let area = Rect::new(0, 0, 40, 48);
        state.metrics = theme::metrics(state.type_size, state.app_size, area);
        state.places = crate::paint::layout(area, &state.theme(), state.metrics);
        let row = state.places.file_rows.y + u16::try_from(index).unwrap() * state.metrics.row_h;

        assert_eq!(state.on_click(3, row), Action::None);

        assert_eq!(state.cwd, dir.0);
        assert_eq!(state.file_cursor, index);
        assert!(state
            .tree
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .any(|row| row.node.path == link.join("child.txt")));
        state.open_button();
        assert_eq!(state.cwd, link);
        state.back();
        assert_eq!(state.cwd, dir.0);
        assert_eq!(state.files[state.file_cursor].path, link);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swipe_left_from_home_opens_the_shell() {
        let mut state = State::new();
        state.screen = Screen::Home;
        state.pointer_down(20, 10);
        state.pointer_drag(10, 10);
        let action = state.pointer_up(12, 10);
        assert_eq!(action, Action::None);
        assert_eq!(state.screen, Screen::Shell);
    }

    #[test]
    fn swipe_right_from_the_shell_returns_home() {
        let mut state = State::new();
        state.screen = Screen::Shell;
        state.pointer_down(5, 10);
        state.pointer_drag(15, 10);
        state.pointer_up(14, 10);
        assert_eq!(state.screen, Screen::Home);
    }

    fn managed(index: usize, status: SessionStatus) -> ManagedSession {
        ManagedSession {
            id: format!("mobux-00000000-0000-4000-8000-{index:012}")
                .parse()
                .unwrap(),
            spec: crate::sessions::LaunchSpec {
                title: format!("App {index}"),
                bin: "test-app".to_string(),
                args: vec![],
                cwd: PathBuf::from(format!("/project-{index}")),
            },
            status,
        }
    }

    #[test]
    fn minimal_home_uses_the_painted_controls_for_project_and_app_actions() {
        let mut state = State::new();
        let area = Rect::new(0, 0, 40, 48);
        state.metrics = theme::metrics(state.type_size, state.app_size, area);
        state.places = crate::paint::layout(area, &state.theme(), state.metrics);
        let p = state.places;
        let Action::Launch { title, cwd, .. } =
            state.on_click(p.workspace_terminal.x + 1, p.workspace_terminal.y + 1)
        else {
            panic!("terminal must launch in one tap");
        };
        assert_eq!(title, "Terminal");
        assert_eq!(cwd, state.cwd);
        state.on_click(p.workspace_agent.x + 1, p.workspace_agent.y + 1);
        assert_eq!(state.screen, Screen::Store);
        state.go_home();
        state.on_click(p.workspace_live.x + 1, p.workspace_live.y + 1);
        assert_eq!(state.screen, Screen::Recents);
        state.go_home();
        state.on_char('/');
        assert_eq!(state.screen, Screen::Search);
    }

    #[test]
    fn automatic_editor_requires_an_added_editor() {
        let mut state = State::new();
        assert_eq!(state.preferred_editor, EditorChoice::Micro);
        state.preferred_editor = EditorChoice::Auto;
        assert_eq!(state.preferred_editor, EditorChoice::Auto);
        assert_eq!(
            state.launch_editor(PathBuf::from("/tmp/example.rs")),
            Action::None
        );
        assert_eq!(state.screen, Screen::Store);
        assert!(state.notice.contains("Add an editor"));
        assert!(state.registered_apps.is_empty());
    }

    #[test]
    fn apps_start_empty_and_add_remove_does_not_launch_or_stop_programs() {
        let mut state = State::new();
        assert!(state.registered_apps.is_empty());
        assert!(state.primary_app().is_none());
        let index = CATALOG.iter().position(|app| app.id == "herdr").unwrap();
        state.screen = Screen::Store;
        state.store_cursor = index;
        assert_eq!(state.on_enter(), Action::None);
        assert_eq!(state.primary_app().unwrap().label, "Herdr");
        assert_eq!(state.primary_app().unwrap().blurb, "Agent manager");
        let config = state.saved_config();
        let mut restored = State::new();
        restored.restore_config(config);
        assert_eq!(restored.registered_apps, ["herdr"]);
        restored.toggle_app(index);
        assert!(restored.registered_apps.is_empty());
        assert!(restored.live_sessions.is_empty());
    }

    #[test]
    fn install_confirmation_is_cancellable_and_enter_does_not_authorize_it() {
        let mut state = State::new();
        state.screen = Screen::Store;
        state.pending_install =
            crate::install::Plan::for_app(crate::catalog::by_bin("gh").unwrap(), true);
        assert_eq!(state.on_enter(), Action::None);
        assert!(state.pending_install.is_some());
        assert_eq!(state.on_char('q'), Action::None);
        state.on_esc();
        assert!(state.pending_install.is_none());
        assert_eq!(state.screen, Screen::Store);
        state.pending_install =
            crate::install::Plan::for_app(crate::catalog::by_bin("micro").unwrap(), true);
        state.go_home();
        assert!(state.pending_install.is_none());
    }

    #[test]
    fn live_sessions_have_no_history_cap_and_exited_sessions_require_restart() {
        let mut state = State::new();
        state.session_backend = true;
        state.replace_live_sessions(
            (0..12)
                .map(|index| managed(index, SessionStatus::Running))
                .collect(),
        );
        state.open_recents();
        assert_eq!(state.session_count(), 12);
        let id = state.live_sessions[0].id.clone();
        assert_eq!(state.on_enter(), Action::ResumeSession { id: id.clone() });
        state.live_sessions[0].status = SessionStatus::Exited(Some(0));
        assert_eq!(state.on_enter(), Action::RestartSession { id });
        assert!(state.sessions.is_empty());
    }

    #[test]
    fn stop_requires_confirmation_and_pins_the_original_generation() {
        let mut state = State::new();
        state.session_backend = true;
        state.replace_live_sessions(vec![
            managed(1, SessionStatus::Running),
            managed(2, SessionStatus::Running),
        ]);
        state.open_recents();
        state.places.list = Rect::new(1, 1, 38, 40);
        state.places.session_cancel = Rect::new(3, 20, 14, 2);
        state.places.session_stop = Rect::new(21, 20, 14, 2);
        let original = state.live_sessions[0].id.clone();
        assert_eq!(state.on_click(35, 2), Action::None);
        assert_eq!(state.pending_stop, Some(original.clone()));
        assert_eq!(state.on_enter(), Action::None);
        state.live_sessions.swap(0, 1);
        assert_eq!(state.on_click(22, 20), Action::StopSession { id: original });
        assert!(state.pending_stop.is_none());
        assert_eq!(state.on_char('y'), Action::None);
        state.on_click(35, 2);
        state.on_esc();
        assert!(state.pending_stop.is_none());
        assert_eq!(state.screen, Screen::Recents);
    }

    #[test]
    fn choosing_a_folder_keeps_it_as_the_home_launch_context() {
        let mut state = State::new();
        state.screen = Screen::Files;
        state.cwd = PathBuf::from("/a project with spaces");
        state.places.files_sheet = Rect::new(2, 3, 36, 30);
        state.places.file_use = Rect::new(20, 30, 15, 2);
        assert_eq!(state.on_click(25, 31), Action::None);
        assert_eq!(state.screen, Screen::Home);
        state.go_shell();
        let Action::Launch { cwd, .. } = state.on_enter() else {
            panic!("expected launch plan");
        };
        assert_eq!(cwd, PathBuf::from("/a project with spaces"));
    }

    #[test]
    fn saved_preferences_restore_without_hijacking_the_startup_directory() {
        let mut first = State::new();
        first.palette = Palette::OneDarkPro;
        first.preferred_editor = EditorChoice::Neovim;
        first.wallpaper = false;
        first.note_session("Terminal", "/bin/sh", &[], PathBuf::from("/old project"));
        let saved = first.saved_config();
        let mut second = State::new();
        let initial = second.cwd.clone();
        second.restore_config(saved.clone());
        assert_eq!(second.saved_config(), saved);
        assert_eq!(second.cwd, initial);
    }

    #[test]
    fn settings_scroll_maps_taps_only_to_visible_rows() {
        let mut state = State::new();
        state.open_settings();
        state.places.settings_sheet = Rect::new(2, 2, 30, 6);
        state.settings_cursor = 6;
        state.keep_settings_visible(2);
        assert_eq!(state.settings_scroll, 5);
        let before = state.preferred_editor;
        state.on_click(3, 5);
        assert_eq!(state.settings_cursor, 6);
        assert_ne!(state.preferred_editor, before);
        let before = state.preferred_editor;
        state.on_click(3, 7);
        assert_eq!(state.preferred_editor, before);
    }

    #[test]
    fn a_small_drag_does_not_change_page() {
        let mut state = State::new();
        state.pointer_down(10, 10);
        state.pointer_drag(12, 10);
        state.pointer_up(12, 11);
        assert_eq!(state.screen, Screen::Home);
    }

    #[test]
    fn click_without_drag_can_open_files() {
        let mut state = State::new();
        state.places.icons[2] = Rect {
            x: 4,
            y: 8,
            width: 5,
            height: 4,
        };
        state.pointer_down(5, 9);
        state.pointer_up(5, 9);
        assert_eq!(state.screen, Screen::Files);
    }

    #[test]
    fn vertical_swipe_on_the_home_screen_opens_the_shell() {
        let mut state = State::new();
        state.on_scroll(true);
        assert_eq!(state.screen, Screen::Shell);
        state.on_scroll(false);
        assert_eq!(state.screen, Screen::Home);
    }

    #[test]
    fn load_dir_lists_parent_then_directories_then_files() {
        let dir = env::temp_dir().join(format!("mobux-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("b.txt"), b"x").unwrap();
        fs::create_dir(dir.join("a")).unwrap();
        let nodes = load_dir(&dir).unwrap();
        let names: Vec<&str> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names.first().copied(), Some(".."));
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b.txt"));
        let dir_at = names.iter().position(|name| *name == "a").unwrap();
        let file_at = names.iter().position(|name| *name == "b.txt").unwrap();
        assert!(dir_at < file_at);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_square_button_opens_recent_apps() {
        let mut state = State::new();
        state.places.recent = Rect {
            x: 20,
            y: 30,
            width: 10,
            height: 1,
        };
        state.on_click(22, 30);
        assert_eq!(state.screen, Screen::Recents);
    }

    #[test]
    fn an_opened_app_stays_on_the_switcher() {
        let mut state = State::new();
        let cwd = PathBuf::from("/tmp");
        state.note_session("git", "lazygit", &[], cwd.clone());
        state.note_session("open code", "opencode", &[], cwd.clone());
        state.note_session("git", "lazygit", &[], cwd);
        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions[0].title, "git");
        assert_eq!(state.sessions[1].title, "open code");
    }

    #[test]
    fn settings_cycles_minimal_android_and_iphone() {
        let mut state = State::new();
        state.open_settings();
        state.on_enter();
        assert_eq!(state.skin, SkinId::Android);
        state.on_enter();
        assert_eq!(state.skin, SkinId::Iphone);
        state.on_enter();
        assert_eq!(state.skin, SkinId::Minimal);
    }

    #[test]
    fn corners_default_to_rounded_and_toggle_with_keyboard() {
        let mut state = State::new();
        assert_eq!(SurfaceStyle::default(), SurfaceStyle::Rounded);
        assert_eq!(state.surface_style, SurfaceStyle::Rounded);
        assert_eq!(state.surface_style.label(), "Rounded");
        state.open_settings();
        state.on_up();
        assert_eq!(state.settings_cursor, 0);
        for _ in 0..3 {
            state.on_down();
        }
        assert_eq!(state.settings_cursor, 3);
        assert_eq!(state.on_enter(), Action::None);
        assert_eq!(state.surface_style, SurfaceStyle::Blocky);
        assert_eq!(state.surface_style.label(), "Blocky");
        state.on_left();
        assert_eq!(state.surface_style, SurfaceStyle::Rounded);
        state.on_right();
        assert_eq!(state.surface_style, SurfaceStyle::Blocky);
        state.on_scroll(false);
        assert_eq!(state.settings_cursor, 2);
        state.on_scroll(true);
        state.on_scroll(true);
        assert_eq!(state.settings_cursor, 4);
    }

    #[test]
    fn tapping_the_fourth_settings_row_toggles_corners() {
        let mut state = State::new();
        state.open_settings();
        let row_h = state.metrics.row_h;
        let sheet = Rect::new(2, 3, 30, 2 + SETTINGS_COUNT as u16 * row_h);
        state.places.settings_sheet = sheet;
        let corners_y = sheet.y + 1 + 3 * row_h;
        for row in corners_y..corners_y + row_h {
            let before = state.surface_style;
            state.pointer_down(sheet.x, row);
            assert_eq!(state.pointer_up(sheet.x, row), Action::None);
            assert_eq!(state.settings_cursor, 3);
            assert_ne!(state.surface_style, before);
        }
        let before = state.surface_style;
        state.on_click(sheet.x, corners_y + row_h);
        assert_eq!(state.surface_style, before);
        assert_eq!(state.settings_cursor, 4);
        assert!(!state.wallpaper);
    }

    #[test]
    fn palette_can_change_without_switching_layout_or_corner_style() {
        let mut state = State::new();
        assert_eq!(state.palette, Palette::GruvboxMaterialDark);
        state.open_settings();
        state.settings_cursor = 5;
        state.on_enter();
        assert_eq!(state.palette, Palette::OneDarkPro);
        assert_eq!(state.skin, SkinId::Minimal);
        assert_eq!(state.surface_style, SurfaceStyle::Rounded);
        state.on_right();
        assert_eq!(state.palette, Palette::GruvboxMaterialLight);
        state.on_left();
        assert_eq!(state.palette, Palette::OneDarkPro);
    }

    #[test]
    fn wallpaper_defaults_on_and_can_be_disabled_without_changing_the_skin() {
        let mut state = State::new();
        assert!(state.wallpaper);
        state.open_settings();
        state.settings_cursor = 4;
        state.on_enter();
        assert!(!state.wallpaper);
        assert_eq!(state.skin, SkinId::Minimal);
        assert_eq!(state.surface_style, SurfaceStyle::Rounded);
        state.on_right();
        assert!(state.wallpaper);
    }

    #[test]
    fn shell_enter_launches_the_user_shell_in_the_current_directory() {
        let mut state = State::new();
        state.cwd = PathBuf::from("/tmp/project with spaces");
        state.go_shell();
        let expected = Action::Launch {
            title: "Terminal".to_string(),
            bin: user_shell(),
            args: Vec::new(),
            cwd: state.cwd.clone(),
        };
        assert_eq!(state.on_enter(), expected);
        assert_eq!(state.screen, Screen::Shell);
        assert!(
            state.sessions.is_empty(),
            "history is recorded only after a successful spawn"
        );
    }

    #[test]
    fn tapping_anywhere_in_the_shell_button_launches_the_user_shell() {
        let mut state = State::new();
        state.go_shell();
        let button = Rect::new(5, 6, 4, 3);
        state.places.terminal_open = button;
        let expected = Action::Launch {
            title: "Terminal".to_string(),
            bin: user_shell(),
            args: Vec::new(),
            cwd: state.cwd.clone(),
        };
        assert_eq!(state.on_click(button.x - 1, button.y), Action::None);
        assert_eq!(state.on_click(button.right(), button.y), Action::None);
        assert_eq!(state.on_click(button.x, button.bottom()), Action::None);
        for row in button.y..button.bottom() {
            for col in button.x..button.right() {
                state.pointer_down(col, row);
                assert_eq!(state.pointer_up(col, row), expected);
            }
        }
        assert!(
            state.sessions.is_empty(),
            "launch plans do not record successful starts"
        );
    }

    #[test]
    fn shell_characters_do_not_execute_commands() {
        let mut state = State::new();
        state.go_shell();
        let cwd = state.cwd.clone();
        for ch in "cd /tmp\necho 'quoted input'\r".chars() {
            assert_eq!(state.on_char(ch), Action::None);
        }
        state.on_backspace();
        state.on_up();
        state.on_down();
        assert!(state.sessions.is_empty());
        assert_eq!(state.cwd, cwd);
        assert_eq!(state.screen, Screen::Shell);
        state.on_esc();
        assert_eq!(state.screen, Screen::Home);
    }

    #[test]
    fn hidden_dots_do_not_navigate() {
        let mut state = State::new();
        state.places.dot_home = Rect::new(10, 20, 2, 1);
        state.places.dot_shell = Rect::new(14, 20, 2, 1);
        // Keep the dots inside sheets so a sheet's outside-tap dismissal is not tested.
        let sheet = Rect::new(0, 0, 40, 21);
        state.places.files_sheet = sheet;
        state.places.search_sheet = sheet;
        state.places.settings_sheet = sheet;
        state.metrics.row_h = 2;
        for screen in [
            Screen::Shell,
            Screen::Files,
            Screen::Store,
            Screen::Search,
            Screen::Settings,
            Screen::Recents,
        ] {
            state.places.list = if screen == Screen::Shell {
                Rect::default()
            } else {
                sheet
            };
            for col in [10, 14] {
                state.screen = screen;
                assert_eq!(state.on_click(col, 20), Action::None);
                assert_eq!(state.screen, screen);
            }
        }
        state.go_home();
        state.on_click(14, 20);
        assert_eq!(state.screen, Screen::Shell);
    }

    #[test]
    fn the_same_agent_in_two_projects_has_distinct_history_entries() {
        let mut state = State::new();
        let first = PathBuf::from("/tmp/project-one");
        let second = PathBuf::from("/tmp/project-two");
        let args = vec!["--resume".to_string()];
        state.note_session("Agent", "opencode", &args, first.clone());
        state.note_session("Agent", "opencode", &args, second.clone());
        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions[0].cwd, second);
        assert_eq!(state.sessions[1].cwd, first);
        state.note_session("Agent", "opencode", &args, first.clone());
        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions[0].cwd, first);
        assert_eq!(state.sessions[1].cwd, second);
    }

    #[test]
    fn the_corner_control_opens_settings() {
        let mut state = State::new();
        state.places.gear = Rect {
            x: 30,
            y: 0,
            width: 5,
            height: 2,
        };
        state.on_click(31, 0);
        assert_eq!(state.screen, Screen::Settings);
    }
}
