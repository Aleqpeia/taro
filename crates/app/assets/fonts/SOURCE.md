# Font provenance

The UI fonts in this directory are the **Liberation Serif** family (Regular,
Bold, Italic) by Red Hat — a metric-compatible substitute for Times New Roman,
chosen for its classical book-serif feel and crisp rendering at small sizes.

- **License:** SIL Open Font License 1.1 (OFL) — free to bundle and redistribute.
- **Source:** the `liberation-serif-fonts` system package
  (`/usr/share/fonts/liberation-serif-fonts/`). Upstream:
  <https://github.com/liberationfonts/liberation-fonts>.

Liberation Serif is a static (non-variable) TTF, which `cosmic-text` (Bevy's
text backend) shapes reliably; variable fonts were avoided deliberately.
