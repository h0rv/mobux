use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::surface::{Background, Canvas, CellSize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::state::{NodeKind, Places, Screen, State, CATALOG, ICONS, SETTINGS_COUNT};
use crate::theme::{slot_count, Metrics, Theme};

pub fn layout(area: Rect, theme: &Theme, metrics: Metrics) -> Places {
    let margin = 1u16;
    let nav_h = metrics.nav_h.max(1);
    let icon_h = metrics.icon_h();
    let nav_y = area.y.saturating_add(area.height).saturating_sub(nav_h);
    let header = if theme.show_clock {
        if area.height >= 28 {
            8
        } else if area.height >= 18 {
            4
        } else if area.height >= 10 {
            2
        } else {
            0
        }
    } else {
        0
    };
    let search_h = if theme.search_bar {
        search_height(area.height, header, nav_h, icon_h, metrics.search_h)
    } else {
        1
    };
    let show_dots = area.height >= 18;
    let dots_y = if show_dots {
        nav_y.saturating_sub(margin + 1)
    } else {
        nav_y
    };
    let above_dots = if theme.search_bar {
        let gap = if show_dots {
            margin + search_h + margin
        } else {
            margin + search_h
        };
        dots_y.saturating_sub(gap)
    } else if show_dots {
        dots_y.saturating_sub(margin)
    } else {
        dots_y
    };
    let icon_y = above_dots
        .saturating_sub(icon_h)
        .max(area.y.saturating_add(header));
    let icons = icon_slots(area.x, icon_y, area.width, metrics.icon_w, icon_h);
    let search = if theme.search_bar {
        let search_x = area.x.saturating_add(2.min(area.width / 4));
        let search_w = area
            .width
            .saturating_sub(search_x.saturating_sub(area.x).saturating_mul(2));
        Rect {
            x: search_x,
            y: icon_y.saturating_add(icon_h).saturating_add(margin),
            width: search_w,
            height: search_h,
        }
    } else {
        let search_w = area.width.saturating_sub(10).clamp(8, 24).min(area.width);
        Rect {
            x: area
                .x
                .saturating_add(area.width.saturating_sub(search_w) / 2),
            y: area.y.saturating_add(margin),
            width: search_w,
            height: search_h,
        }
    };
    let gear = icons[icons.len() - 1];

    let mut places = Places {
        icons,
        search,
        back: third(area.x, nav_y, area.width, nav_h, 0),
        home: third(area.x, nav_y, area.width, nav_h, 1),
        recent: third(area.x, nav_y, area.width, nav_h, 2),
        list: Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: nav_y.saturating_sub(area.y).saturating_sub(2),
        },
        files_sheet: Rect::default(),
        search_sheet: Rect::default(),
        file_rows: Rect::default(),
        file_query: Rect::default(),
        file_submit: Rect::default(),
        file_names: Rect::default(),
        file_text: Rect::default(),
        file_open: Rect::default(),
        file_use: Rect::default(),
        project: Rect::default(),
        workspace_agent: Rect::default(),
        workspace_review: Rect::default(),
        workspace_files: Rect::default(),
        workspace_terminal: Rect::default(),
        workspace_github: Rect::default(),
        workspace_apps: Rect::default(),
        workspace_live: Rect::default(),
        terminal_open: Rect::default(),
        notice: Rect::default(),
        session_dialog: Rect::default(),
        install_cancel: Rect::default(),
        install_confirm: Rect::default(),
        session_cancel: Rect::default(),
        session_stop: Rect::default(),
        names_chip: Rect::default(),
        text_chip: Rect::default(),
        dot_home: Rect::default(),
        dot_shell: Rect::default(),
        gear: Rect::default(),
        settings_sheet: Rect::default(),
    };
    let notice_width = area.width.saturating_sub(4).min(64);
    places.notice = Rect::new(
        area.x + area.width.saturating_sub(notice_width) / 2,
        nav_y.saturating_sub(3).max(area.y),
        notice_width,
        2.min(nav_y.saturating_sub(area.y)),
    );
    let dialog = floating(area, nav_y, 8, area.y + area.height.saturating_sub(8) / 2);
    places.session_dialog = dialog;
    let button_y = dialog.bottom().saturating_sub(4);
    let button_width = dialog.width.saturating_sub(5) / 2;
    places.session_cancel = Rect::new(dialog.x + 2, button_y, button_width, 3);
    places.session_stop = Rect::new(dialog.x + button_width + 3, button_y, button_width, 3);
    places.install_cancel = places.session_cancel;
    places.install_confirm = places.session_stop;
    let sheet_top = area.y.saturating_add(header).saturating_add(margin);
    places.files_sheet = floating(area, nav_y, nav_y.saturating_sub(area.y + 2), area.y + 1);
    places.search_sheet = places.files_sheet;
    let settings_h = 2u16.saturating_add(metrics.row_h.saturating_mul(SETTINGS_COUNT as u16));
    places.settings_sheet = floating(area, nav_y, settings_h, sheet_top);
    let body = places.list;
    places.terminal_open = Rect::new(
        body.x + 2.min(body.width / 4),
        body.y + body.height / 2,
        body.width.saturating_sub(4),
        3.min(body.height),
    );
    let project_y = area.y + if theme.show_clock { header + 1 } else { 4 };
    if area.height >= 28 && project_y.saturating_add(4) <= icon_y {
        let width = area.width.saturating_sub(4).min(48);
        places.project = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            project_y,
            width,
            3,
        );
    }
    let files = places.files_sheet;
    let footer_y = files.bottom().saturating_sub(4).max(files.y);
    let half = files.width.saturating_sub(3) / 2;
    let other = files.width.saturating_sub(3 + half);
    let compact = files.height < 28;
    places.file_open = Rect::new(files.x + 1, footer_y, half, 3).intersection(files);
    places.file_use = Rect::new(files.x + half + 2, footer_y, other, 3).intersection(files);
    let query_w = files.width.saturating_sub(12);
    places.file_query = Rect::new(
        files.x + 1,
        files.y + if compact { 2 } else { 4 },
        query_w,
        3,
    )
    .intersection(files);
    places.file_submit = Rect::new(
        files.x + query_w + 2,
        files.y + if compact { 2 } else { 4 },
        9,
        3,
    )
    .intersection(files);
    places.file_names = Rect::new(
        files.x + 1,
        files.y + if compact { 6 } else { 8 },
        half,
        if compact { 2 } else { 3 },
    )
    .intersection(files);
    places.file_text = Rect::new(
        files.x + half + 2,
        files.y + if compact { 6 } else { 8 },
        other,
        if compact { 2 } else { 3 },
    )
    .intersection(files);
    let rows_y = (files.y + if compact { 9 } else { 12 }).min(footer_y);
    places.file_rows = Rect::new(
        files.x + 1,
        rows_y,
        files.width.saturating_sub(2),
        footer_y.saturating_sub(rows_y + 1),
    );
    places.names_chip = places.file_names;
    places.text_chip = places.file_text;
    let dots_w = 5u16;
    let dots_x = area.x.saturating_add(area.width.saturating_sub(dots_w) / 2);
    let dot_h = if show_dots { 1 } else { 0 };
    places.dot_home = Rect {
        x: dots_x,
        y: dots_y,
        width: 1,
        height: dot_h,
    };
    places.dot_shell = Rect {
        x: dots_x.saturating_add(2),
        y: dots_y,
        width: if area.width >= 24 { 10 } else { 1 },
        height: if dot_h > 0 { 2 } else { 0 },
    };
    places.gear = gear;
    if theme.workspace && area.width >= 30 && area.height >= 34 {
        workspace_layout(area, nav_y, &mut places);
    }
    places
}

