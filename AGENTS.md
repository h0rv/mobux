# Mobux

## Product direction

Mobux is a touch-first terminal workspace for phones. Push what modern terminals can display while keeping the interface calm and useful. The product is mobile interaction, not just a curated launcher. It remains a general-purpose terminal environment.

Minimal is the default layout. Use restrained surfaces, readable labels, generous spacing, and predictable actions. Android and iPhone are alternate layouts. Preserve dark and light palettes independently of layout. Do not build a chat sidebar, fake activity feeds, or decorative controls without actions.

No optional apps are preselected or bundled. Users add and remove catalog entries; the registry is saved separately from running sessions. Do not hardcode a preferred tool stack into Home. Herdr is an agent manager, not a coding agent. It manages agents within an app. Mobux manages the surrounding programs. Do not rebuild Herdr's agent session dashboard. Keep real app names, including OpenCode and Hunk. Hunk launches as `hunk diff`. Show attention or approval states only when a reliable integration reports them. A running process does not mean an agent is working.

Herdr's official installer rejects native Termux. OpenCode and Hunk have no verified native Android path here. GitHub CLI and micro have documented Termux packages. Package availability, graphics support, and touchscreen usability are separate claims. Never infer all three from one.

Termux is the initial mobile host. Keep Mobux an orthogonal terminal binary over tmux, not a replacement Android environment or native app. A native app can be considered later for a demonstrated need. Local development and SSH to another host are both valid. There is no connection manager, Android document-provider integration, or cross-machine browser yet.

## Implementation

Rust, Ratatui, and tiny-skia draw the interface. Unsafe code is forbidden.

- `src/state.rs` owns navigation, project context, launch plans, and interaction. Rectangles from the last frame are the source of truth for taps. Scrolling and row hit tests must match painting.
- `src/paint.rs` owns layout and painting through `src/surface.rs`. Smooth curves are filled geometry, never pipe characters. Minimal uses thin pixel borders and ordinary native text. Keep compact layouts usable and controls separate from navigation.
- `src/theme.rs` owns layout choices and target sizes. `src/palette.rs` validates and caches portable Base16-style JSON. Always use `state.theme()` for a selected palette. The terminal controls font size. Layout settings change hit targets, not the font.
- `src/graphics.rs` uploads a compressed background below text with Kitty graphics at `z=-1`. It detects Kitty and Ghostty outside multiplexers. `MOBUX_GRAPHICS=kitty` opts in and `MOBUX_GRAPHICS=cells` selects the fallback. Do not send capability queries that might become typed input. Delete only our image ID.
- `src/sessions.rs` wraps tmux 3.4 or newer on a private socket. It owns discovery, attach, restart, and stop. Generation-specific UUID names prevent numeric ID reuse. Mutations use exact session-qualified targets. Never kill unrelated sessions or a user's default server. Keep ownership metadata, readiness checks, locking, and working-directory guards.
- `src/main.rs` owns terminal setup, the event loop, and program handoff. Restore terminal modes before attaching a child. Keep the workspace visible through session preflight and clear graphics only at ownership transfer. Batch return-to-workspace setup and its complete first frame in one synchronized update. Do not reintroduce an opening-overlay/early-clear flash. Never redraw repeatedly while idle. Ignore pointer motion for repainting. Do not reintroduce a fake shell parser or blocking command capture.
- `src/config.rs` validates versioned settings and history. Save privately and atomically. Preserve invalid files and retry temporary save failures. Do not restore an old cwd over the startup directory.
- `src/install.rs` contains audited host-specific package plans. Install only after explicit confirmation. Use literal argv, not shell interpolation. App `requires` commands and `termux_package` recipes are declared in `catalog/apps.json`. micro is the preferred editor for new configurations, but it is installed only after explicit confirmation. Do not override existing editor preferences. No remote scripts, automatic sudo, or guessed Android packages. Tests must never run a real package manager.
- `src/host.rs` resolves executable paths, including a Termux PREFIX. Do not assume Android has `/bin/sh`.
- `src/catalog.rs` reads `catalog/apps.json`. Keep app identity, command arguments, source URLs, and compatibility notes accurate. The GitHub entry opens a CLI overview in a shell rather than disappearing after printing help.
- `src/tree.rs` owns lazy directory expansion and cached tree projection. Failed reads or symlink ancestor cycles preserve the view. Nested expansion is limited to 32 levels and 2,000 projected rows. The initial root listing is not truncated.
- `src/find.rs` implements bounded filename discovery and cancellable background ripgrep jobs. Pass patterns with `-e`. Content searches have a two-second deadline and 512 KiB output bound. Editing cancels the job; explicit submission starts another. Filename discovery is capped at 500 paths and six levels.
- `src/icons.rs` caches bundled masks. Source assets and licenses are in `assets/THIRD_PARTY.md`. Do not rasterize icons every frame.
- `src/shot.rs` creates previews using the same background as live rendering. Preview text uses bundled JetBrains Mono; fallback tests retain the bitmap renderer. `src/bin/bench.rs` measures CPU drawing and encoding, not terminal display.

## Interaction rules

Every action needs a tap path. Do not require horizontal dragging or keyboard chords. Native Termux does not provide every desktop mouse gesture. Cell dimensions are not physical touch dimensions. Validate targets with the eventual host and software keyboard before claiming accessibility or phone compatibility.

Switching and stopping are distinct. Home, Apps, and quitting the UI must leave managed processes alive. Exited apps require explicit Restart. Stop requires confirmation pinned to a generation-specific ID. Recheck the working directory before restarting and keep execution guarded against cwd disappearance. Literal arguments and zero-argument binaries must remain literal.

Herdr stays an optional app. One persistent Herdr instance is a valid workflow. Do not add an agent framework or a custom terminal emulator to solve app switching. Use existing package managers for confirmed installs rather than building a new one.

## Build and verify

```sh
mise install
mise run dev
mise run fmt
mise run check
mise run test-ui
```

`check` runs formatting checks, Clippy with warnings denied, tests, and generated previews. The optimized development profile is intentional. Per-cell pixel fills and unoptimized graphics dependencies previously caused visible lag. See `PERFORMANCE.md`.

The public repository is `h0rv/mobux`. GitHub Actions checks Linux and publishes Android 9+, Linux, and macOS snapshots for ARM64 and x86_64. Android binaries are cross-compiled. `scripts/install-termux.sh` installs published snapshots with checksums; `SETUP.md` also covers on-phone source builds. Cross-compilation does not establish physical phone compatibility.

`test-ui` requires tmux, ripgrep, and Expect. It covers nested tree expansion, fuzzy filtering, focused input, and content search as well as app switching and confirmed installation. Its servers, files, and fake package manager are private test fixtures. Never point tests at the default tmux socket. Include a native supported-terminal check after visual work. Passing unit tests does not prove that a layout looks good.

## Small follow-ups

Saved commands may be useful as explicit project-scoped entries with review before execution. Do not copy shell history automatically or add an LLM command-generation subsystem. Prefer optional existing tools when they solve the task.

## Previews and limits

`shots/home-mobile.png` and `shots/home-desktop.png` show Minimal. `shots/light-mobile.png` and `shots/one-dark-mobile.png` show alternate palettes. `shots/iphone-mobile.png` shows the alternate layout. Other files cover cards, live-app fixtures, stop confirmation, and rendering options.

Generated pictures use fixed data and are not live phone captures. Actual Ghostty captures belong under `shots/live/`. Document what was tested. Do not claim Android, software keyboard, screen reader, nested Herdr, or actual package installation verification without evidence. Android can kill background processes even when tmux persistence works on a desktop host.
