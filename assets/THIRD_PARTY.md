# Third party assets

## Use and layout

The five Home icons are `account_tree` (Git utility), `opencode`, `folder` (Files), `apps`, and `settings`. `terminal` is an additional command utility icon, not a sixth Home icon. Each name has an original `.svg` and a prepared `.png` in `icons/`.

The PNG files are transparent 96 by 96 RGBA images. Their black glyphs have a maximum visible dimension of 58 pixels, about 60% of the badge width. They are centered without a circle, shadow, or background. The renderer can tint their alpha masks and add its own adaptive shape. No SVG parser, font, or network request is needed at runtime.

The Google symbols follow the current Material Symbols Rounded style. They are not copies of proprietary Pixel launcher app artwork. Git uses Google's generic `account_tree` symbol, not the Git logo. No Git trademark artwork is bundled.

## Google Material Symbols

Google publishes these icons under Apache License 2.0. The upstream [README license section](https://github.com/google/material-design-icons/blob/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/README.md#license) explicitly covers the icons. The complete upstream license is bundled at `icons/licenses/Material-Symbols-Apache-2.0.txt`.

Revision `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7` was resolved from `master` when fetched. These are Rounded variants with fill 1, weight 400, grade 0, and optical size 24. The bundled SVG files are unmodified downloads. Exact sources follow.

* `icons/folder.svg`: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/symbols/web/folder/materialsymbolsrounded/folder_fill1_24px.svg
* `icons/settings.svg`: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/symbols/web/settings/materialsymbolsrounded/settings_fill1_24px.svg
* `icons/apps.svg`: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/symbols/web/apps/materialsymbolsrounded/apps_fill1_24px.svg
* `icons/terminal.svg`: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/symbols/web/terminal/materialsymbolsrounded/terminal_fill1_24px.svg
* `icons/account_tree.svg`: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/symbols/web/account_tree/materialsymbolsrounded/account_tree_fill1_24px.svg
* License: https://raw.githubusercontent.com/google/material-design-icons/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/LICENSE

The PNG derivatives were modified for Mobux by rasterizing, cropping transparent margins, scaling, and adding transparent padding. Their glyph geometry is unchanged.

## OpenCode

`icons/opencode.svg` is the unmodified standalone light logo from Anomaly's official repository. It is the asset offered by the [official brand page](https://opencode.ai/brand), not a generic agent symbol. The page's [source](https://github.com/anomalyco/opencode/blob/2e8a861e11d6fe51c03d6aa4ac93a7d8a41b4aab/packages/console/app/src/routes/brand/index.tsx) imports that exact brand asset.

Revision `2e8a861e11d6fe51c03d6aa4ac93a7d8a41b4aab` was resolved from `dev` when fetched. The repository's root MIT license states `Copyright (c) 2025 opencode`. That complete notice is bundled at `icons/licenses/OpenCode-MIT.txt`. This is the copyright license evidence for the repository asset, not a claim that the brand is unrestricted or that Anomaly endorses Mobux.

* Original: https://raw.githubusercontent.com/anomalyco/opencode/2e8a861e11d6fe51c03d6aa4ac93a7d8a41b4aab/packages/console/app/src/asset/brand/opencode-logo-light.svg
* License: https://raw.githubusercontent.com/anomalyco/opencode/2e8a861e11d6fe51c03d6aa4ac93a7d8a41b4aab/LICENSE

`icons/opencode-monochrome.svg` is a Mobux modification. It retains the original outer path, changes its fill to black, and omits the gray interior path. The removed interior is transparent, so the official rectangular outline remains recognizable on a themed badge. Unneeded clipping and masking wrappers were removed. `icons/opencode.png` was rasterized from this derivative, scaled, and padded. It is not an unmodified official monochrome variant.

## Reproduce PNG files

Run from the project root with librsvg and ImageMagick installed. These files were generated with `rsvg-convert` 2.63.4 and ImageMagick 7.1.2-35.

```sh
for name in folder settings apps terminal account_tree; do
  rsvg-convert --width 480 --keep-aspect-ratio "assets/icons/$name.svg" |
    magick png:- -trim +repage -resize 58x58 -gravity center \
      -background none -extent 96x96 -strip "PNG32:assets/icons/$name.png"
done
rsvg-convert --width 480 --keep-aspect-ratio assets/icons/opencode-monochrome.svg |
  magick png:- -resize 58x58 -gravity center -background none \
    -extent 96x96 -strip PNG32:assets/icons/opencode.png
```

Do not trim the OpenCode source by corner color. Its opaque frame touches every source edge, and that operation would remove the frame.

## Preview font

`fonts/JetBrainsMono-Regular.ttf` is an unmodified JetBrains Mono font from [JetBrains/JetBrainsMono](https://github.com/JetBrains/JetBrainsMono/tree/master/fonts/ttf). It is distributed under the SIL Open Font License 1.1, bundled at `fonts/OFL.txt`. The source download is https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Regular.ttf.

This font is used for generated preview text only. Live text remains native and uses the terminal's selected font. The cell-renderer comparison retains its bitmap text renderer.