fn workspace_layout(area: Rect, nav_y: u16, places: &mut Places) {
    let width = area.width.saturating_sub(6).min(64);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y;
    let half = width.saturating_sub(2) / 2;
    places.icons = [Rect::default(); 5];
    places.dot_home = Rect::default();
    places.dot_shell = Rect::default();
    places.gear = Rect::new(x + width.saturating_sub(10), y + 1, 10, 3);
    let tall = area.height >= 44;
    places.project = Rect::new(
        x,
        y + if tall { 6 } else { 5 },
        width,
        if tall { 5 } else { 4 },
    );
    places.workspace_agent = Rect::new(
        x,
        y + if tall { 13 } else { 11 },
        width,
        if tall { 8 } else { 7 },
    );
    let tools_y = y + if tall { 23 } else { 19 };
    let tools_h = if tall { 5 } else { 3 };
    places.workspace_review = Rect::new(x, tools_y, half, tools_h);
    places.workspace_files = Rect::new(x + half + 2, tools_y, width - half - 2, tools_h);
    let quick_y = y + if tall { 29 } else { 23 };
    places.workspace_terminal = Rect::new(x, quick_y, half, 3);
    places.workspace_github = Rect::new(x + half + 2, quick_y, width - half - 2, 3);
    let apps_y = y + if tall { 34 } else { 27 };
    places.workspace_apps = Rect::new(x, apps_y, half, 3);
    places.workspace_live = Rect::new(x + half + 2, apps_y, width - half - 2, 3);
    places.search = if tall {
        Rect::new(x, nav_y.saturating_sub(5), width, 3)
    } else {
        Rect::new(x, y + 3, width.saturating_sub(12), 2)
    };
}

fn search_height(screen_h: u16, header: u16, nav_h: u16, icon_h: u16, wanted: u16) -> u16 {
    let chrome = header
        .saturating_add(nav_h)
        .saturating_add(icon_h)
        .saturating_add(6);
    let room = screen_h.saturating_sub(chrome);
    wanted.max(1).min(room.max(1))
}

fn floating(area: Rect, nav_y: u16, wanted: u16, top: u16) -> Rect {
    let inset = 2.min(area.width / 8);
    let width = area.width.saturating_sub(inset.saturating_mul(2)).min(72);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = top.min(nav_y.saturating_sub(6));
    let max_h = nav_y.saturating_sub(y).saturating_sub(1);
    Rect {
        x,
        y,
        width,
        height: wanted.min(max_h).max(6.min(max_h)),
    }
}

fn icon_slots(x: u16, y: u16, width: u16, icon_w: u16, height: u16) -> [Rect; 5] {
    let count = 5u16;
    let icon_w = icon_w.min(width / count);
    let total = icon_w.saturating_mul(count);
    let slack = width.saturating_sub(total);
    let gaps = count + 1;
    let gap = slack / gaps;
    let spare = slack - gap * gaps;
    let mut cursor = x.saturating_add(gap).saturating_add(spare / 2);
    let mut slots = [Rect::default(); 5];
    for slot in &mut slots {
        *slot = Rect {
            x: cursor,
            y,
            width: icon_w,
            height,
        };
        cursor = cursor.saturating_add(icon_w).saturating_add(gap);
    }
    slots
}

fn third(x: u16, y: u16, width: u16, height: u16, index: u16) -> Rect {
    let slice = width / 3;
    Rect {
        x: x.saturating_add(slice.saturating_mul(index)),
        y,
        width: if index == 2 {
            width.saturating_sub(slice.saturating_mul(2))
        } else {
            slice
        },
        height,
    }
}

pub fn paint(frame: &mut Frame<'_>, state: &State) {
    let _ = scene(frame, state, CellSize::default());
}

pub fn scene(frame: &mut Frame<'_>, state: &State, cells: CellSize) -> Option<Background> {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let mut canvas = Canvas::new(
        frame.buffer_mut(),
        cells,
        state.image_icons,
        state.surface_style,
    );
    let buf = &mut canvas;
    let theme = state.theme();
    if state.wallpaper {
        paint_wallpaper(buf, area, &theme);
    } else {
        buf.clear_background();
    }
    match state.screen {
        Screen::Home => paint_home(buf, area, state, &theme),
        Screen::Shell => paint_shell(buf, area, state, &theme),
        Screen::Files => paint_files(buf, state, &theme),
        Screen::Search => paint_search_card(buf, state, &theme),
        Screen::Settings => paint_settings(buf, state, &theme),
        Screen::Store => paint_store(buf, state, &theme),
        Screen::Recents => paint_recents(buf, area.height, state, &theme),
    }
    paint_nav(buf, area, state, &theme);
    if !state.notice.is_empty() && state.screen != Screen::Shell {
        let rect = state.places.notice;
        buf.rounded(rect, theme.pill.color(), 1.0);
        put_centered(
            buf,
            rect.x,
            rect.y,
            rect.width,
            &fit(&state.notice, usize::from(rect.width.saturating_sub(2))),
            Style::default()
                .fg(theme.pill_ink.color())
                .bg(theme.pill.color()),
        );
        if rect.height > 1 {
            put_centered(
                buf,
                rect.x,
                rect.y + 1,
                rect.width,
                "Tap to dismiss",
                Style::default()
                    .fg(theme.pill_ink.color())
                    .bg(theme.pill.color()),
            );
        }
    }
    paint_install(buf, state, &theme);
    paint_opening(buf, area, state, &theme);
    let background = canvas.finish();
    if (state.screen == Screen::Files && state.files_input_focused)
        || state.screen == Screen::Search
    {
        let rect = state.places.file_query;
        if rect.width > 2 && rect.height > 1 {
            let width = UnicodeWidthStr::width(state.query.as_str())
                .min(usize::from(rect.width - 2)) as u16;
            frame.set_cursor_position((rect.x + 1 + width, rect.y + 1));
        }
    }
    background
}

fn paint_wallpaper(buf: &mut Canvas<'_>, area: Rect, theme: &Theme) {
    let steps = area.height.saturating_mul(2).max(1);
    for row in 0..area.height {
        let y = area.y.saturating_add(row);
        let top = wall(row.saturating_mul(2), steps, theme);
        let bottom = wall(row.saturating_mul(2).saturating_add(1), steps, theme);
        buf.background_row(y, top, bottom);
    }
}

