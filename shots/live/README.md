# Live Ghostty captures

These PNG files are actual Ghostty window captures on macOS. They use the shared pixel background with native terminal text. They are not generated phone mockups.

`ghostty-workspace-files.png` shows the current file tree beside the empty Minimal workspace. The source folder is expanded through a mouse event. The panes are about 42 columns wide and 49 rows tall. The inactive pane is dimmed by Ghostty, not by Mobux's palette.

`ghostty-minimal-dark-light.png` shows an earlier dark and light comparison before optional apps became explicitly registered. The other older captures cover the previous Android layout, settings, and palettes. Keep these distinct from the current workspace.

The automated PTY tests cover nested expansion, focused input, fuzzy filtering, content search, confirmed installation with a fake package manager, and app persistence after a graceful UI restart. No packages were installed. The native check used scripted keyboard and mouse events, not physical phone touches.

Window captures do not verify Android, software keyboards, physical target sizes, or screen reader behavior. Generated fixed-data previews remain directly under `shots/`.
