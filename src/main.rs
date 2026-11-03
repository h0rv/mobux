use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, window_size, BeginSynchronizedUpdate, EndSynchronizedUpdate,
    EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use mobux::paint;
use mobux::sessions::{Backend, LaunchSpec, ManagedSession, ReturnTo, SessionStatus};
use mobux::state::{Action, State};
use mobux::surface::{Background, CellSize};
use mobux::theme::{self, slot_count};

struct Restored;
impl Drop for Restored {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen,
            Show
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CommandLine {
    Run(Option<std::path::PathBuf>),
    Help,
    Version,
}

fn parse_args(args: &[std::ffi::OsString]) -> Result<CommandLine, String> {
    match args {
        [] => Ok(CommandLine::Run(None)),
        [arg] if arg == "--help" || arg == "-h" => Ok(CommandLine::Help),
        [arg] if arg == "--version" || arg == "-V" => Ok(CommandLine::Version),
        [arg] if !arg.as_encoded_bytes().starts_with(b"-") => {
            Ok(CommandLine::Run(Some(arg.into())))
        }
        [separator, path] if separator == "--" => Ok(CommandLine::Run(Some(path.into()))),
        _ => Err("Usage: mobux [DIRECTORY]\nUse mobux --help for options.".to_string()),
    }
}

fn main() -> io::Result<()> {
    let project = match parse_args(&std::env::args_os().skip(1).collect::<Vec<_>>())
        .map_err(io::Error::other)?
    {
        CommandLine::Help => {
            println!("Mobux: a touch-first terminal workspace\n\nUsage: mobux [DIRECTORY]\n       mobux --help\n       mobux --version\n\nTap Project to browse folders. Open apps in that folder.\nLive sessions use tmux 3.4+ with Home and Apps touch targets.\nPreferences are saved under ~/.config/mobux (or XDG_CONFIG_HOME).\n\nMOBUX_THEME       Preset name or Base16 JSON file\nMOBUX_GRAPHICS    kitty or cells\nMOBUX_SESSIONS    foreground disables persistent sessions\nMOBUX_CONFIG_DIR  Absolute configuration directory");
            return Ok(());
        }
        CommandLine::Version => {
            println!("Mobux {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CommandLine::Run(project) => project,
    };
    let mut app = State::new();
    if let Some(path) = project {
        let path = std::fs::canonicalize(&path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Cannot open project {}: {err}", path.display()),
            )
        })?;
        app.change_directory(path).map_err(io::Error::other)?;
    }
    let store = load_preferences(&mut app);
    app.image_icons = mobux::graphics::supported();
    if let Ok(theme) = std::env::var("MOBUX_THEME") {
        match mobux::palette::Palette::from_name_or_file(&theme) {
            Ok(palette) => app.palette = palette,
            Err(err) => app.notice = format!("Theme: {err}"),
        }
    }
    let _restored = Restored;
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    let result = run(&mut terminal, &mut app, store);
    let _ = disable_raw_mode();
    result
}

fn load_preferences(app: &mut State) -> Option<mobux::config::Store> {
    match mobux::config::Store::discover() {
        Ok(store) => match store.load() {
            Ok(config) => {
                app.restore_config(config);
                Some(store)
            }
            Err(err) => {
                app.notice = format!("Settings not loaded or overwritten: {err}");
                None
            }
        },
        Err(err) => {
            app.notice = err;
            None
        }
    }
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut State,
    store: Option<mobux::config::Store>,
) -> io::Result<()> {
    let backend = session_backend(app);
    let mut last_session_check = Instant::now();
    let mut preferences = Preferences::new(store, app.saved_config());
    let mut graphics = mobux::graphics::Graphics::default();
    let mut dirty = true;
    let mut minute = chrono::Local::now().timestamp() / 60;
    loop {
        dirty |= app.poll_search();
        dirty |= preferences.save(app, Instant::now());
        if dirty {
            present(terminal, app, &mut graphics)?;
            dirty = false;
        }
        if matches!(
            app.screen,
            mobux::state::Screen::Home | mobux::state::Screen::Recents
        ) && app.pending_stop.is_none()
            && last_session_check.elapsed() >= Duration::from_secs(1)
        {
            if let Some(backend) = &backend {
                dirty |= refresh_sessions(app, backend);
            }
            last_session_check = Instant::now();
            if dirty {
                continue;
            }
        }
        if !event::poll(Duration::from_millis(250))? {
            let now = chrono::Local::now().timestamp() / 60;
            if now != minute {
                minute = now;
                dirty = app.screen == mobux::state::Screen::Home;
            }
            continue;
        }
        let before = app.screen;
        let event = event::read()?;
        dirty = event_needs_redraw(&event);
        let action = match event {
            Event::Key(key) if key.kind == KeyEventKind::Release => Action::None,
            Event::Key(key) => on_key(app, key.code, key.modifiers),
            Event::Mouse(mouse) => on_mouse(app, mouse.kind, mouse.column, mouse.row),
            Event::Paste(text) => {
                if app.screen == mobux::state::Screen::Search
                    || (app.screen == mobux::state::Screen::Files && app.files_input_focused)
                {
                    app.paste_query(&text);
                }
                Action::None
            }
            Event::Resize(_, _) => {
                graphics.clear(&mut stdout())?;
                Action::None
            }
            _ => Action::None,
        };
        match action {
            Action::None => {}
            Action::Quit => {
                preferences.flush(app).map_err(io::Error::other)?;
                return Ok(());
            }
            Action::Launch {
                title,
                bin,
                args,
                cwd,
            } => {
                if let Some(backend) = &backend {
                    let spec = LaunchSpec {
                        title,
                        bin,
                        args,
                        cwd,
                    };
                    match backend.ensure(&spec) {
                        Ok(session) => use_session(terminal, app, &mut graphics, backend, session)?,
                        Err(err) => app.notice = err,
                    }
                } else {
                    let result = launch(&title, &bin, &args, &cwd, &mut graphics);
                    app.go_home();
                    match result {
                        Ok(status) => {
                            app.note_session(&title, &bin, &args, cwd);
                            if !status.success() {
                                app.notice = format!("{title}: {status}");
                            }
                        }
                        Err(err) => app.notice = err.to_string(),
                    }
                    resume_and_present(terminal, app, &mut graphics)?;
                }
                dirty = true;
            }
            Action::ResumeSession { id } => {
                if let Some(backend) = &backend {
                    match backend.list().and_then(|sessions| {
                        sessions
                            .into_iter()
                            .find(|session| session.id == id)
                            .ok_or_else(|| "Session no longer exists".to_string())
                    }) {
                        Ok(session) => use_session(terminal, app, &mut graphics, backend, session)?,
                        Err(err) => {
                            app.notice = err;
                            let _ = refresh_sessions(app, backend);
                        }
                    }
                }
                dirty = true;
            }
            Action::RestartSession { id } => {
                if let Some(backend) = &backend {
                    match backend.restart(&id) {
                        Ok(session) => use_session(terminal, app, &mut graphics, backend, session)?,
                        Err(err) => app.notice = err,
                    }
                }
                dirty = true;
            }
            Action::StopSession { id } => {
                if let Some(backend) = &backend {
                    match backend.stop(&id) {
                        Ok(()) => {
                            let _ = refresh_sessions(app, backend);
                            app.notice = "Session stopped".to_string();
                        }
                        Err(err) => app.notice = err,
                    }
                }
                dirty = true;
            }
        }
        if app.screen == mobux::state::Screen::Recents && before != app.screen {
            if let Some(backend) = &backend {
                dirty |= refresh_sessions(app, backend);
            }
        }
    }
}

struct Preferences {
    store: Option<mobux::config::Store>,
    saved: mobux::config::Config,
    retry_at: Option<Instant>,
    last_error: Option<String>,
}

impl Preferences {
    fn new(store: Option<mobux::config::Store>, saved: mobux::config::Config) -> Self {
        Self {
            store,
            saved,
            retry_at: None,
            last_error: None,
        }
    }