fn paint_home(buf: &mut Canvas<'_>, area: Rect, state: &State, theme: &Theme) {
    if !state.places.workspace_agent.is_empty() {
        paint_workspace(buf, area, state, theme);
        return;
    }
    if theme.show_clock {
        let (time, date) = clock(state);
        let clock_y = area.y + if area.height >= 28 { 3 } else { 0 };
        put_centered(
            buf,
            area.x,
            clock_y,
            area.width,
            &time,
            if state.wallpaper {
                ink_on(theme, clock_y, area.height).add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            },
        );
        if area.height >= 18 {
            let date_y = area
                .y
                .saturating_add(if area.height >= 28 && area.width >= 24 {
                    5
                } else {
                    2
                });
            put_centered(
                buf,
                area.x,
                date_y,
                area.width,
                &date,
                if state.wallpaper {
                    ink_on(theme, date_y, area.height)
                } else {
                    Style::default()
                },
            );
        }
    }
    if let Some(dock) = theme.dock {
        paint_dock(buf, state, dock.color(), area.height, theme);
    }
    let on_dock = theme.dock.map(|dock| dock.color());
    for (icon, rect) in ICONS.iter().zip(state.places.icons) {
        if crate::icons::visible_body(rect, state.places.back.y, state.places.recent.right())
            .is_none()
        {
            continue;
        }
        let app = match icon.target {
            crate::state::IconTarget::App(index) => state
                .registered_apps
                .get(index)
                .and_then(|id| CATALOG.iter().find(|app| app.id == *id)),
            _ => None,
        };
        paint_icon(
            buf,
            rect,
            IconJob {
                glyph: app
                    .and_then(|app| crate::icons::Glyph::from_catalog(&app.icon))
                    .unwrap_or(icon.display_glyph()),
                label: app.map_or(icon.display_label(), |app| app.label.as_str()),
                rgb: icon.rgb,
                behind: on_dock,
                blocky: state.surface_style == crate::theme::SurfaceStyle::Blocky,
                label_ink: if state.wallpaper {
                    theme.ink.color()
                } else {
                    Color::Reset
                },
            },
            area.height,
            theme,
        );
    }
    if state.places.project.height > 0 {
        let rect = state.places.project;
        buf.rounded(rect, theme.chip.color(), 1.5);
        put_centered(
            buf,
            rect.x,
            rect.y,
            rect.width,
            "Project",
            Style::default()
                .fg(theme.muted.color())
                .bg(theme.chip.color()),
        );
        put_centered(
            buf,
            rect.x,
            rect.y + 1,
            rect.width,
            &fit(
                &display_cwd(state),
                usize::from(rect.width.saturating_sub(2)),
            ),
            Style::default()
                .fg(theme.ink.color())
                .bg(theme.chip.color()),
        );
    }
    if state.places.project.height >= 3 {
        let rect = state.places.project;
        put_centered(
            buf,
            rect.x,
            rect.y + 2,
            rect.width,
            if state.session_backend {
                "Live sessions"
            } else {
                "Foreground apps"
            },
            Style::default()
                .fg(theme.muted.color())
                .bg(theme.chip.color()),
        );
    }
    paint_search(buf, state.places.search, area.height, theme);
    paint_dots(buf, state, area.height, theme);
}

fn workspace_text(
    buf: &mut Canvas<'_>,
    rect: Rect,
    dy: u16,
    text: &str,
    fg: Color,
    bg: Color,
    bold: bool,
) {
    let style = Style::default().fg(fg).bg(bg);
    put(
        buf,
        rect.x + 2,
        rect.y + dy,
        &fit(text, usize::from(rect.width.saturating_sub(4))),
        if bold {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        },
    );
}

fn paint_workspace(buf: &mut Canvas<'_>, area: Rect, state: &State, theme: &Theme) {
    let p = state.places;
    let bg = if state.wallpaper {
        theme.nav.color()
    } else {
        Color::Reset
    };
    let ink = theme.ink.color();
    let muted = theme.muted.color();
    let card = theme.card.color();
    let (accent, action_ink) = theme.action_colors();
    put(
        buf,
        p.project.x,
        area.y + 2,
        "Mobux",
        Style::default().fg(ink).bg(bg).add_modifier(Modifier::BOLD),
    );
    put(
        buf,
        p.project.x,
        area.y + 4,
        "Your workspace.",
        Style::default().fg(muted).bg(bg),
    );
    buf.rounded(p.gear, theme.chip.color(), 1.0);
    put_centered(
        buf,
        p.gear.x,
        p.gear.y + 1,
        p.gear.width,
        "Settings",
        Style::default().fg(ink).bg(theme.chip.color()),
    );

    buf.panel(p.project, card, theme.chip.color(), 1.25);
    workspace_text(buf, p.project, 1, "PROJECT  /  Change", muted, card, false);
    let project = display_cwd(state);
    workspace_text(
        buf,
        p.project,
        p.project.height.saturating_sub(2),
        &project,
        ink,
        card,
        true,
    );
    put(
        buf,
        p.project.x,
        p.workspace_agent.y.saturating_sub(1),
        "YOUR APPS",
        Style::default().fg(muted).bg(bg),
    );
    buf.panel(p.workspace_agent, card, theme.chip.color(), 1.25);
    let session = state.workspace_agent_session();
    let app = state.primary_app();
    let title = session
        .map(|session| session.spec.title.as_str())
        .or_else(|| app.map(|app| app.label.as_str()))
        .unwrap_or("Make it yours");
    workspace_text(buf, p.workspace_agent, 1, title, ink, card, true);
    workspace_text(
        buf,
        p.workspace_agent,
        if p.workspace_agent.height >= 8 { 3 } else { 2 },
        if session.is_some() {
            "Running in this project"
        } else {
            app.map_or("Add the apps you want to use.", |app| app.blurb.as_str())
        },
        muted,
        card,
        false,
    );
    let button = Rect::new(
        p.workspace_agent.x + 2,
        p.workspace_agent.bottom().saturating_sub(4),
        p.workspace_agent.width.saturating_sub(4),
        3,
    );
    buf.rounded(button, accent, 0.75);
    put_centered(
        buf,
        button.x,
        button.y + button.height / 2,
        button.width,
        if session.is_some() {
            "Resume"
        } else if app.is_some() {
            "Open app"
        } else {
            "Add an app"
        },
        Style::default()
            .fg(action_ink)
            .bg(accent)
            .add_modifier(Modifier::BOLD),
    );
    put(
        buf,
        p.project.x,
        p.workspace_review.y.saturating_sub(1),
        "TOOLS",
        Style::default().fg(muted).bg(bg),
    );
    for (rect, title, detail) in [
        (p.workspace_review, "Apps", "Switch"),
        (p.workspace_files, "Files", "Browse"),
    ] {
        buf.panel(rect, card, theme.chip.color(), 1.0);
        workspace_text(buf, rect, 1, title, ink, card, true);
        if rect.height > 3 {
            workspace_text(buf, rect, 3, detail, muted, card, false);
        }
    }
    for (rect, label) in [
        (p.workspace_terminal, "Terminal"),
        (p.workspace_github, "Search"),
        (p.workspace_apps, "Add apps"),
    ] {
        buf.panel(rect, card, theme.chip.color(), 1.0);
        put_centered(
            buf,
            rect.x,
            rect.y + 1,
            rect.width,
            label,
            Style::default().fg(ink).bg(card),
        );
    }
    if !p.workspace_live.is_empty() {
        let count = state.session_count();
        let label = if state.session_backend {
            format!("Open apps ({count})")
        } else {
            "History".to_string()
        };
        put(
            buf,
            p.workspace_live.x + 2,
            p.workspace_live.y + 1,
            &fit(
                &label,
                usize::from(p.workspace_live.width.saturating_sub(2)),
            ),
            Style::default().fg(ink).bg(bg),
        );
        put(
            buf,
            p.workspace_live.x + 2,
            p.workspace_live.y + 2,
            if state.session_backend {
                "Live sessions"
            } else {
                "Foreground apps"
            },
            Style::default().fg(muted).bg(bg),
        );
    }
    if !p.search.is_empty() {
        buf.panel(p.search, card, theme.chip.color(), 1.0);
        put(
            buf,
            p.search.x + 2,
            p.search.y + 1,
            if p.search.height >= 3 {
                "Search files"
            } else {
                "Search"
            },
            Style::default().fg(muted).bg(card),
        );
        put(
            buf,
            p.search.right().saturating_sub(3),
            p.search.y + 1,
            "/",
            Style::default().fg(muted).bg(card),
        );
    }
}

