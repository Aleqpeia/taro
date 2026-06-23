# Card art provenance

The 78 card images in this directory are downloaded from Wikimedia Commons,
category **["Tarot de Marseille (Single Cards)"](https://commons.wikimedia.org/wiki/Category:Tarot_de_Marseille_(Single_Cards))**
— a complete, single-style scan set of a historical Tarot de Marseille deck.

- **License:** Public domain (`PD-US expired` on Commons — the underlying
  artwork predates copyright, and faithful 2D reproductions are not separately
  copyrightable).
- **Naming:** Files are renamed to match `taro_domain::Card::id()` so
  `Card::asset_path()` resolves directly (e.g. `major_00.png`, `coupes_ace.png`).
  Commons trumps `T1`–`T21` map to `major_01`–`major_21`; `TT` (the Fool) maps
  to `major_00`. Minor suits: `C`→Coupes, `P`→Deniers, `S`→Épées, `B`→Bâtons.
- **Regenerate:** `python3 scripts/fetch_cards.py` (idempotent; `--force` to
  re-download, `--width N` to change thumbnail size).

These files are ~500px-wide PNG thumbnails (~62 MB total). For distribution,
consider re-fetching at a smaller `--width` or converting to a compressed format.
