#!/usr/bin/env python3
"""Fetch Tarot de Marseille card art from Wikimedia Commons.

Source: Commons category "Tarot de Marseille (Single Cards)" — a complete,
single-style 78-card scan set, all marked PD-US (public domain by age).

Files are downloaded as ~600px-wide PNG thumbnails into
`crates/app/assets/cards/<card_id>.png`, where `<card_id>` matches
`taro_domain::Card::id()` (so `Card::asset_path()` resolves directly).

Re-runnable: existing files are skipped unless --force is passed.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

UPLOAD = "https://upload.wikimedia.org/wikipedia/commons/thumb"
USER_AGENT = "taro-card-fetch/0.1 (tarot art downloader; contact: local dev)"

ASSETS = Path(__file__).resolve().parent.parent / "crates" / "app" / "assets" / "cards"

# our suit key -> Commons suit letter (P=Pentacles/Deniers, C=Cups, S=Swords, B=Batons)
SUIT_LETTER = {"coupes": "C", "deniers": "P", "epees": "S", "batons": "B"}
# our rank key -> Commons rank token
RANK_TOKEN = {
    "ace": "1", "two": "2", "three": "3", "four": "4", "five": "5",
    "six": "6", "seven": "7", "eight": "8", "nine": "9", "ten": "10",
    "valet": "J", "cavalier": "H", "reine": "Q", "roi": "K",
}


def build_mapping() -> dict[str, str]:
    """card_id -> Commons file title (without the 'File:' prefix)."""
    mapping: dict[str, str] = {}
    # Major Arcana: TT = Le Mat (Fool, 0); T1..T21 = the numbered trumps.
    mapping["major_00"] = "TT Tarot.png"
    for n in range(1, 22):
        mapping[f"major_{n:02d}"] = f"T{n} Tarot.png"
    # Minor Arcana: "<rank><suit> Tarot.png".
    for suit, letter in SUIT_LETTER.items():
        for rank, token in RANK_TOKEN.items():
            mapping[f"{suit}_{rank}"] = f"{token}{letter} Tarot.png"
    return mapping


def thumb_url(title: str, width: int) -> str:
    """Build the deterministic Wikimedia thumbnail URL for a Commons file.

    Files are stored under a 2-level hashpath derived from the md5 of the
    underscored filename: /thumb/<h0>/<h0h1>/<name>/<width>px-<name>
    """
    name = title.replace(" ", "_")
    h = hashlib.md5(name.encode("utf-8")).hexdigest()
    enc = urllib.parse.quote(name)
    return f"{UPLOAD}/{h[0]}/{h[:2]}/{enc}/{width}px-{enc}"


def download(url: str, dest: Path, *, retries: int = 5) -> None:
    """Download with exponential backoff on HTTP 429 / transient errors."""
    delay = 2.0
    for attempt in range(retries):
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                dest.write_bytes(resp.read())
            return
        except urllib.error.HTTPError as e:
            if e.code == 429 and attempt < retries - 1:
                time.sleep(delay)
                delay *= 2
                continue
            raise


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--width", type=int, default=500, help="thumbnail width in px")
    ap.add_argument("--delay", type=float, default=0.7, help="seconds between downloads")
    ap.add_argument("--force", action="store_true", help="re-download existing files")
    args = ap.parse_args()

    ASSETS.mkdir(parents=True, exist_ok=True)
    mapping = build_mapping()
    assert len(mapping) == 78, f"expected 78 cards, built {len(mapping)}"

    todo = {
        cid: title
        for cid, title in mapping.items()
        if args.force or not (ASSETS / f"{cid}.png").exists()
    }
    if not todo:
        print("All 78 cards already present. Use --force to re-download.")
        return 0

    print(f"Downloading {len(todo)} cards (width={args.width}px)...")
    ok, fail = 0, []
    for cid, title in sorted(todo.items()):
        try:
            download(thumb_url(title, args.width), ASSETS / f"{cid}.png")
            ok += 1
            print(f"  {cid:<16} <- {title}")
        except Exception as e:  # noqa: BLE001
            fail.append((cid, title, str(e)))
        time.sleep(args.delay)  # be polite — avoid 429 throttling

    print(f"\nDownloaded {ok}/{len(todo)} cards into {ASSETS}")
    if fail:
        print("FAILED:")
        for cid, title, why in fail:
            print(f"  {cid} ({title}): {why}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
