"""
glb_tools.py
------------
Minimal, dependency-free GLB (binary glTF 2.0) reader/writer used by the character
pipeline. Importable from plain CPython, so it can be unit tested without Blender.

Only what the pipeline needs is implemented: reading the JSON + binary chunks, decoding
float accessors, rebuilding a compact buffer, and replacing an embedded image by an
external file reference.
"""
import json
import struct

MAGIC = 0x46546C67  # "glTF"
CHUNK_JSON = 0x4E4F534A
CHUNK_BIN = 0x004E4942

COMPONENT_FLOAT = 5126
TYPE_SIZES = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_glb(path):
    """Returns `(gltf_json, binary_chunk)` of a `.glb` file."""
    with open(path, "rb") as f:
        data = f.read()
    magic, version, _length = struct.unpack_from("<III", data, 0)
    if magic != MAGIC or version != 2:
        raise ValueError(f"{path}: not a glTF 2.0 binary")
    gltf, binary, offset = None, b"", 12
    while offset < len(data):
        chunk_len, chunk_type = struct.unpack_from("<II", data, offset)
        chunk = data[offset + 8: offset + 8 + chunk_len]
        if chunk_type == CHUNK_JSON:
            gltf = json.loads(chunk)
        elif chunk_type == CHUNK_BIN:
            binary = bytes(chunk)
        offset += 8 + chunk_len
    if gltf is None:
        raise ValueError(f"{path}: missing JSON chunk")
    return gltf, binary


def _pad(data, fill):
    return data + fill * (-len(data) % 4)


def write_glb(path, gltf, binary):
    """Writes `gltf` and `binary` as a `.glb`; `buffers[0].byteLength` is updated."""
    if binary:
        gltf["buffers"] = [{"byteLength": len(binary)}]
    else:
        gltf.pop("buffers", None)
    json_chunk = _pad(json.dumps(gltf, separators=(",", ":")).encode("utf-8"), b" ")
    bin_chunk = _pad(binary, b"\x00")
    total = 12 + 8 + len(json_chunk) + (8 + len(bin_chunk) if bin_chunk else 0)
    out = bytearray(struct.pack("<III", MAGIC, 2, total))
    out += struct.pack("<II", len(json_chunk), CHUNK_JSON) + json_chunk
    if bin_chunk:
        out += struct.pack("<II", len(bin_chunk), CHUNK_BIN) + bin_chunk
    with open(path, "wb") as f:
        f.write(out)


def buffer_view_bytes(gltf, binary, index):
    view = gltf["bufferViews"][index]
    start = view.get("byteOffset", 0)
    return binary[start: start + view["byteLength"]]


COMPONENT_FORMATS = {
    5120: ("b", 127.0),
    5121: ("B", 255.0),
    5122: ("h", 32767.0),
    5123: ("H", 65535.0),
    5125: ("I", None),
    5126: ("f", None),
}


def read_accessor(gltf, binary, index):
    """
    Decodes a tightly-packed accessor into a list of tuples (or numbers for SCALAR).
    Normalized integer accessors are returned as floats in [0, 1] / [-1, 1].
    """
    acc = gltf["accessors"][index]
    fmt, norm = COMPONENT_FORMATS[acc["componentType"]]
    width = TYPE_SIZES[acc["type"]]
    size = struct.calcsize(fmt)
    view = gltf["bufferViews"][acc["bufferView"]]
    if view.get("byteStride", 0) not in (0, width * size):
        raise ValueError("interleaved accessors are not supported")
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    flat = struct.unpack_from(f"<{acc['count'] * width}{fmt}", binary, start)
    if acc.get("normalized") and norm:
        flat = [max(c / norm, -1.0) for c in flat]
    if width == 1:
        return list(flat)
    return [tuple(flat[i: i + width]) for i in range(0, len(flat), width)]


class BufferBuilder:
    """Accumulates buffer views/accessors for a freshly built glTF document."""

    def __init__(self, gltf, initial=b""):
        self.gltf = gltf
        self.data = bytearray(initial)
        gltf.setdefault("bufferViews", [])
        gltf.setdefault("accessors", [])

    def add_view(self, raw):
        self.data += b"\x00" * (-len(self.data) % 4)
        self.gltf["bufferViews"].append(
            {"buffer": 0, "byteOffset": len(self.data), "byteLength": len(raw)}
        )
        self.data += raw
        return len(self.gltf["bufferViews"]) - 1

    def add_floats(self, values, type_name):
        """Adds a float accessor from a list of floats (SCALAR) or tuples; returns its index."""
        width = TYPE_SIZES[type_name]
        flat = list(values) if width == 1 else [c for v in values for c in v]
        view = self.add_view(struct.pack(f"<{len(flat)}f", *flat))
        acc = {
            "bufferView": view,
            "componentType": COMPONENT_FLOAT,
            "count": len(values),
            "type": type_name,
        }
        if width == 1:
            acc["min"], acc["max"] = [min(values)], [max(values)]
        self.gltf["accessors"].append(acc)
        return len(self.gltf["accessors"]) - 1


def externalize_image(gltf, binary, uri):
    """
    Replaces every embedded image by a reference to the external file `uri` (relative to
    the glTF) and drops its bytes from the binary chunk. Returns the new binary chunk.
    All images of the document must be the same picture (true for the single-atlas packs).
    """
    images = gltf.get("images", [])
    drop = {img["bufferView"] for img in images if "bufferView" in img}
    for img in images:
        img.pop("bufferView", None)
        img.pop("mimeType", None)
        img["uri"] = uri
    return _drop_buffer_views(gltf, binary, drop)


def _drop_buffer_views(gltf, binary, drop):
    """Rebuilds the binary chunk without the buffer views in `drop`, fixing all references."""
    new_views, remap, data = [], {}, bytearray()
    for index, view in enumerate(gltf.get("bufferViews", [])):
        if index in drop:
            continue
        data += b"\x00" * (-len(data) % 4)
        moved = dict(view)
        moved["byteOffset"] = len(data)
        data += buffer_view_bytes(gltf, binary, index)
        remap[index] = len(new_views)
        new_views.append(moved)
    gltf["bufferViews"] = new_views
    if not new_views:
        gltf.pop("bufferViews")
    for acc in gltf.get("accessors", []):
        if "bufferView" in acc:
            acc["bufferView"] = remap[acc["bufferView"]]
    for img in gltf.get("images", []):
        if "bufferView" in img:
            img["bufferView"] = remap[img["bufferView"]]
    return bytes(data)


def node_world_paths(gltf):
    """Maps node index -> tuple of node names from the scene root down to that node."""
    nodes = gltf["nodes"]
    paths = {}
    stack = [(root, ()) for root in gltf["scenes"][gltf.get("scene", 0)]["nodes"]]
    while stack:
        index, prefix = stack.pop()
        path = prefix + (nodes[index].get("name"),)
        paths[index] = path
        stack.extend((child, path) for child in nodes[index].get("children", []))
    return paths
