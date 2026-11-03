# Theme sources and licenses

These files contain color data. The application loads them through `src/palette.rs`, independently of its Android/iPhone layout. Gruvbox Material Dark is the default. The alternate presets are One Dark Pro, Gruvbox Material Light, Catppuccin Mocha, Catppuccin Latte, and Tokyo Night Dark. `MOBUX_THEME` can select a preset or a custom JSON file.

Each JSON file has `name`, `author`, `variant`, and `palette` fields. The palette contains all 16 Base16 roles, from `base00` through `base0F`, as six lowercase hexadecimal digits without `#`.

## Gruvbox Material

The bundled `gruvbox-material-dark.json` and `gruvbox-material-light.json` use the Medium variants from [Tinted Theming's schemes repository](https://github.com/tinted-theming/schemes). The upstream author credit is Mayush Kumar and sainnhe. The full credit is preserved in each JSON file.

The pinned revision is `dd2a1379533372ad79560bb4392570782a8a0a9d`.

Sources at that revision:

* [Dark Medium palette](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/base16/gruvbox-material-dark-medium.yaml).
* [Light Medium palette](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/base16/gruvbox-material-light-medium.yaml).
* [MIT license](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/LICENSE). An unchanged copy is included in [licenses/tinted-theming-MIT.txt](licenses/tinted-theming-MIT.txt).

The YAML was converted to JSON. The `system` field and the leading `#` on each color were removed to fit the local schema. Names, author credits, variants, and all palette values are unchanged. These are upstream Base16 schemes, not new color adaptations.

## Catppuccin and Tokyo Night

The bundled `catppuccin-mocha.json`, `catppuccin-latte.json`, and `tokyo-night-dark.json` use the Base16 palettes from [Tinted Theming's schemes repository](https://github.com/tinted-theming/schemes) at revision `dd2a1379533372ad79560bb4392570782a8a0a9d`.

Sources at that revision:

* [Catppuccin Mocha](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/base16/catppuccin-mocha.yaml).
* [Catppuccin Latte](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/base16/catppuccin-latte.yaml).
* [Tokyo Night Dark](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/base16/tokyo-night-dark.yaml).
* [MIT license](https://github.com/tinted-theming/schemes/blob/dd2a1379533372ad79560bb4392570782a8a0a9d/LICENSE). An unchanged copy is included in [licenses/tinted-theming-MIT.txt](licenses/tinted-theming-MIT.txt).

The Catppuccin schemes credit `https://github.com/catppuccin/catppuccin`. Tokyo Night Dark credits Michaël Ball. Each JSON file preserves the upstream name, author, and variant.

The YAML was converted to JSON without adding a parser or changing any Base16 role. The `system` field, YAML comments, and leading `#` on each color were removed. Hexadecimal letters were made lowercase. Tokyo Night Dark is the upstream dark palette, not the Storm or Light palette. Its Base16 role assignments are preserved even where they differ from common Tokyo Night terminal colors.

The startup names are `catppuccin-mocha`, `catppuccin-latte`, and `tokyo-night-dark`. Cycling appends them in that order after the existing Gruvbox Material Dark, One Dark Pro, and Gruvbox Material Light presets.

## One Dark Pro

The bundled `one-dark-pro.json` is a Base16 adaptation of [Binaryify's One Dark Pro](https://github.com/Binaryify/OneDark-Pro). It uses the regular One Dark Pro variant, not the Darker, Flat, Mix, or Night Flat variants. It was not copied from a generic One Dark scheme.

The pinned revision is `75c80ab18d90655e05ad837bb8d89f5e68d5ce53`.

Sources at that revision:

* [OneDark-Pro.json](https://github.com/Binaryify/OneDark-Pro/blob/75c80ab18d90655e05ad837bb8d89f5e68d5ce53/themes/OneDark-Pro.json).
* [MIT license](https://github.com/Binaryify/OneDark-Pro/blob/75c80ab18d90655e05ad837bb8d89f5e68d5ce53/LICENSE.txt). An unchanged copy is included in [licenses/one-dark-pro-MIT.txt](licenses/one-dark-pro-MIT.txt).

Upstream is a VS Code theme, not a Base16 scheme. The role assignments below are local choices. Every color comes from that upstream file. No colors were blended or invented. The leading `#` was removed and hexadecimal letters were made lowercase. The name and author fields identify the adaptation.

| Base16 role | Color | Upstream source and local use |
| --- | --- | --- |
| `base00` | `282c34` | `colors.editor.background`. This is the default background. |
| `base01` | `2c313c` | `colors.editor.lineHighlightBackground`. This is an alternate background. |
| `base02` | `323842` | `colors.list.focusBackground`. This is the selection background. |
| `base03` | `7f848e` | The `Comments` token rule for `comment, punctuation.definition.comment`. This is the comment color. |
| `base04` | `9da5b4` | `colors.statusBar.foreground`. This is a muted foreground. |
| `base05` | `abb2bf` | `colors.editor.foreground`. This is the default foreground. |
| `base06` | `d7dae0` | `colors.list.activeSelectionForeground`. This is a brighter foreground. |
| `base07` | `e6e6e6` | `colors.terminal.ansiBrightWhite`. This is the brightest foreground. |
| `base08` | `e06c75` | The `js variable readwrite` token rule, including `variable.other.readwrite`. This is the variable and red accent color. |
| `base09` | `d19a66` | The `Integers` token rule for `constant.numeric`. This is the number and orange accent color. |
| `base0A` | `e5c07b` | The `Type Name` token rule for `entity.name.type`. This is the type and yellow accent color. |
| `base0B` | `98c379` | The `Strings` token rule for `string`. This is the string and green accent color. |
| `base0C` | `56b6c2` | The `Regular Expressions` token rule for `string.regexp`. This is the regular expression and cyan accent color. |
| `base0D` | `61afef` | The `Functions` token rule, including `entity.name.function`. This is the function and blue accent color. |
| `base0E` | `c678dd` | The `Keywords` token rule for `keyword`. This is the keyword and violet accent color. |
| `base0F` | `be5046` | The `punctuation.section.embedded.begin.php` token rule. This is the embedded delimiter and brown/red accent color. |

The opaque list focus color is used for `base02` instead of the editor's translucent selection color (`#67769660`). The comment color comes from the general `Comments` rule, not the darker `Comment Markup Link` rule. The `base0F` accent comes from PHP embedded delimiters rather than an invented brown.

This adaptation does not reproduce VS Code's language-specific rules, semantic tokens, font styles, or transparent overlays. Its Base16 role mapping is not an upstream Binaryify release or an exact conversion of all VS Code behavior.
