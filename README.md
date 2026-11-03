# Mobux

Mobux is a touch-first terminal workspace for phones. Choose a project, work with an agent, review changes, and switch to a shell without closing the other programs.

The goal is a terminal that is comfortable to use with a thumb. It should remain useful for ordinary terminal work. It is not a chat client, a replacement shell, or a new agent framework.

## Install in Termux

Mobux is experimental. Android binaries are built in CI, but physical phone support is not yet verified. Start with the [Termux installation and source-build guide](SETUP.md).

```sh
pkg install tmux ripgrep curl jq tar coreutils
curl -fLo install-mobux.sh https://raw.githubusercontent.com/h0rv/mobux/main/scripts/install-termux.sh
bash install-mobux.sh
mobux
```

The installer needs a successfully published [Android snapshot](https://github.com/h0rv/mobux/releases). Inspect the downloaded script before running it. You can build from source if a binary is not available.

Termux hosts the environment. tmux keeps programs alive. Mobux provides navigation and switching. Editors and agents stay independent optional programs. No account or hosted Mobux service is required.

## The workspace

Minimal is the default layout. It uses quiet surfaces, readable labels, and large controls. Android and iPhone layouts remain available in Settings. Layout and color are independent. Gruvbox Material Dark, One Dark Pro, and Gruvbox Material Light are bundled.

The home screen provides project selection and access to your added apps. No optional tools are preselected, installed, or bundled as a default stack. Add catalog entries to the workspace and remove them without stopping their processes. These choices are saved. Terminal opens the user's shell in one tap. A running primary app in the current project has a Resume action with its own name.

Herdr is an agent manager, not a coding agent. It can manage agents inside a Mobux app. Mobux manages the surrounding programs. We do not need to duplicate Herdr's agent dashboard or introduce a sidebar of chat sessions. Herdr's official installer rejects native Termux. Use a supported host for Herdr. Native Android support for OpenCode and Hunk remains unverified.

Files has a lazy expandable tree and an inline filter. Filename search uses fuzzy matching. Contents search runs ripgrep in the background after Find or Enter. Editing cancels the old search. Clearing the filter restores the expanded tree. Files and results use the available height, with a compact layout for shorter screens.

Tap a folder to expand it. Open enters the selected folder or opens a file. Use as project changes the launch context for new apps. Resuming an existing app retains that app's original directory. Automatic editor selection uses added editors rather than guessing a tool stack.

## Persistent apps

Install tmux 3.4 or newer to keep programs alive. Mobux uses a private tmux server. Its two bottom rows provide wide Home and Apps controls while another program owns the terminal. Returning home, switching apps, or quitting Mobux leaves those processes alive.

Open apps distinguishes Resume from Restart. An exited program never restarts automatically. Stop requires confirmation. Session identities are unique across server restarts so a stale confirmation cannot stop a replacement process.

Without a supported tmux, or inside an enclosing tmux, apps run in the foreground. `MOBUX_SESSIONS=foreground` selects this mode explicitly. It does not provide persistent switching. The private backend does not enable graphics passthrough for child apps.

Preferences and recent launch history are saved atomically in `~/.config/mobux/state.json`, or under `XDG_CONFIG_HOME`. `MOBUX_CONFIG_DIR` overrides the directory. Invalid configurations are reported and preserved. Temporary save failures are retried. The project directory is not restored over a directory passed at startup.

## Apps and setup

The bundled catalog is `catalog/apps.json`. It includes Herdr, OpenCode, Hunk, GitHub CLI, and editors. Inclusion does not mean touchscreen use has been tested.

On Termux, add GitHub CLI or micro and open its missing entry to request a confirmed installation through `pkg`. The dialog shows the command before execution. No remote scripts or guessed Android packages are used. Package installation itself has not been tested on a phone. Automated tests use a fake package manager and install nothing.

GitHub opens `gh status` in a persistent shell. From there, use `gh pr list`, `gh issue list`, or `gh auth login`. Mobux does not store GitHub credentials or automatically publish or merge changes.

For a local Termux starting point, see [SETUP.md](SETUP.md). Native Termux's current graphics capabilities are a separate constraint from package availability.

## Graphics and accessibility

Rust, Ratatui, and tiny-skia draw the interface. Supported terminals receive a pixel background through Kitty graphics with native terminal text above it. Live surfaces and generated previews use the same geometry. The terminal controls the font.

Kitty and Ghostty enable graphics automatically outside terminal multiplexers. Use `MOBUX_GRAPHICS=kitty` on another compatible host or `MOBUX_GRAPHICS=cells` for the development fallback. Emitting the protocol does not establish support in a phone terminal.

Every action needs a tap path. Labels should be readable and controls forgiving. Keyboard shortcuts remain available. Software keyboard behavior, physical target sizes, and screen reader support need testing with the eventual mobile host. This is not yet an accessibility certification or a verified phone release.

Termux is the initial mobile host. A native app is not part of the current plan. Projects can be local or on a host reached over SSH. Mobux does not include an SSH connection manager or cross-machine file browser.

## Run and develop

```sh
mise install
mise run dev
mise exec -- cargo run --bin mobux -- /path/to/project
mise run fmt
mise run check
mise run test-ui
```

`check` runs formatting checks, Clippy, tests, and preview generation. `test-ui` requires tmux and Expect. It tests touch switching, process identity after UI restart, and confirmed installation with a fake package manager.

Generated previews are under `shots/`. Start with `home-mobile.png` and `light-mobile.png`. Their text uses bundled JetBrains Mono to approximate a terminal font. They are not phone screenshots. Development-machine window captures stay local and are excluded from Git. `shots/live/README.md` describes those checks.

Choose a startup palette with `MOBUX_THEME=one-dark-pro`, `MOBUX_THEME=gruvbox-material-light`, or a path to a custom JSON palette. Portable Base16 color data and source licenses are in `themes/`. Icon and font credits are in `assets/THIRD_PARTY.md`.

Rendering is event-driven. Idle screens do not repaint and unchanged backgrounds are not uploaded again. Content searches have a two-second deadline and a 512 KiB output bound. Results are capped at 30. Filename discovery is currently limited to 500 paths and six directory levels; it is not an exhaustive index. `mise exec -- cargo run --bin bench` measures drawing and PNG encoding, not terminal display time. See `PERFORMANCE.md` and `AGENTS.md`.
