#!/usr/bin/env python3
"""
fetch_icons.py
--------------
Baixa ícones Phosphor (MIT, github.com/phosphor-icons/core), rasteriza em PNG branco e salva em
assets/ui/icons/<nome>.png. No bevy_hui o ícone é tingido com `image_color` (ex.: tint-slate-400).
Só baixa/renderiza o que ainda não existe (--force refaz).

Requer o venv local (resvg-py):
  python3 -m venv tools/ui/.venv && tools/ui/.venv/bin/pip install resvg_py
  tools/ui/.venv/bin/python tools/ui/fetch_icons.py [--force] [--weight fill]

Para adicionar um ícone, inclua o nome (ex.: "armchair") em ICONS.
"""
import os
import sys
import urllib.request

import resvg_py

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
OUT_DIR = os.path.join(ROOT, "client", "assets", "ui", "icons")
URL = "https://raw.githubusercontent.com/phosphor-icons/core/main/assets/{weight}/{name}{suffix}.svg"
SIZE = 96  # rasterized at 3x of the 32px display size, for sharp mobile screens

ICONS = [
    "backpack", "x", "magnifying-glass", "armchair", "couch", "plant", "cube", "bed", "rug",
    "package", "sparkle", "question", "check", "hand-grabbing",
    "coins", "storefront", "shopping-cart", "check-circle", "warning-circle",
    "user", "compass", "gear", "hammer", "arrow-counter-clockwise", "arrow-clockwise", "dice-five",
    "buildings", "fire", "house", "plus-circle", "door-open",
    "caret-down", "caret-right", "bell", "users", "drop", "scissors", "t-shirt", "pants", "sneaker", "sunglasses",
]


def svg_url(name, weight):
    suffix = "" if weight == "regular" else f"-{weight}"
    return URL.format(weight=weight, name=name, suffix=suffix)


def to_white(svg):
    return svg.replace('fill="currentColor"', 'fill="#ffffff"').replace("currentColor", "#ffffff")


def main():
    argv = sys.argv[1:]
    force = "--force" in argv
    weight = argv[argv.index("--weight") + 1] if "--weight" in argv else "fill"
    os.makedirs(OUT_DIR, exist_ok=True)
    for name in ICONS:
        dst = os.path.join(OUT_DIR, f"{name}.png")
        if os.path.exists(dst) and not force:
            continue
        with urllib.request.urlopen(svg_url(name, weight), timeout=30) as resp:
            svg = to_white(resp.read().decode("utf-8"))
        with open(dst, "wb") as fh:
            fh.write(bytes(resvg_py.svg_to_bytes(svg_string=svg, width=SIZE, height=SIZE)))
        print(f"wrote {os.path.relpath(dst, ROOT)}")


if __name__ == "__main__":
    main()
