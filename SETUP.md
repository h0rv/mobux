# Install Mobux in Termux

Mobux runs inside Termux and uses tmux for persistent apps. It is an experimental terminal workspace, not a separate Android app. No coding agents or editors are installed automatically.

Android binaries are cross-compiled in GitHub Actions. A successful build is not proof of phone compatibility. Physical phone testing is still needed.

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

Add catalog entries to the workspace before opening them. On Termux, a missing GitHub CLI or micro entry offers a confirmed package installation. The dialog shows `pkg install gh` or `pkg install micro` before execution. No remote install scripts or guessed Android packages are used for catalog entries.

GitHub sign-in is separate. Run `gh auth login` in the GitHub shell. Mobux does not save GitHub tokens or automatically publish or merge changes.

Herdr is an optional agent manager, not a coding agent. Its official installer rejects native Android and Termux. Use a supported host for it. OpenCode and Hunk have no verified native Termux installation path in this project. Package availability does not establish touchscreen usability.

## Sources

- [Termux installation and documentation](https://github.com/termux/termux-app).
- [Termux execution environment](https://github.com/termux/termux-packages/wiki/Termux-execution-environment).
- [GitHub CLI Linux and Termux installation](https://github.com/cli/cli/blob/trunk/docs/install_linux.md).
- [Termux micro package recipe](https://github.com/termux/termux-packages/blob/master/packages/micro/build.sh).
- [Herdr installer and host checks](https://herdr.dev/install.sh).
- [OpenCode documentation](https://opencode.ai/docs/).
- [Hunk](https://hunk.dev).
