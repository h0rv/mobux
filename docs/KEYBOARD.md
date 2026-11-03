# Keyboard setup

Try HeliBoard for typing and keep terminal controls in Termux's extra-key row. You do not need a Mobux keyboard or a separate native app to expose arrows.

This recommendation is based on current project documentation. It has not been tested on a phone with Mobux.

## Vim setup

Use HeliBoard for letters and keep the terminal keys in Termux. That avoids making a custom keyboard layout just to get Esc.

Download the optional setup helper and run it explicitly:

```sh
curl -fLo setup-mobux-vim.sh https://raw.githubusercontent.com/h0rv/mobux/main/scripts/setup-termux-vim.sh
bash setup-mobux-vim.sh
```

The helper shows its changes and asks for confirmation. It preserves other Termux properties and saves backup copies of existing files. It replaces only the extra-key row setting. It does not change HeliBoard, your editor, or Mobux preferences.

The preset has two rows:

```properties
extra-keys = [['ESC','CTRL','ALT','TAB','HOME','END'],[':','/','LEFT','DOWN','UP','RIGHT']]
```

Use Esc to leave Vim's insert mode. The dedicated colon and slash keys give quick access to commands and search. Ctrl+U and Ctrl+D scroll in Vim. Those are Vim's bindings, not Mobux commands.

For Mobux navigation, turn on **Vim keys** in Settings. It is off by default and saved with your preferences.

| Key | Mobux behavior outside a text field |
| --- | --- |
| j / k | Move down or up in lists and Settings. |
| h | Go back. In Files, go to the parent directory. |
| l | Open the selected item. |
| h / l in Settings | Change the selected setting. |
| / in Files | Focus the filename filter. |
| Enter / Esc | Open or go back using the normal controls. |

Search fields keep typing normally, including h, j, k, and l. Use arrows or touch to select search results. Mobux is not a full Vim editor. Attached programs keep their own bindings.

Choose Neovim as the editor in Settings if you prefer it. micro remains the default.

## Matching colors

To apply the current saved Mobux palette to Termux too, use the same helper with an explicit option:

```sh
bash setup-mobux-vim.sh --theme
```

This also backs up and replaces Termux's `colors.properties`. It does not change Android's system theme.

Mobux can export a matching HeliBoard theme:

```sh
mobux --print-heliboard-theme > mobux-heliboard.json
```

In HeliBoard's color-theme picker, choose Load, then load the JSON file. Alternatively, copy the JSON text and use Load > Paste. Select the imported theme afterward. The export uses HeliBoard 4.2's documented source format. It has not been imported on a physical phone yet.

If Android's file picker cannot access the file in Termux, copy it to shared storage you have explicitly enabled, or use the clipboard route. No Termux:API plugin is required for manually copying text.

For a different palette without changing saved preferences:

```sh
MOBUX_THEME=catppuccin-mocha mobux --print-heliboard-theme > mobux-heliboard.json
```

`mobux --print-termux-theme` prints the Termux color file without writing settings. These are one-time exports. Changing a Mobux theme later does not update HeliBoard automatically.

## HeliBoard customization

HeliBoard supports system day/night colors and custom themes. It also has editable functional layouts with Esc and Ctrl. Those changes must be made inside HeliBoard. There is no supported external settings API that Mobux can use to select a keyboard or silently change its layout.

I recommend starting with the Termux preset above. If you later edit a HeliBoard functional layout, use its [layout documentation](https://github.com/HeliBorg/HeliBoard/blob/v4.2/layouts.md). Its plain Ctrl key applies to the next key or until released. Ctrl Lock stays active until you turn it off.

## Compact arrow row

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

[Termux touch keyboard documentation](https://wiki.termux.com/wiki/Touch_Keyboard). HeliBoard 4.2's [theme exporter](https://github.com/HeliBorg/HeliBoard/blob/v4.2/app/src/main/java/helium314/keyboard/settings/screens/ColorsScreen.kt) and [theme importer](https://github.com/HeliBorg/HeliBoard/blob/v4.2/app/src/main/java/helium314/keyboard/settings/dialogs/ColorThemePickerDialog.kt) define the JSON format used by Mobux.