    fn save(&mut self, app: &mut State, now: Instant) -> bool {
        let current = app.saved_config();
        if current == self.saved || self.retry_at.is_some_and(|deadline| now < deadline) {
            return false;
        }
        let Some(store) = &self.store else {
            return false;
        };
        match store.save(&current) {
            Ok(()) => {
                self.saved = current;
                self.retry_at = None;
                let clear_notice = self
                    .last_error
                    .as_ref()
                    .is_some_and(|error| app.notice == *error);
                self.last_error = None;
                if clear_notice {
                    app.notice.clear();
                }
                clear_notice
            }
            Err(err) => {
                self.retry_at = Some(now + Duration::from_secs(2));
                let message = format!("Settings not saved; retrying: {err}");
                let changed = app.notice != message;
                app.notice = message.clone();
                self.last_error = Some(message);
                changed
            }
        }
    }

    fn flush(&mut self, app: &mut State) -> Result<(), String> {
        self.retry_at = None;
        let _ = self.save(app, Instant::now());
        if self.store.is_some() && self.saved != app.saved_config() {
            return Err(self
                .last_error
                .clone()
                .unwrap_or_else(|| "Settings could not be saved".to_string()));
        }
        Ok(())
    }
}

fn session_backend(app: &mut State) -> Option<Backend> {
    if std::env::var("MOBUX_SESSIONS").as_deref() == Ok("foreground") || !Backend::available() {
        return None;
    }
    let backend = match std::env::var("MOBUX_SESSION_SOCKET") {
        Ok(socket) => match Backend::with_socket(&socket) {
            Ok(backend) => backend,
            Err(err) => {
                app.notice = err;
                return None;
            }
        },
        Err(_) => Backend::new(),
    };
    match backend.list() {
        Ok(sessions) => {
            app.session_backend = true;
            app.replace_live_sessions(sessions);
            Some(backend)
        }
        Err(err) => {
            app.notice = format!("Live sessions unavailable: {err}");
            None
        }
    }
}

fn refresh_sessions(app: &mut State, backend: &Backend) -> bool {
    match backend.list() {
        Ok(sessions) => {
            // Sorting in State follows launch history; compare after that stable ordering.
            let previous = app.live_sessions.clone();
            app.replace_live_sessions(sessions);
            app.live_sessions != previous
        }
        Err(err) => {
            let changed = app.notice != err;
            app.notice = err;
            changed
        }
    }
}

fn use_session(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut State,
    graphics: &mut mobux::graphics::Graphics,
    backend: &Backend,
    session: ManagedSession,
) -> io::Result<()> {
    app.note_session(
        &session.spec.title,
        &session.spec.bin,
        &session.spec.args,
        session.spec.cwd.clone(),
    );
    if let SessionStatus::Exited(code) = session.status {
        let _ = refresh_sessions(app, backend);
        app.open_recents();
        app.notice = format!(
            "Exited {}. Tap Restart to run it again.",
            code.map_or_else(|| "?".to_string(), |value| value.to_string())
        );
        return Ok(());
    }
    // Keep the workspace visible through validation and tmux preflight.
    let mut suspended = false;
    let result = backend.attach_with(&session, || {
        suspended = true;
        handoff(graphics).map_err(|error| error.to_string())?;
        Ok(())
    });
    app.go_home();
    let _ = refresh_sessions(app, backend);
    match result {
        Ok(ReturnTo::Home) => {}
        Ok(ReturnTo::Apps) => app.open_recents(),
        Err(err) => app.notice = err,
    }
    if app.live_sessions.iter().any(|current| {
        current.id == session.id && matches!(current.status, SessionStatus::Exited(_))
    }) {
        app.open_recents();
    }
    if suspended {
        resume_and_present(terminal, app, graphics)?;
    }
    Ok(())
}

fn event_needs_redraw(event: &Event) -> bool {
    !matches!(event,
        Event::Key(key) if key.kind == KeyEventKind::Release)
        && !matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Moved | MouseEventKind::Drag(_) | MouseEventKind::Down(_)))
        && !matches!(event, Event::FocusGained | Event::FocusLost)
}

