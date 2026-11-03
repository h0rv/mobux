# Install Mobux in Termux

Mobux runs inside Termux and uses tmux for persistent apps. It is an experimental terminal workspace, not a separate Android app. No coding agents or editors are installed automatically.

GitHub Actions publishes prebuilt Android, Linux, and macOS binaries for ARM64 and x86_64. Android binaries are cross-compiled. A successful build is not proof of phone compatibility. Physical phone testing is still needed.

## Install a prebuilt snapshot

The snapshots support Android 9 or newer on ARM64 phones and x86_64 devices. Use a current Termux installation. See the [Termux installation documentation](https://github.com/termux/termux-app#installation) for supported sources.

In Termux, install the runtime and installer tools:

```sh
pkg update
pkg install tmux ripgrep curl jq tar coreutils
```

Download the installer so you can inspect it before running it:

```sh
curl -fLo install-mobux.sh \
  https://raw.githubusercontent.com/h0rv/mobux/main/scripts/install-termux.sh
bash install-mobux.sh
mobux --version
mobux
```

The installer downloads the newest published experimental snapshot from this repository, verifies its SHA-256 checksum, and installs only Mobux and its license notices under `$PREFIX`. It does not ask for root or install optional apps. Checksums detect corruption; they are not cryptographic signatures.

If no snapshot is available yet, [check the build](https://github.com/h0rv/mobux/actions/workflows/ci.yml). The first build must finish before the installer can download a binary. You can also build from source below.

To update, run the installer again. Existing settings and tmux sessions are not deleted.

## Linux and macOS downloads

The [snapshot releases](https://github.com/h0rv/mobux/releases) also contain Linux and macOS archives. Choose `x86_64` or `aarch64` for your machine, and `unknown-linux-gnu` or `apple-darwin` for the operating system. Extract the archive and run its `mobux/mobux` binary. Install tmux and ripgrep through your system package manager. Linux builds use Ubuntu 24.04. Windows and iOS binaries are not provided.

## Build from source on the phone

```sh
pkg update
pkg install git rust clang make pkg-config tmux ripgrep
git clone https://github.com/h0rv/mobux.git
cd mobux
cargo install --locked --path . --bin mobux --root "$PREFIX"
mobux --version
mobux "$HOME"
```

The first source build can take time. Keep the phone awake and ensure it has space for Rust dependencies. If replacing a previously installed binary, add `--force` to the `cargo install` command.

Keep projects under the Termux home directory rather than shared storage when tools need Unix permissions or executable files. Shared storage and Android document-provider access are separate constraints.

## First phone test

Start from a project directory:

```sh
mkdir -p "$HOME/mobux-test/src"
printf 'hello from the phone\n' > "$HOME/mobux-test/src/example.txt"
cd "$HOME/mobux-test"
mobux
```

Check these before relying on it for work:

1. Open Files and expand `src` without changing the project folder.
2. Tap the filter and find `example.txt`. Clear the filter and confirm the tree returns.
3. Choose Contents, type `hello`, and tap Find.
4. Return Home, open Terminal, and tap the tmux Home or Apps footer to leave it running.
5. Reopen Terminal and confirm the same shell is still there.
6. Quit Mobux, start it again, and check Open apps.
7. Try the same controls with the software keyboard visible.

Termux normally uses the cell renderer. Smooth native graphics require a terminal that supports the Kitty graphics protocol. Do not force `MOBUX_GRAPHICS=kitty` unless the host supports it. To force the development fallback:

```sh
MOBUX_GRAPHICS=cells mobux
```

Phone target sizes, keyboard behavior, and accessibility remain unverified. Android may kill background processes. tmux does not protect against the operating system killing its server.

## Termux practices

Use a supported Termux release from the sources in its official installation guide. Do not mix app and plugin packages from different signing sources. Keep development projects in `$HOME`; shared storage does not provide the same Unix file behavior. Only request storage access if you need it.

Do not paste credentials into saved commands or shell history. Android may stop background apps, so test session recovery on your device before trusting a long job to it. If Termux is being stopped, review its battery settings. Mobux cannot override Android's process limits.

## Keyboard and editor

See the [keyboard guide](docs/KEYBOARD.md) for a Termux arrow-key row and current FOSS keyboard choices. Try HeliBoard for typing and Termux's extra keys for terminal controls.

micro is the preferred editor for new configurations. Opening a file offers a confirmed installation if micro is missing on Termux. Existing preferences are preserved. You can choose Neovim in Settings instead.

## Themes

Change Theme in Settings, or set `MOBUX_THEME` before starting:

```sh
MOBUX_THEME=catppuccin-mocha mobux
MOBUX_THEME=catppuccin-latte mobux
MOBUX_THEME=tokyo-night-dark mobux
```

Gruvbox Material Dark, Gruvbox Material Light, and One Dark Pro are also included. Custom JSON palettes use the format in `themes/`. The [theme gallery](docs/THEMES.md) shows generated mobile layouts.

## App switching and recovery

Mobux needs tmux 3.4 or newer for the wide Home and Apps status controls. Check with `tmux -V`. Without it, apps run in the foreground and cannot be switched persistently. `MOBUX_SESSIONS=foreground` selects that mode explicitly.

An exited app requires Restart. Stop requires confirmation. Removing an app from the workspace registry does not stop its process.

The session server is ordinary tmux on a private socket. You can inspect it without Mobux:

```sh
tmux -L mobux list-sessions
tmux -L mobux attach
```

Use the standard tmux detach command, Ctrl+b followed by d, if you need to leave an attached session without the touch footer.

## Optional tools

Add catalog entries to the workspace before opening them. Confirmed Termux package recipes are available for micro, GitHub CLI, Neovim, Helix, lazygit, and Yazi. The dialog shows the package command before execution. The package manager may ask for confirmation too.

Definitions are in `catalog/apps.json`. `termux_package` names an audited Termux package. `requires` lists other executable commands an app needs. Mobux checks those commands before launch and offers a known package recipe when available. The package manager handles library dependencies. Mobux does not enable extra repositories automatically.

Core runtime tools are tmux 3.4 or newer, ripgrep for content search, and coreutils for process launch. The Termux installer instructions include them. No optional editor or coding agent is bundled.

GitHub sign-in is separate. Run `gh auth login` in the GitHub shell. Mobux does not save GitHub tokens or automatically publish or merge changes.

GitHub Dash (`gh-dash`) is an optional TUI for pull requests and issues. It uses GitHub CLI authentication. Its Termux package is in the separate [Termux User Repository](https://github.com/termux-user-repository/tur/blob/master/tur/gh-dash/build.sh), not the main repository. Mobux does not install or enable that repository for you. Upstream has a bottom preview layout, but the installed package may lag. [Mouse support remains an open PR](https://github.com/dlvhdr/gh-dash/pull/722), so this is a keyboard-oriented candidate, not a verified touch interface.

Herdr is an optional agent manager, not a coding agent. Its official installer rejects native Android and Termux. Use a supported host for it. OpenCode and Hunk have no verified native Termux installation path in this project. Package availability does not establish touchscreen usability.

## Sources

- [Termux installation and documentation](https://github.com/termux/termux-app).
- [Termux execution environment](https://github.com/termux/termux-packages/wiki/Termux-execution-environment).
- [GitHub CLI Linux and Termux installation](https://github.com/cli/cli/blob/trunk/docs/install_linux.md).
- [Termux micro package recipe](https://github.com/termux/termux-packages/blob/master/packages/micro/build.sh).
- [Herdr installer and host checks](https://herdr.dev/install.sh).
- [OpenCode documentation](https://opencode.ai/docs/).
- [Hunk](https://hunk.dev).
