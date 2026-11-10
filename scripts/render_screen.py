#!/usr/bin/env python3
"""Render costcrew output to a terminal-style PNG screenshot.

Parses the ANSI sequences costcrew actually emits (so screenshots are always
genuine), draws them in Consolas on a terminal window with chrome.

Usage:
  python scripts/render_screen.py assets/demo.png "costcrew demo" -- demo
  python scripts/render_screen.py assets/scan.png "costcrew scan --since 30d" -- scan --since 30d
  python scripts/render_screen.py assets/help.png "costcrew --help" -- --help

Commit the PNGs under assets/ — they are pure program output, no hand edits.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
EXE = ROOT / "target" / "release" / "costcrew.exe"

FG = (230, 237, 243)
DIM = (139, 148, 158)
COLORS = {31: (240, 87, 79), 32: (63, 185, 80), 33: (210, 153, 34), 36: (121, 192, 255)}
BRIGHT = {31: (255, 123, 114), 32: (86, 211, 100), 33: (227, 179, 65)}
BG = (13, 17, 23)
BAR_BG = (30, 34, 43)
DOTS = [(255, 95, 87), (254, 188, 46), (40, 200, 64)]

ANSI_RE = re.compile(r"\x1b\[([0-9;]*)m")

Cell = tuple[tuple[int, int, int], bool, str]  # (color, bold, char)


def tokenize_cli(cli_text: str) -> list[Cell]:
    """Tokenizer for a plain (non-ANSI) command line: whole command in bright FG."""
    return [(FG, True, ch) for ch in cli_text]


def tokenize_ansi(text: str) -> list[Cell]:
    """Tokenize ANSI text: returns (color, bold, char), newline chars included."""
    out: list[Cell] = []
    color = FG
    bold = False
    pos = 0
    for m in ANSI_RE.finditer(text):
        for ch in text[pos : m.start()]:
            out.append((color, bold, ch))
        pos = m.end()
        for c in (int(x) for x in m.group(1).split(";") if x != ""):
            if c == 0:
                color, bold = FG, False
            elif c == 1:
                bold = True
            elif c == 2:
                color = DIM
            elif c == 22:
                bold = False
            elif c in COLORS:
                base = BRIGHT.get(c) if (bold and c in BRIGHT) else COLORS[c]
                color = base
    out.extend((color, bold, ch) for ch in text[pos:])
    return out


def split_rows(cells: list[Cell]) -> list[list[Cell]]:
    rows: list[list[Cell]] = []
    row: list[Cell] = []
    for cell in cells:
        if cell[2] == "\n":
            rows.append(row)
            row = []
        else:
            row.append(cell)
    if row:
        rows.append(row)
    return rows


def render(rows: list[list[Cell]], prompt: list[Cell], out_path: Path, title: str):
    font = ImageFont.truetype(r"C:\Windows\Fonts\consola.ttf", 26)
    font_bold = ImageFont.truetype(r"C:\Windows\Fonts\consolab.ttf", 26)
    cell_w = font.getlength("0")
    cell_h = 34
    pad = 34
    bar_h = 58
    gap = 12

    grid = [prompt] + [[]] + rows  # prompt line, blank line, then output rows
    max_len = max(len(r) for r in grid)
    width = int(max_len * cell_w) + pad * 2
    height = bar_h + gap + cell_h + gap + len(grid[2:]) * cell_h + pad

    img = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(img)

    # title bar + traffic lights
    d.rectangle([0, 0, width, bar_h], fill=BAR_BG)
    for i, tc in enumerate(DOTS):
        cx = 22 + i * 38
        cy = bar_h // 2
        d.ellipse([cx - 11, cy - 11, cx + 11, cy + 11], fill=tc)
    tf = ImageFont.truetype(r"C:\Windows\Fonts\segoeui.ttf", 22)
    tw = d.textlength(title, font=tf)
    d.text(((width - tw) / 2, (bar_h - 24) / 2 + 2), title, font=tf, fill=(200, 205, 215))

    # prompt + output
    y = bar_h + gap
    for row in grid:
        x = 0.0
        for color, bold, ch in row:
            f = font_bold if bold else font
            d.text((pad + x, y), ch, font=f, fill=color)
            x += cell_w
        y += cell_h

    # rounded corners
    mask = Image.new("L", (width, height), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, width - 1, height - 1], radius=16, fill=255)
    rgba = img.convert("RGBA")
    rgba.putalpha(mask)
    rgba.save(out_path)
    print(f"saved {out_path} ({width}x{height})")


def main():
    if len(sys.argv) < 4 or sys.argv.count("--") != 1:
        sys.exit(__doc__)
    out_path = Path(sys.argv[1])
    out_path.parent.mkdir(exist_ok=True)
    title = sys.argv[2]

    max_lines = None
    if "--max-lines" in sys.argv:
        i = sys.argv.index("--max-lines")
        max_lines = int(sys.argv[i + 1])
        del sys.argv[i : i + 2]

    run_args = sys.argv[sys.argv.index("--") + 1 :]

    proc = subprocess.run(
        [str(EXE), *run_args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    body = (proc.stdout if proc.stdout.strip() else proc.stderr) + "\x1b[0m"

    prompt = [
        *[(DIM, False, ch) for ch in "PS C:\\sources\\costcrew> "],
        *tokenize_cli(title),
        (FG, False, "\n"),
    ]
    cells = prompt + tokenize_ansi(body)
    rows = split_rows(cells)
    if max_lines:
        rows = rows[: max_lines + 1]  # prompt row + N body rows
        rows.append([])  # ellipsis hint line
        rows.append([(DIM, False, ch) for ch in "  ... (full output with `costcrew catalog`)"])
    render(rows[1:], rows[0], out_path, title)


if __name__ == "__main__":
    main()
