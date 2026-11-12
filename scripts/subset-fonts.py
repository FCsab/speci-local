#!/usr/bin/env python3
"""Shrink the embedded 0xProto Nerd Fonts: keep every text glyph, and of the ~10 600 icons
only the ones `src/ui.rs` names. Run it again after adding an icon there:

    pip install fonttools
    scripts/subset-fonts.py <dir with the original 0xProtoNerdFont{Mono,Propo}-Regular.ttf>

The originals are in the Nerd Fonts release "0xProto" (https://www.nerdfonts.com).
"""
import re
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

root = Path(__file__).resolve().parent.parent


def icon(c):  # the Private Use Areas, where Nerd Fonts keeps its icons
    return 0xE000 <= c <= 0xF8FF or c >= 0xF0000


used = {int(h, 16) for h in re.findall(r"\\u\{([0-9a-fA-F]+)\}", (root / "src/ui.rs").read_text())}
for name in ["0xProtoNerdFontMono-Regular.ttf", "0xProtoNerdFontPropo-Regular.ttf"]:
    font = TTFont(Path(sys.argv[1]) / name)
    have = set(font.getBestCmap())
    missing = sorted(c for c in used if icon(c) and c not in have)
    assert not missing, f"{name} has no glyph for {[hex(c) for c in missing]}"
    keep = {c for c in have if not icon(c) or c in used}
    opts = subset.Options(layout_features=["*"], name_IDs=["*"], notdef_outline=True, hinting=True)
    s = subset.Subsetter(opts)
    s.populate(unicodes=keep)
    s.subset(font)
    font.save(root / "assets" / name)
    print(f"{name}: {len(have)} -> {len(keep)} characters, {(root / 'assets' / name).stat().st_size} bytes")
