# Mobux

Mobux adds a touch-friendly workspace to Termux. Browse files, search a project, and switch between programs without closing them.

Termux runs the tools. tmux keeps them alive. Mobux provides navigation.

## Install

Install Termux from [F-Droid](https://f-droid.org/en/packages/com.termux/) first. The [Google Play version](https://play.google.com/store/apps/details?id=com.termux) is experimental. See [Termux's installation guide](https://github.com/termux/termux-app#installation) for differences. Keep Termux and its plugins from the same source.

Then, in Termux on Android 9 or newer:

```sh
pkg update
pkg install tmux ripgrep curl jq tar coreutils
curl -fLo install-mobux.sh https://raw.githubusercontent.com/h0rv/mobux/main/scripts/install-termux.sh
bash install-mobux.sh
mobux
```

The installer downloads a prebuilt binary and verifies its checksum. Phone support is experimental.

[Setup and keyboard guide](SETUP.md) · [Downloads](https://github.com/h0rv/mobux/releases)

## Recommended software

- Try [HeliBoard from F-Droid](https://f-droid.org/en/packages/helium314.keyboard/) for typing. Leave out its optional closed-source glide library for a fully FOSS setup. Use [Termux's extra-key row](docs/KEYBOARD.md) for arrows and Ctrl.
- Use micro for simple editing. It is the default for new Mobux configurations, with installation offered for confirmation. Neovim is also supported.

## Preview

These are generated mobile layouts, not phone screenshots. Termux uses the cell renderer by default.

<p>
  <img src="shots/home-mobile.png" width="260" alt="Mobux workspace">
  <img src="shots/files-mobile.png" width="260" alt="File browser and filter">
</p>

## Development

```sh
mise run check
mise run test-ui
```

Mobux is MIT licensed. [Asset credits](assets/THIRD_PARTY.md) and [theme sources](themes/THIRD_PARTY.md) cover third-party files.