fn present(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut State,
    graphics: &mut mobux::graphics::Graphics,
) -> io::Result<()> {
    synchronized_update(|| draw_frame(terminal, app, graphics))
}

fn draw_frame(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut State,
    graphics: &mut mobux::graphics::Graphics,
) -> io::Result<()> {
    let cells = window_size()
        .ok()
        .filter(|size| {
            size.columns > 0
                && size.rows > 0
                && size.width >= size.columns
                && size.height >= size.rows
        })
        .map(|size| CellSize {
            width: size.width / size.columns,
            height: size.height / size.rows,
        })
        .unwrap_or_default();
    let mut background = None;
    terminal.draw(|frame| background = draw_scene(frame, app, cells))?;
    graphics.draw(&mut stdout(), background)
}

fn synchronized_update(work: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    execute!(stdout(), BeginSynchronizedUpdate)?;
    let result = work();
    let end = execute!(stdout(), EndSynchronizedUpdate);
    result.and(end)
}

fn resume_and_present(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut State,
    graphics: &mut mobux::graphics::Graphics,
) -> io::Result<()> {
    // Hold screen entry, buffer reset, and the complete first frame together.
    synchronized_update(|| {
        resume(terminal)?;
        draw_frame(terminal, app, graphics)
    })
}