fn paint_dock(buf: &mut Canvas<'_>, state: &State, color: Color, _term_h: u16, _theme: &Theme) {
    let icons = state.places.icons;
    let x = icons[0].x.saturating_sub(1);
    let y = icons[0].y.saturating_sub(1);
    let right = icons[icons.len() - 1].right().saturating_add(1);
    let bottom = icons[0].bottom();
    buf.rounded(
        Rect {
            x,
            y,
            width: right
                .saturating_sub(x)
                .min(buf.area.width.saturating_sub(x)),
            height: bottom.saturating_sub(y),
        },
        color,
        2.0,
    );
}

struct IconJob<'a> {
    glyph: crate::icons::Glyph,
    label: &'a str,
    rgb: (u8, u8, u8),
    behind: Option<Color>,
    blocky: bool,
    label_ink: Color,
}

fn paint_icon(buf: &mut Canvas<'_>, rect: Rect, icon: IconJob<'_>, term_h: u16, theme: &Theme) {
    if rect.width < 3 || rect.height < 3 {
        return;
    }
    let body = crate::icons::body(rect);
    let backdrop = |row| icon.behind.unwrap_or_else(|| paper(row, term_h, theme));
    let shape = if icon.blocky {
        crate::icons::IconShape::Square
    } else if theme.search_bar {
        crate::icons::IconShape::Circle
    } else {
        crate::icons::IconShape::Rounded
    };
    if buf.native() {
        let asset = crate::icons::raster(icon.glyph, icon.rgb, shape);
        buf.icon(body, &asset);
    } else {
        crate::icons::paint(buf, body, icon.glyph, icon.rgb, shape, backdrop);
    }
    let label_y = body.bottom();
    if label_y < rect.bottom() {
        let label_bg = icon.behind.unwrap_or_else(|| paper(label_y, term_h, theme));
        put_centered(
            buf,
            rect.x,
            label_y,
            rect.width,
            &fit(icon.label, usize::from(rect.width)),
            Style::default().fg(icon.label_ink).bg(label_bg),
        );
    }
}

fn paint_round_fill(
    buf: &mut Canvas<'_>,
    rect: Rect,
    color: Color,
    _behind: impl Fn(u16) -> Color,
) {
    buf.rounded(rect, color, f32::INFINITY);
}

fn paper(row: u16, term_h: u16, theme: &Theme) -> Color {
    wall(
        row.saturating_mul(2).saturating_add(1),
        term_h.saturating_mul(2).max(1),
        theme,
    )
}

fn paint_search(buf: &mut Canvas<'_>, rect: Rect, term_h: u16, theme: &Theme) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let ink = Style::default()
        .fg(theme.pill_ink.color())
        .bg(theme.pill.color());
    if rect.height >= 3 && rect.width >= 4 {
        paint_round_fill(buf, rect, theme.pill.color(), |row| {
            paper(row, term_h, theme)
        });
    } else {
        fill(buf, rect, theme.pill.color());
    }
    let text_y = rect.y.saturating_add(rect.height / 2);
    put(
        buf,
        rect.x.saturating_add(2),
        text_y,
        theme.search_label,
        ink,
    );
}

fn paint_dots(buf: &mut Canvas<'_>, state: &State, term_h: u16, theme: &Theme) {
    if state.places.dot_home.height == 0 {
        return;
    }
    let (home_fg, shell_fg) = match state.screen {
        Screen::Shell => (theme.muted.color(), theme.ink.color()),
        _ => (theme.ink.color(), theme.muted.color()),
    };
    let bg = wall(
        state.places.dot_home.y.saturating_mul(2).saturating_add(1),
        term_h.saturating_mul(2).max(1),
        theme,
    );
    set(
        buf,
        state.places.dot_home.x,
        state.places.dot_home.y,
        "●",
        Style::default().fg(home_fg).bg(bg),
    );
    put_centered(
        buf,
        state.places.dot_shell.x,
        state.places.dot_shell.y,
        state.places.dot_shell.width,
        if state.places.dot_shell.width > 1 {
            "Terminal"
        } else {
            "●"
        },
        Style::default().fg(shell_fg).bg(bg),
    );
}

fn paint_shell(buf: &mut Canvas<'_>, _area: Rect, state: &State, theme: &Theme) {
    let body = state.places.list;
    buf.rounded(body, theme.card.color(), 2.0);
    let ink = Style::default()
        .fg(theme.ink.color())
        .bg(theme.card.color());
    put_centered(
        buf,
        body.x,
        body.y + body.height / 3,
        body.width,
        "Terminal",
        ink.add_modifier(Modifier::BOLD),
    );
    put_centered(
        buf,
        body.x,
        body.y + body.height / 3 + 2,
        body.width,
        &fit(&display_cwd(state), usize::from(body.width)),
        ink,
    );
    let button = state.places.terminal_open;
    buf.rounded(button, theme.pill.color(), f32::INFINITY);
    put_centered(
        buf,
        button.x,
        button.y + button.height / 2,
        button.width,
        "Open terminal",
        Style::default()
            .fg(theme.pill_ink.color())
            .bg(theme.pill.color()),
    );
}

