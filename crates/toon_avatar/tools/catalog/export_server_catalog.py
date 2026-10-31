#!/usr/bin/env python3
"""
export_server_catalog.py
------------------------
Writes the cosmetics catalog the server validates against (`data/cosmetics/catalog.json`, version 3)
from the files of the asset pipeline: `outfits.json` (garments per body), `attachments.json`
(hair, hats, glasses) and, optionally, `custom_items.json`.

An item id is the catalog name (`Top.01`, `Hairstyle.06`, `F_head.001`). An item present for both
bodies is listed once with both body types. Props (things an avatar can hold) are listed in
`--props`.

Usage:
  python3 export_server_catalog.py --items <assets>/characters/items --out <server>/data/cosmetics/catalog.json \
      [--props cup,can] [--client-out <assets>/characters --materials <assets>/characters/materials]
"""
import argparse
import json
import os

BODY_TYPES = {"F": "female", "M": "male"}
GARMENTS = ("face", "top", "bottom", "shoes", "outfit")
ATTACHMENTS = ("hair", "hat", "glasses")


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def build(items_dir, props):
    outfits = load(os.path.join(items_dir, "outfits.json"))
    attachments = load(os.path.join(items_dir, "attachments.json"))
    custom_path = os.path.join(items_dir, "custom_items.json")
    custom = load(custom_path) if os.path.exists(custom_path) else {}

    # (category, id) -> set of body types, in first-seen order
    found = {}

    def add(category, name, body):
        found.setdefault((category, name), []).append(body) if body not in found.get((category, name), []) else None

    for letter, body in BODY_TYPES.items():
        for source in (custom.get(letter, {}), outfits.get(letter, {})):
            for category in GARMENTS:
                for entry in source.get(category, []):
                    add(category, entry["name"], body)
    for category in ATTACHMENTS:
        for entry in attachments.get(category, []):
            for body in BODY_TYPES.values():
                add(category, entry["name"], body)
    for prop in props:
        for body in BODY_TYPES.values():
            add("prop", prop, body)

    items = []
    for (category, name), bodies in found.items():
        items.append({"id": name, "name": name, "category": category, "body_types": bodies})
    items.sort(key=lambda i: (i["category"], i["id"]))
    return {"version": 3, "items": items}


def client_catalog(items_dir):
    """The catalog the client's `AvatarCatalog::from_json` reads: both bodies with their garments and attachments."""
    outfits = load(os.path.join(items_dir, "outfits.json"))
    attachments = load(os.path.join(items_dir, "attachments.json"))
    custom_path = os.path.join(items_dir, "custom_items.json")
    custom = load(custom_path) if os.path.exists(custom_path) else {}
    out = {}
    for letter in BODY_TYPES:
        body = {"body": outfits[letter]["body"]}
        for category in GARMENTS:
            body[category] = list(custom.get(letter, {}).get(category, [])) + list(outfits[letter].get(category, []))
        for category in ATTACHMENTS:
            body[category] = list(attachments.get(category, []))
        out[letter] = body
    return out


def merged_materials(materials_dir):
    """`materials.json` and `custom_materials.json` as one document (the custom ones override)."""
    merged = {}
    for name in ("materials.json", "custom_materials.json"):
        path = os.path.join(materials_dir, name)
        if os.path.exists(path):
            merged.update(load(path))
    return merged


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--items", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--props", default="cup,can")
    parser.add_argument("--client-out", help="directory to write the client's avatar_catalog.json and materials.json to")
    parser.add_argument("--materials", help="directory with materials.json (and custom_materials.json), for --client-out")
    args = parser.parse_args()
    catalog = build(args.items, [p for p in args.props.split(",") if p])
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(catalog, f, indent=2)
        f.write("\n")
    if args.client_out:
        os.makedirs(args.client_out, exist_ok=True)
        with open(os.path.join(args.client_out, "avatar_catalog.json"), "w", encoding="utf-8") as f:
            json.dump(client_catalog(args.items), f, indent=2)
            f.write("\n")
        if args.materials:
            with open(os.path.join(args.client_out, "materials.json"), "w", encoding="utf-8") as f:
                json.dump(merged_materials(args.materials), f)
        print(f"wrote the client files to {args.client_out}")
    counts = {}
    for item in catalog["items"]:
        counts[item["category"]] = counts.get(item["category"], 0) + 1
    print(f"wrote {args.out}: {len(catalog['items'])} items {counts}")


if __name__ == "__main__":
    main()