fn handoff(graphics: &mut mobux::graphics::Graphics) -> io::Result<()> {
    // Delete our graphics only at the actual ownership transfer, not during preflight.
    synchronized_update(|| {
        graphics.clear(&mut stdout())?;
        suspend()
    })
}

#[cfg(test)]
fn draw(frame: &mut ratatui::Frame<'_>, app: &mut State) {
    let _ = draw_scene(frame, app, CellSize::default());
}

fn draw_scene(
    frame: &mut ratatui::Frame<'_>,
    app: &mut State,
    cells: CellSize,
) -> Option<Background> {
    let area = frame.area();
    app.metrics = theme::metrics(app.type_size, app.app_size, area);
    app.places = paint::layout(area, &app.theme(), app.metrics);
    let list_rows = slot_count(app.places.list.height, 1, 1, app.metrics.row_h);
    let file_rows = usize::from(app.places.file_rows.height / app.metrics.row_h.max(1));
    let hit_rows = file_rows;
    let session_rows = slot_count(app.places.list.height, 1, 0, app.metrics.recent_stride());
    app.keep_file_visible(file_rows);
    if app.screen == mobux::state::Screen::Search {
        app.keep_hit_visible(hit_rows);
    }
    app.keep_store_visible(list_rows);
    app.keep_settings_visible(slot_count(
        app.places.settings_sheet.height,
        1,
        1,
        app.metrics.row_h,
    ));
    app.keep_session_visible(session_rows);
    paint::scene(frame, app, cells)
}

fn on_key(app: &mut State, code: KeyCode, modifiers: KeyModifiers) -> Action {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Action::Quit;
    }
    match code {
        KeyCode::Char(ch) => app.on_char(ch),
        KeyCode::Backspace => {
            app.on_backspace();
            Action::None
        }
        KeyCode::Enter => app.on_enter(),
        KeyCode::Esc => {
            app.on_esc();
            Action::None
        }
        KeyCode::Left => {
            app.on_left();
            Action::None
        }
        KeyCode::Right => {
            app.on_right();
            Action::None
        }
        KeyCode::Up => {
            app.on_up();
            Action::None
        }
        KeyCode::Down => {
            app.on_down();
            Action::None
        }
        _ => Action::None,
    }
}