fn paint_files(buf: &mut Canvas<'_>, state: &State, theme: &Theme) {
    let p = state.places;
    let sheet = p.files_sheet;
    let card = theme.card.color();
    let ink = theme.ink.color();
    let muted = theme.muted.color();
    buf.panel(sheet, card, theme.chip.color(), 1.25);
    workspace_text(
        buf,
        sheet,
        1,
        if state.screen == Screen::Search {
            "Find"
        } else {
            "Files"
        },
        ink,
        card,
        true,
    );
    if p.file_query.y > sheet.y + 2 {
        workspace_text(
            buf,
            sheet,
            2,
            &fit_tail(
                &display_cwd(state),
                usize::from(sheet.width.saturating_sub(4)),
            ),
            muted,
            card,
            false,
        );
    }
    let query = p.file_query;
    buf.panel(
        query,
        theme.chip.color(),
        if state.files_input_focused || state.screen == Screen::Search {
            theme.folder.color()
        } else {
            theme.selected.color()
        },
        0.75,
    );
    let shown = if state.query.is_empty() {
        if state.find_mode == crate::find::FindMode::Names {
            "Filter files…"
        } else {
            "Search contents…"
        }
    } else {
        &state.query
    };
    put(
        buf,
        query.x + 1,
        query.y + 1,
        &fit_tail(shown, usize::from(query.width.saturating_sub(2))),
        Style::default().fg(ink).bg(theme.chip.color()),
    );
    for (rect, label, selected) in [
        (
            p.file_names,
            "File names",
            state.find_mode == crate::find::FindMode::Names,
        ),
        (
            p.file_text,
            "Contents",
            state.find_mode == crate::find::FindMode::Text,
        ),
    ] {
        paint_chip(buf, rect, label, selected, theme);
    }
    buf.rounded(p.file_submit, theme.chip.color(), 0.75);
    put_centered(
        buf,
        p.file_submit.x,
        p.file_submit.y + p.file_submit.height / 2,
        p.file_submit.width,
        if state.find_mode == crate::find::FindMode::Text {
            "Find"
        } else if state.query.is_empty() {
            "Refresh"
        } else {
            "Clear"
        },
        Style::default().fg(ink).bg(theme.chip.color()),
    );
    let filtering = state.files_filtering();
    let info = if state.search_job.is_some() {
        "Searching…".to_string()
    } else if !filtering {
        "Tree · tap folders to expand".into()
    } else if state.find_mode == crate::find::FindMode::Names {
        format!("{} shown · 500-path index", state.hits.len())
    } else {
        format!("{} shown · up to 30", state.hits.len())
    };
    put(
        buf,
        p.file_rows.x,
        p.file_rows.y.saturating_sub(1),
        &fit(&info, usize::from(p.file_rows.width)),
        Style::default().fg(muted).bg(card),
    );
    let rows = usize::from(p.file_rows.height / state.metrics.row_h.max(1));
    let scroll = if filtering {
        state.hit_scroll
    } else {
        state.file_scroll
    };
    for offset in 0..rows {
        let index = scroll + offset;
        let y = p.file_rows.y + offset as u16 * state.metrics.row_h;
        let row = Rect::new(p.file_rows.x, y, p.file_rows.width, state.metrics.row_h);
        let selected = index
            == if filtering {
                state.hit_cursor
            } else {
                state.file_cursor
            };
        let bg = if selected {
            theme.selected.color()
        } else {
            card
        };
        if filtering {
            let Some(hit) = state.hits.get(index) else {
                break;
            };
            buf.rounded(row, bg, 0.5);
            put(
                buf,
                row.x + 1,
                y,
                &fit(&hit.title, usize::from(row.width.saturating_sub(2))),
                Style::default().fg(ink).bg(bg),
            );
            if row.height > 1 {
                put(
                    buf,
                    row.x + 1,
                    y + 1,
                    &fit(&hit.detail, usize::from(row.width.saturating_sub(2))),
                    Style::default().fg(ink).bg(bg),
                );
            }
        } else {
            let Some(node) = state.file_node(index) else {
                break;
            };
            let tree_row = state.tree.as_ref().and_then(|tree| tree.rows.get(index));
            let depth = tree_row.map_or(0, |row| row.depth).min(8) as u16;
            let marker = match node.kind {
                NodeKind::Parent => "↑",
                NodeKind::Dir => {
                    if tree_row.is_some_and(|row| row.expanded) {
                        "▾"
                    } else {
                        "▸"
                    }
                }
                NodeKind::File => " ",
            };
            buf.rounded(row, bg, 0.5);
            let x = (row.x + 1 + depth * 2).min(row.right().saturating_sub(3));
            let label = if node.kind == NodeKind::Parent {
                "Parent folder"
            } else {
                &node.name
            };
            put(
                buf,
                x,
                y + row.height / 2,
                &fit(
                    &format!("{marker} {label}"),
                    usize::from(row.right().saturating_sub(x + 1)),
                ),
                Style::default().fg(ink).bg(bg),
            );
        }
    }
    if filtering && state.hits.is_empty() && state.search_job.is_none() && p.file_rows.height > 1 {
        put(
            buf,
            p.file_rows.x + 1,
            p.file_rows.y + 1,
            if state.find_mode == crate::find::FindMode::Text && !state.text_search_ready {
                "Type a phrase, then Find"
            } else {
                "No matches"
            },
            Style::default().fg(muted).bg(card),
        );
    }
    for (rect, label, color, fg) in [
        (
            p.file_open,
            "Open",
            theme.pill.color(),
            theme.pill_ink.color(),
        ),
        (p.file_use, "Use as project", theme.chip.color(), ink),
    ] {
        buf.rounded(rect, color, 0.75);
        put_centered(
            buf,
            rect.x,
            rect.y + rect.height / 2,
            rect.width,
            &fit(label, usize::from(rect.width)),
            Style::default().fg(fg).bg(color),
        );
    }
}

fn paint_search_card(buf: &mut Canvas<'_>, state: &State, theme: &Theme) {
    paint_files(buf, state, theme);
}

fn paint_settings(buf: &mut Canvas<'_>, state: &State, theme: &Theme) {
    let sheet = state.places.settings_sheet;
    paint_card(buf, sheet, theme, "Settings", "");
    let rows = [
        ("Look", state.skin.label()),
        ("Target size", state.type_size.label()),
        ("Icon size", state.app_size.label()),
        ("Corners", state.surface_style.label()),
        (
            "Wallpaper",
            if state.wallpaper {
                "On"
            } else {
                "Terminal background"
            },
        ),
        ("Theme", state.palette.label()),
        ("Editor", state.preferred_editor.label()),
        ("Vim keys", if state.vim_keys { "On" } else { "Off" }),
    ];
    let visible = slot_count(sheet.height, 1, 1, state.metrics.row_h);
    for (offset, row) in rows
        .iter()
        .skip(state.settings_scroll)
        .take(visible)
        .enumerate()
    {
        let y = sheet.y.saturating_add(1).saturating_add(
            u16::try_from(offset)
                .unwrap_or(0)
                .saturating_mul(state.metrics.row_h),
        );
        let line = FatLine {
            title: row.0,
            detail: row.1,
            selected: state.settings_scroll + offset == state.settings_cursor,
            folder: false,
        };
        paint_fat_row(buf, sheet, y, state.metrics.row_h, theme, line);
    }
}

