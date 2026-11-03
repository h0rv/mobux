# Keyboard setup

Try HeliBoard for typing and keep terminal controls in Termux's extra-key row. You do not need a Mobux keyboard or a separate native app to expose arrows.

This recommendation is based on current project documentation. It has not been tested on a phone with Mobux.

## Show arrows in Termux

Edit `~/.termux/termux.properties`. Add or update the `extra-keys` setting without replacing your other settings:

```properties
extra-keys = [['ESC','CTRL','ALT','TAB','LEFT','DOWN','UP','RIGHT']]
```

Then run:

```sh
termux-reload-settings
```

Termux documents Volume Up + Q as the shortcut to toggle the extra-key row. The controls belong to Termux, so they are also available to micro, your shell, and other terminal programs.

For long prompts, try Termux's text-input view by swiping the extra-key row left. That gives the keyboard an Android text field. Use direct terminal input when navigating an editor.

## Keyboard choices

Checked against official sources in November 2026:

| Keyboard | Current status | Recommendation |
| --- | --- | --- |
| [HeliBoard](https://github.com/HeliBorg/HeliBoard) | GPLv3. Stable 4.2 was released October 12, 2026. | Try this first with the Termux arrow row. Its optional glide library is closed source; leave that out for a fully FOSS setup. |
| [Unexpected Keyboard](https://github.com/Julow/Unexpected-Keyboard) | GPLv3. Version 2.2.0 was released November 1, 2026. | If you are using an older version, the current release includes input fixes. Its corner gestures still may not suit you. |
| [FlorisBoard](https://github.com/florisboard/florisboard) | Apache 2.0. The project still describes current development as beta. | I would not choose it first to avoid input rough edges. |
| [FUTO Keyboard](https://github.com/futo-org/android-keyboard) | Source-available, with restrictions on commercial redistribution. | It is not a FOSS recommendation. |

A keyboard being maintained does not establish that it works well with a terminal. Test typing, selection, and modifiers in the apps you use.

## Editing

micro is the preferred editor for new Mobux configurations. If it is missing, opening a file in Termux offers `pkg install micro` and asks for confirmation. It is not bundled or installed automatically. Existing editor preferences are kept.

In micro, Ctrl+S saves, Ctrl+Q quits, and Ctrl+G opens help. Neovim remains an optional editor in Settings and Apps. Familiarity may make it useful to you, but modes and extra key combinations can be awkward on a software keyboard.

If Ctrl+S pauses output at a shell prompt, Ctrl+Q resumes it. `stty -ixon` disables that flow control for the current terminal. That shell behavior is separate from micro's own key handling.

## Source

[Termux touch keyboard documentation](https://wiki.termux.com/wiki/Touch_Keyboard).