fn on_mouse(app: &mut State, kind: MouseEventKind, col: u16, row: u16) -> Action {
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.pointer_down(col, row);
            Action::None
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.pointer_drag(col, row);
            Action::None
        }
        MouseEventKind::Up(MouseButton::Left) => app.pointer_up(col, row),
        MouseEventKind::ScrollDown | MouseEventKind::ScrollLeft => {
            app.on_scroll(true);
            Action::None
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollRight => {
            app.on_scroll(false);
            Action::None
        }
        _ => Action::None,
    }
}

fn launch(
    title: &str,
    bin: &str,
    args: &[String],
    cwd: &std::path::Path,
    graphics: &mut mobux::graphics::Graphics,
) -> io::Result<std::process::ExitStatus> {
    handoff(graphics)?;
    let _ = writeln!(stdout(), "{title} (foreground)");
    let status = std::process::Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .status();
    status.map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Cannot launch {title} in {}: {err}", cwd.display()),
        )
    })
}

fn suspend() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        Show
    )?;
    stdout().flush()
}

fn resume(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    execute!(
        stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    enable_raw_mode()?;
    terminal.clear()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mobux::find::Hit;
    use mobux::state::{Node, NodeKind, Screen};
    use mobux::theme::TypeSize;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    #[test]
    fn cli_project_paths_are_literal_and_help_does_not_require_a_terminal() {
        use std::ffi::OsString;
        assert_eq!(parse_args(&[]).unwrap(), CommandLine::Run(None));
        assert_eq!(
            parse_args(&[OsString::from("--help")]).unwrap(),
            CommandLine::Help
        );
        assert_eq!(
            parse_args(&[OsString::from("a folder; not a command")]).unwrap(),
            CommandLine::Run(Some("a folder; not a command".into()))
        );
        assert_eq!(
            parse_args(&[OsString::from("--"), OsString::from("-folder")]).unwrap(),
            CommandLine::Run(Some("-folder".into()))
        );
        assert!(parse_args(&[OsString::from("--unknown")]).is_err());
    }

    #[test]
    fn temporary_save_failures_retry_without_disabling_preferences() {
        let root = std::env::temp_dir().join(format!("mobux-autosave-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let blocker = root.join("blocked");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let store = mobux::config::Store::at(blocker.join("state.json"));
        let mut app = State::new();
        let mut persistence = Preferences::new(Some(store.clone()), app.saved_config());
        app.wallpaper = false;
        let start = Instant::now();
        assert!(persistence.save(&mut app, start));
        assert!(persistence.store.is_some());
        assert!(persistence.saved.settings.wallpaper);
        std::fs::remove_file(&blocker).unwrap();
        assert!(!persistence.save(&mut app, start + Duration::from_secs(1)));
        assert!(persistence.save(&mut app, start + Duration::from_secs(3)));
        assert!(!store.load().unwrap().settings.wallpaper);
        assert_eq!(persistence.saved, app.saved_config());
        assert!(app.notice.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pointer_motion_and_key_releases_do_not_schedule_frames() {
        use ratatui::crossterm::event::{KeyEvent, MouseEvent};
        let mouse = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            })
        };
        assert!(!event_needs_redraw(&mouse(MouseEventKind::Moved)));
        assert!(!event_needs_redraw(&mouse(MouseEventKind::Down(
            MouseButton::Left
        ))));
        assert!(!event_needs_redraw(&mouse(MouseEventKind::Drag(
            MouseButton::Left
        ))));
        assert!(event_needs_redraw(&mouse(MouseEventKind::Up(
            MouseButton::Left
        ))));
        assert!(!event_needs_redraw(&Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release
        ))));
        assert!(event_needs_redraw(&Event::Resize(40, 48)));
    }

    #[test]
    fn recent_button_toggles_recents_and_home_from_every_screen() {
        for screen in [
            Screen::Home,
            Screen::Shell,
            Screen::Files,
            Screen::Store,
            Screen::Search,
            Screen::Settings,
        ] {
            let mut app = State::new();
            app.screen = screen;
            app.places.recent = Rect::new(20, 30, 10, 1);
            assert_eq!(app.on_click(22, 30), Action::None);
            assert_eq!(app.screen, Screen::Recents);
            assert!(app.sessions.is_empty());
            assert_eq!(app.on_click(22, 30), Action::None);
            assert_eq!(app.screen, Screen::Home);
        }
    }

    fn populate_files(app: &mut State) {
        app.files = (0..100)
            .map(|index| Node {
                name: format!("file{index}"),
                path: app.cwd.join(format!("file{index}")),
                kind: NodeKind::File,
            })
            .collect();
    }

    #[test]
    fn app_rows_use_the_same_tall_hit_targets_as_the_painter() {
        let mut app = State::new();
        app.screen = Screen::Store;
        app.places.list = Rect::new(1, 1, 32, 8);
        let row_h = app.metrics.row_h;
        let _ = app.on_click(3, app.places.list.y + 1 + row_h + 1);
        assert_eq!(app.store_cursor, 1);
        let _ = app.on_click(3, app.places.list.bottom() - 1);
        assert_eq!(app.store_cursor, 1);
    }

    #[test]
    fn files_ignore_unpainted_rows_and_footer() {
        for row_h in [2, 3] {
            let mut app = State::new();
            app.screen = Screen::Files;
            app.metrics.row_h = row_h;
            app.places.files_sheet = Rect::new(10, 5, 20, 15);
            app.places.file_rows = Rect::new(11, 6, 18, 13 - row_h);
            app.file_scroll = 2;
            populate_files(&mut app);
            let rows = slot_count(15, 1, row_h + 1, row_h);
            let first_hidden = 6 + rows as u16 * row_h;
            for row in first_hidden..20 {
                assert_eq!(app.on_click(11, row), Action::None);
                assert_eq!(app.file_cursor, 0);
                assert_eq!(app.screen, Screen::Files);
            }
            assert_eq!(app.on_click(11, 6), Action::None);
            assert_eq!(app.file_cursor, 2);
        }
    }

    #[test]
    fn search_ignores_partial_unpainted_rows() {
        for row_h in [2, 3] {
            let mut app = State::new();
            app.screen = Screen::Search;
            app.metrics.row_h = row_h;
            app.places.search_sheet = Rect::new(10, 5, 20, 14);
            app.places.files_sheet = app.places.search_sheet;
            app.places.file_rows = Rect::new(11, 8, 18, 11);
            app.hit_scroll = 2;
            app.hits = (0..100)
                .map(|index| Hit {
                    title: format!("folder{index}"),
                    detail: String::new(),
                    path: app.cwd.clone(),
                    is_dir: true,
                })
                .collect();
            let rows = slot_count(14, 3, 0, row_h);
            let first_hidden = 8 + rows as u16 * row_h;
            for row in first_hidden..19 {
                assert_eq!(app.on_click(11, row), Action::None);
                assert_eq!(app.hit_cursor, 0);
                assert_eq!(app.screen, Screen::Search);
            }
            assert_eq!(app.on_click(11, 8), Action::None);
            assert_eq!(app.hit_cursor, 2);
            assert_eq!(app.screen, Screen::Files);
        }
    }

    #[test]
    fn draw_keeps_selected_file_within_painted_rows() {
        for type_size in [TypeSize::Regular, TypeSize::Large] {
            let mut app = State::new();
            app.screen = Screen::Files;
            app.type_size = type_size;
            populate_files(&mut app);
            app.file_cursor = 99;
            let mut terminal = Terminal::new(TestBackend::new(40, 48)).unwrap();
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let rows = usize::from(app.places.file_rows.height / app.metrics.row_h);
            assert!(rows > 0);
            assert!(app.file_cursor >= app.file_scroll);
            assert!(app.file_cursor < app.file_scroll + rows);
        }
    }
}