fn paint_recents(buf: &mut Canvas<'_>, term_h: u16, state: &State, theme: &Theme) {
    let list = state.places.list;
    put(
        buf,
        list.x,
        list.y,
        if state.session_backend {
            "Open apps"
        } else {
            "Recent apps (foreground)"
        },
        ink_on(theme, list.y, term_h).add_modifier(Modifier::BOLD),
    );
    if state.session_count() == 0 {
        put(
            buf,
            list.x,
            list.y + 2,
            if state.session_backend {
                "No open apps yet"
            } else {
                "No recent launches"
            },
            ink_on(theme, list.y + 2, term_h),
        );
    }
    let stride = state.metrics.recent_stride();
    let rows = slot_count(list.height, 1, 0, stride);
    if state.session_backend {
        for (offset, session) in state
            .live_sessions
            .iter()
            .skip(state.session_scroll)
            .take(rows)
            .enumerate()
        {
            let y = list.y + 1 + offset as u16 * stride;
            let width = list.width.saturating_sub(7);
            let card = Rect::new(list.x, y, width, state.metrics.row_h);
            let project = session
                .spec
                .cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "/".to_string());
            let title = format!("{} · {project}", session.spec.title);
            let detail = match session.status {
                crate::sessions::SessionStatus::Running => "Running · Resume".to_string(),
                crate::sessions::SessionStatus::Exited(code) => format!(
                    "Exited {} · Restart",
                    code.map_or_else(|| "?".to_string(), |value| value.to_string())
                ),
            };
            let bg = if state.session_cursor == state.session_scroll + offset {
                theme.selected.color()
            } else {
                theme.card.color()
            };
            buf.rounded(card, bg, 1.0);
            put(
                buf,
                card.x + 1,
                y,
                &fit(&title, usize::from(card.width.saturating_sub(2))),
                Style::default()
                    .fg(theme.ink.color())
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            );
            if card.height > 1 {
                put(
                    buf,
                    card.x + 1,
                    y + 1,
                    &fit(&detail, usize::from(card.width.saturating_sub(2))),
                    Style::default().fg(theme.muted.color()).bg(bg),
                );
            }
            if card.height > 2 {
                put(
                    buf,
                    card.x + 1,
                    y + 2,
                    &fit(
                        &crate::state::display_path(&session.spec.cwd),
                        usize::from(card.width.saturating_sub(2)),
                    ),
                    Style::default().fg(theme.muted.color()).bg(bg),
                );
            }
            let stop = Rect::new(list.right().saturating_sub(6), y, 6, state.metrics.row_h);
            buf.rounded(stop, theme.chip.color(), 1.0);
            put_centered(
                buf,
                stop.x,
                stop.y + stop.height.saturating_sub(1) / 2,
                stop.width,
                "Stop",
                Style::default()
                    .fg(theme.ink.color())
                    .bg(theme.chip.color()),
            );
        }
    } else {
        for (offset, session) in state
            .sessions
            .iter()
            .skip(state.session_scroll)
            .take(rows)
            .enumerate()
        {
            let y = list.y + 1 + offset as u16 * stride;
            let detail = crate::state::display_path(&session.cwd);
            paint_fat_row(
                buf,
                list,
                y,
                state.metrics.row_h,
                theme,
                FatLine {
                    title: &session.title,
                    detail: &detail,
                    selected: state.session_cursor == state.session_scroll + offset,
                    folder: false,
                },
            );
        }
    }
    if let Some(id) = &state.pending_stop {
        let rect = state.places.session_dialog;
        let title = state
            .live_sessions
            .iter()
            .find(|session| &session.id == id)
            .map_or("this session", |session| session.spec.title.as_str());
        buf.rounded(rect, theme.card.color(), 2.0);
        put_centered(
            buf,
            rect.x,
            rect.y + 1,
            rect.width,
            &format!("Stop {title}?"),
            Style::default()
                .fg(theme.ink.color())
                .bg(theme.card.color())
                .add_modifier(Modifier::BOLD),
        );
        put_centered(
            buf,
            rect.x,
            rect.y + 3,
            rect.width,
            "This ends the program.",
            Style::default()
                .fg(theme.muted.color())
                .bg(theme.card.color()),
        );
        for (button, label, bg, fg) in [
            (
                state.places.session_cancel,
                "Cancel",
                theme.chip.color(),
                theme.ink.color(),
            ),
            (
                state.places.session_stop,
                "Stop",
                state.palette.scheme().colors[8].color(),
                theme.nav.color(),
            ),
        ] {
            buf.rounded(button, bg, 1.0);
            put_centered(
                buf,
                button.x,
                button.y + button.height.saturating_sub(1) / 2,
                button.width,
                label,
                Style::default().fg(fg).bg(bg),
            );
        }
    }
}

fn paint_chip(buf: &mut Canvas<'_>, rect: Rect, label: &str, on: bool, theme: &Theme) {
    let (bg, fg) = if on {
        (theme.pill.color(), theme.pill_ink.color())
    } else {
        (theme.chip.color(), theme.ink.color())
    };
    fill(buf, rect, bg);
    put(
        buf,
        rect.x.saturating_add(1),
        rect.y + rect.height / 2,
        label,
        Style::default().fg(fg).bg(bg),
    );
}

struct FatLine<'a> {
    title: &'a str,
    detail: &'a str,
    selected: bool,
    folder: bool,
}

fn paint_fat_row(
    buf: &mut Canvas<'_>,
    sheet: Rect,
    y: u16,
    row_h: u16,
    theme: &Theme,
    line: FatLine<'_>,
) {
    let bg = if line.selected {
        theme.selected.color()
    } else {
        theme.card.color()
    };
    let width = sheet.width.saturating_sub(2);
    let row = Rect {
        x: sheet.x.saturating_add(1),
        y,
        width,
        height: row_h.max(1),
    };
    fill(buf, row, bg);
    let title_fg = if line.folder {
        theme.folder.color()
    } else {
        theme.ink.color()
    };
    put(
        buf,
        row.x,
        row.y,
        &fit(line.title, usize::from(width)),
        Style::default()
            .fg(title_fg)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    );
    if row_h > 1 {
        put(
            buf,
            row.x,
            row.y.saturating_add(1),
            &fit(line.detail, usize::from(width)),
            Style::default().fg(theme.muted.color()).bg(bg),
        );
    }
}

fn paint_store(buf: &mut Canvas<'_>, state: &State, theme: &Theme) {
    let sheet = state.places.list;
    paint_card(buf, sheet, theme, "Apps", "curated terminal apps");
    let rows = slot_count(sheet.height, 1, 1, state.metrics.row_h);
    for (offset, item) in CATALOG
        .iter()
        .skip(state.store_scroll)
        .take(rows)
        .enumerate()
    {
        let index = state.store_scroll + offset;
        let y = sheet.y + 1 + offset as u16 * state.metrics.row_h;
        let row = Rect::new(
            sheet.x + 1,
            y,
            sheet.width.saturating_sub(2),
            state.metrics.row_h,
        );
        let bg = if index == state.store_cursor {
            theme.selected.color()
        } else {
            theme.card.color()
        };
        buf.rounded(row, bg, 1.0);
        let icon = Rect::new(row.x + 1, y, 4.min(row.width), row.height);
        let glyph =
            crate::icons::Glyph::from_catalog(&item.icon).unwrap_or(crate::icons::Glyph::Terminal);
        let shape = if state.surface_style == crate::theme::SurfaceStyle::Blocky {
            crate::icons::IconShape::Square
        } else {
            crate::icons::IconShape::Circle
        };
        let accent = if item.id == "opencode" {
            (72, 62, 100)
        } else {
            (65, 90, 145)
        };
        if buf.native() {
            buf.icon(icon, &crate::icons::raster(glyph, accent, shape));
        } else {
            crate::icons::paint(buf, icon, glyph, accent, shape, |_| bg);
        }
        let x = row.x.saturating_add(6).min(row.right());
        let width = usize::from(row.right().saturating_sub(x).saturating_sub(7));
        let installed = state.installed.get(index).copied().unwrap_or(false);
        put(
            buf,
            x,
            y,
            &fit(&item.label, width),
            Style::default()
                .fg(theme.ink.color())
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        );
        if row.height > 1 {
            let detail = format!(
                "{} · {}",
                item.blurb,
                if !state.registered_apps.contains(&item.id) {
                    "Add to workspace"
                } else if installed {
                    "Open"
                } else {
                    "Setup required"
                }
            );
            put(
                buf,
                x,
                y + 1,
                &fit(&detail, width),
                Style::default().fg(theme.muted.color()).bg(bg),
            );
        }
        let toggle = Rect::new(row.right().saturating_sub(7), y, 7, row.height);
        put_centered(
            buf,
            toggle.x,
            toggle.y + toggle.height / 2,
            toggle.width,
            if state.registered_apps.contains(&item.id) {
                "Remove"
            } else {
                "Add"
            },
            Style::default().fg(theme.ink.color()).bg(bg),
        );
    }
}

fn paint_card(buf: &mut Canvas<'_>, rect: Rect, theme: &Theme, title: &str, subtitle: &str) {
    buf.rounded(rect, theme.card.color(), 2.0);
    let title_style = Style::default()
        .fg(theme.ink.color())
        .bg(theme.card.color())
        .add_modifier(Modifier::BOLD);
    put(
        buf,
        rect.x.saturating_add(2),
        rect.y,
        &fit(
            &format!("{title}  {subtitle}"),
            usize::from(rect.width.saturating_sub(2)),
        ),
        title_style,
    );
}

fn paint_nav(buf: &mut Canvas<'_>, area: Rect, state: &State, theme: &Theme) {
    let y = state.places.back.y;
    let height = state.places.back.height.max(1);
    fill(
        buf,
        Rect {
            x: area.x,
            y,
            width: area.width,
            height,
        },
        theme.nav.color(),
    );
    let text_y = y.saturating_add(height.saturating_sub(1) / 2);
    let style = Style::default()
        .fg(theme.nav_ink.color())
        .bg(theme.nav.color());
    if theme.home_indicator {
        let pill = 8.min(area.width);
        let x = area.x.saturating_add(area.width.saturating_sub(pill) / 2);
        for step in 0..pill {
            set(buf, x.saturating_add(step), text_y, "▄", style);
        }
        return;
    }
    put_centered(
        buf,
        state.places.back.x,
        text_y,
        state.places.back.width,
        theme.marks[0],
        style,
    );
    put_centered(
        buf,
        state.places.home.x,
        text_y,
        state.places.home.width,
        theme.marks[1],
        style,
    );
    put_centered(
        buf,
        state.places.recent.x,
        text_y,
        state.places.recent.width,
        theme.marks[2],
        style,
    );
}

fn paint_install(buf: &mut Canvas<'_>, state: &State, theme: &Theme) {
    let Some(plan) = &state.pending_install else {
        return;
    };
    let rect = state.places.session_dialog;
    buf.panel(rect, theme.card.color(), theme.chip.color(), 1.25);
    workspace_text(
        buf,
        rect,
        1,
        &format!("Install {}?", plan.title),
        theme.ink.color(),
        theme.card.color(),
        true,
    );
    workspace_text(
        buf,
        rect,
        3,
        &plan.command(),
        theme.muted.color(),
        theme.card.color(),
        false,
    );
    for (button, label, color, ink) in [
        (
            state.places.install_cancel,
            "Cancel",
            theme.chip.color(),
            theme.ink.color(),
        ),
        (
            state.places.install_confirm,
            "Install",
            theme.action_colors().0,
            theme.action_colors().1,
        ),
    ] {
        buf.rounded(button, color, 0.75);
        put_centered(
            buf,
            button.x,
            button.y,
            button.width,
            label,
            Style::default().fg(ink).bg(color),
        );
    }
}

fn paint_opening(buf: &mut Canvas<'_>, area: Rect, state: &State, theme: &Theme) {
    let Some(title) = &state.opening else {
        return;
    };
    let width = area.width.saturating_sub(6).max(8).min(area.width);
    let height = 5.min(area.height).max(3);
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area
        .y
        .saturating_add(area.height.saturating_sub(height) / 2);
    let rect = Rect {
        x,
        y,
        width,
        height,
    };
    buf.rounded(rect, theme.card.color(), 2.0);
    let ink = Style::default()
        .fg(theme.ink.color())
        .bg(theme.card.color())
        .add_modifier(Modifier::BOLD);
    put_centered(buf, x, y.saturating_add(1), width, title, ink);
    put_centered(
        buf,
        x,
        y.saturating_add(3.min(height.saturating_sub(1))),
        width,
        "opening",
        Style::default()
            .fg(theme.muted.color())
            .bg(theme.card.color()),
    );
}

fn ink_on(theme: &Theme, y: u16, term_h: u16) -> Style {
    Style::default().fg(theme.ink.color()).bg(wall(
        y.saturating_mul(2).saturating_add(1),
        term_h.saturating_mul(2).max(1),
        theme,
    ))
}

fn wall(step: u16, steps: u16, theme: &Theme) -> Color {
    let stops = theme.stops;
    if stops.is_empty() {
        return theme.card.color();
    }
    let steps = steps.max(1);
    let t = u32::from(step.min(steps - 1)) * 1000 / u32::from(steps - 1).max(1);
    let t = u16::try_from(t).unwrap_or(1000);
    let mut index = 0;
    while index + 1 < stops.len() && t > stops[index + 1].0 {
        index += 1;
    }
    let left = stops[index];
    let right = stops[(index + 1).min(stops.len() - 1)];
    let span = right.0.saturating_sub(left.0).max(1);
    let along = t.saturating_sub(left.0).min(span);
    Color::Rgb(
        lerp(left.1, right.1, along, span),
        lerp(left.2, right.2, along, span),
        lerp(left.3, right.3, along, span),
    )
}

fn lerp(from: u8, to: u8, num: u16, den: u16) -> u8 {
    let den = u32::from(den.max(1));
    let num = u32::from(num.min(den as u16));
    let from = u32::from(from);
    let to = u32::from(to);
    let value = if to >= from {
        from + (to - from) * num / den
    } else {
        from - (from - to) * num / den
    };
    u8::try_from(value).unwrap_or(from as u8)
}

fn fill(buf: &mut Canvas<'_>, rect: Rect, color: Color) {
    let rect = rect.intersection(buf.area);
    buf.fill(rect, color);
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            set(buf, x, y, " ", Style::default().bg(color));
        }
    }
}

fn put(buf: &mut Canvas<'_>, x: u16, y: u16, text: &str, style: Style) {
    if y < buf.area.y || y >= buf.area.bottom() || x < buf.area.x || x >= buf.area.right() {
        return;
    }
    let width = usize::from(buf.area.right() - x);
    buf.set_stringn(x, y, text, width, style);
}

fn put_centered(buf: &mut Canvas<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let text = fit(text, usize::from(width));
    let len = u16::try_from(UnicodeWidthStr::width(text.as_str())).unwrap_or(width);
    let offset = width.saturating_sub(len) / 2;
    put(buf, x.saturating_add(offset), y, &text, style);
}

fn set(buf: &mut Canvas<'_>, x: u16, y: u16, symbol: &str, style: Style) {
    buf.block(x, y, symbol, &style);
    if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
        cell.set_symbol(symbol).set_style(style);
    }
}

fn fit_tail(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = Vec::new();
    let mut used = 1;
    for part in text.graphemes(true).rev() {
        let w = UnicodeWidthStr::width(part);
        if used + w > width {
            break;
        }
        used += w;
        result.push(part);
    }
    result.reverse();
    format!("…{}", result.concat())
}

fn fit(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    let suffix = if width >= 2 { "…" } else { "" };
    let limit = width.saturating_sub(UnicodeWidthStr::width(suffix));
    let mut out = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let columns = UnicodeWidthStr::width(grapheme);
        if used + columns > limit {
            break;
        }
        out.push_str(grapheme);
        used += columns;
    }
    out.push_str(suffix);
    out
}

fn display_cwd(state: &State) -> String {
    crate::state::display_path(&state.cwd)
}

fn clock(state: &State) -> (String, String) {
    if let Some(fixed) = &state.clock_override {
        return fixed.clone();
    }
    let now = chrono::Local::now();
    let day = now.format("%d").to_string();
    let day = day.trim_start_matches('0');
    let date = format!("{}, {} {day}", now.format("%a"), now.format("%b"));
    (now.format("%H:%M").to_string(), date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::state::{Screen, State};
    use crate::theme::{metrics, AppSize, SkinId, TypeSize};

    fn place(state: &mut State, width: u16, height: u16) {
        let area = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        state.metrics = metrics(state.type_size, state.app_size, area);
        state.places = layout(area, &state.theme(), state.metrics);
    }

    fn frame_text(width: u16, height: u16, state: &State) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| paint(frame, state)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if let Some(cell) = buf.cell(Position::new(x, y)) {
                    text.push_str(cell.symbol());
                }
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn small_viewports_clip_every_screen_without_panics_or_fake_pipe_corners() {
        for native in [false, true] {
            for width in 1..=16 {
                for height in 1..=8 {
                    for screen in [
                        Screen::Home,
                        Screen::Shell,
                        Screen::Files,
                        Screen::Store,
                        Screen::Search,
                        Screen::Settings,
                        Screen::Recents,
                    ] {
                        let mut state = State::new();
                        state.image_icons = native;
                        state.screen = screen;
                        place(&mut state, width, height);
                        let text = frame_text(width, height, &state);
                        assert!(!text.chars().any(|ch| matches!(ch, '╭' | '╮' | '╰' | '╯')));
                    }
                }
            }
        }
    }

    #[test]
    fn browser_targets_are_separate_and_use_the_available_height() {
        for width in [30, 40, 80, 120] {
            for height in [20, 24, 32, 40, 48, 60] {
                let mut state = State::new();
                place(&mut state, width, height);
                let p = state.places;
                let controls = [
                    p.file_query,
                    p.file_submit,
                    p.file_names,
                    p.file_text,
                    p.file_open,
                    p.file_use,
                    p.file_rows,
                ];
                assert!(p.files_sheet.bottom() <= p.back.y);
                assert_eq!(p.files_sheet.width.min(72), p.files_sheet.width);
                assert!(p.file_rows.height >= 2, "{width}x{height}");
                for (i, rect) in controls.iter().enumerate() {
                    assert_eq!(rect.intersection(p.files_sheet), *rect);
                    for other in controls.iter().skip(i + 1) {
                        assert!(
                            rect.intersection(*other).is_empty(),
                            "{width}x{height}: {rect:?} {other:?}"
                        );
                    }
                }
            }
        }
        assert_eq!(
            fit_tail("a/very/long/project/needle.rs", 12),
            "…t/needle.rs"
        );
    }

    #[test]
    fn minimal_workspace_has_readable_controls_and_nonoverlapping_touch_targets() {
        for width in [30, 40, 80, 120] {
            for height in [34, 36, 40, 43, 44, 48, 60] {
                let mut state = State::new();
                place(&mut state, width, height);
                let p = state.places;
                let controls = [
                    p.project,
                    p.gear,
                    p.workspace_agent,
                    p.workspace_review,
                    p.workspace_files,
                    p.workspace_terminal,
                    p.workspace_github,
                    p.workspace_apps,
                    p.workspace_live,
                    p.search,
                ];
                for (i, rect) in controls.iter().enumerate() {
                    assert!(!rect.is_empty());
                    assert!(
                        rect.right() <= width && rect.bottom() <= p.back.y,
                        "{width}x{height}: {rect:?}"
                    );
                    for other in controls.iter().skip(i + 1) {
                        assert!(
                            rect.intersection(*other).is_empty(),
                            "{width}x{height}: {rect:?} overlaps {other:?}"
                        );
                    }
                }
                let text = frame_text(width, height, &state);
                for label in [
                    "Mobux",
                    "Make it yours",
                    "Apps",
                    "Files",
                    "Terminal",
                    "Search",
                ] {
                    assert!(text.contains(label), "{width}x{height} missing {label}");
                }
            }
        }
    }

    #[test]
    fn android_home_shows_added_apps_and_core_controls() {
        let mut state = State::new();
        state.skin = SkinId::Android;
        state.registered_apps.push("lazygit".into());
        place(&mut state, 40, 32);
        let text = frame_text(40, 32, &state);
        assert!(text.contains("lazygit"), "{text}");
        assert!(text.contains("Terminal"), "{text}");
        assert!(text.contains("Files"), "{text}");
        assert!(text.contains("Apps"), "{text}");
        assert!(text.contains("Search"), "{text}");
        assert!(text.contains("Settings"), "{text}");
        assert!(text.contains('◀'), "{text}");
        assert!(text.contains('▀'), "{text}");
    }

    #[test]
    fn shell_uses_the_bottom_nav_to_go_home() {
        let mut state = State::new();
        state.screen = Screen::Shell;
        place(&mut state, 40, 32);
        let text = frame_text(40, 32, &state);
        assert!(text.contains("Back"), "{text}");
        assert!(text.contains("Home"), "{text}");
        assert!(text.contains("Apps"), "{text}");
    }

    #[test]
    fn iphone_search_sits_above_the_dock() {
        let mut state = State::new();
        state.skin = SkinId::Iphone;
        place(&mut state, 40, 32);
        assert!(state.places.search.y < state.places.icons[0].y);
        assert!(state.places.icons[0].bottom() <= state.places.back.y);
        assert_eq!(state.places.gear, state.places.icons[4]);
    }

    #[test]
    fn large_type_keeps_icons_clear_of_the_clock() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 32,
        };
        let metrics = metrics(TypeSize::Large, AppSize::Regular, area);
        let places = layout(area, &SkinId::Android.theme(), metrics);
        assert!(places.icons[0].y >= 4);
        assert!(places.icons[0].bottom() <= places.search.y);
        assert!(places.back.y >= places.search.bottom());
    }

    #[test]
    fn iphone_home_hides_the_clock_and_uses_a_dock() {
        let mut state = State::new();
        state.skin = SkinId::Iphone;
        state.clock_override = Some(("12:00".to_string(), "Fri, Sep 22".to_string()));
        place(&mut state, 40, 32);
        let text = frame_text(40, 32, &state);
        assert!(text.contains("Search"), "{text}");
        assert!(text.contains('●'), "{text}");
        assert!(!text.contains("12:00"), "{text}");
    }

    #[test]
    fn a_short_screen_keeps_the_nav_under_the_icons() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 12,
        };
        let metrics = metrics(TypeSize::Regular, AppSize::Regular, area);
        let places = layout(area, &SkinId::Android.theme(), metrics);
        assert_eq!(places.back.bottom(), 12);
        assert!(places.icons[0].bottom() <= places.search.y);
        assert!(places.search.bottom() <= places.back.y);
        assert!(places.icons[4].right() <= 40);
    }

    #[test]
    fn icon_slots_stay_inside_the_screen() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 32,
        };
        let metrics = metrics(TypeSize::Regular, AppSize::Regular, area);
        let places = layout(area, &SkinId::Android.theme(), metrics);
        for rect in places.icons {
            assert!(rect.right() <= 40, "{rect:?}");
            assert!(rect.bottom() <= 32, "{rect:?}");
            assert!(rect.width >= 5);
        }
        assert!(places.icons[0].x < places.icons[1].x);
        assert!(places.search.y > places.icons[0].y);
        assert!(places.back.y > places.search.y);
        assert!(places.gear.right() <= 40);
    }

    #[test]
    fn large_apps_are_wider_on_a_wide_screen() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 40,
        };
        let theme = SkinId::Android.theme();
        let regular = layout(
            area,
            &theme,
            metrics(TypeSize::Regular, AppSize::Regular, area),
        );
        let large = layout(
            area,
            &theme,
            metrics(TypeSize::Regular, AppSize::Large, area),
        );
        assert!(large.icons[0].width > regular.icons[0].width);
    }
}
