"""
tailwind.py
-----------
Traduz classes utilitárias Tailwind em atributos de template do bevy_hui 0.7.
Lógica pura (sem I/O); usada por build_ui.py. Classe desconhecida levanta UnknownClass.
"""
import re

SPACING_UNIT_PX = 4  # Tailwind: 1 unit = 0.25rem = 4px

# Tailwind palette (sRGB hex).
PALETTE = {
    "slate": {50: "f8fafc", 100: "f1f5f9", 200: "e2e8f0", 300: "cbd5e1", 400: "94a3b8", 500: "64748b",
              600: "475569", 700: "334155", 800: "1e293b", 900: "0f172a", 950: "020617"},
    "indigo": {50: "eef2ff", 100: "e0e7ff", 200: "c7d2fe", 300: "a5b4fc", 400: "818cf8", 500: "6366f1",
               600: "4f46e5", 700: "4338ca", 800: "3730a3", 900: "312e81", 950: "1e1b4b"},
    "emerald": {50: "ecfdf5", 100: "d1fae5", 200: "a7f3d0", 300: "6ee7b7", 400: "34d399", 500: "10b981",
                600: "059669", 700: "047857", 800: "065f46", 900: "064e3b", 950: "022c22"},
    "rose": {400: "fb7185", 500: "f43f5e", 600: "e11d48", 700: "be123c"},
    "amber": {400: "fbbf24", 500: "f59e0b", 600: "d97706"},
}
SPECIAL_COLORS = {"white": "ffffff", "black": "000000", "transparent": "00000000"}

TEXT_SIZES = {"xs": 12, "sm": 14, "base": 16, "lg": 18, "xl": 20, "2xl": 24, "3xl": 30}
RADII = {"none": 0, "sm": 2, "": 4, "md": 6, "lg": 8, "xl": 12, "2xl": 16, "3xl": 24, "full": 9999}
ALIGN = {"start": "flex_start", "end": "flex_end", "center": "center", "stretch": "stretch"}
JUSTIFY = {"start": "flex_start", "end": "flex_end", "center": "center", "between": "space_between",
           "around": "space_around", "evenly": "space_evenly"}
TEXT_ALIGN = {"left": "left word_boundary", "center": "center word_boundary", "right": "right word_boundary"}
STATE_PREFIXES = ("hover", "active", "pressed")


class UnknownClass(ValueError):
    pass


def px(n):
    value = n * SPACING_UNIT_PX
    return f"{int(value)}px" if value == int(value) else f"{value}px"


def spacing(token):
    """'4' -> '16px', 'px' -> '1px', '0.5' -> '2px'."""
    if token == "px":
        return "1px"
    try:
        return px(float(token))
    except ValueError:
        raise UnknownClass(token)


def size(token):
    """w/h values: 'full', 'auto', 'N', '[123px]', '1/2'."""
    if token == "full":
        return "100%"
    if token == "auto":
        return "auto"
    if token == "screen":
        return "100vw"
    if token.startswith("[") and token.endswith("]"):
        return token[1:-1]
    if "/" in token:
        num, den = token.split("/")
        return f"{round(float(num) / float(den) * 100, 4):g}%"
    return spacing(token)


def color(token):
    """'slate-900', 'slate-900/80', 'white', '[#abc]', '[{prop}]' -> hui color string."""
    if token.startswith("[") and token.endswith("]"):
        return token[1:-1]
    opacity = None
    if "/" in token:
        token, op = token.rsplit("/", 1)
        opacity = int(op)
    if token in SPECIAL_COLORS:
        hexv = SPECIAL_COLORS[token]
    else:
        m = re.fullmatch(r"([a-z]+)-(\d+)", token)
        if not m or m.group(1) not in PALETTE or int(m.group(2)) not in PALETTE[m.group(1)]:
            raise UnknownClass(token)
        hexv = PALETTE[m.group(1)][int(m.group(2))]
    if opacity is not None:
        if not 0 <= opacity <= 100:
            raise UnknownClass(token)
        hexv = hexv[:6] + format(round(opacity * 255 / 100), "02x")
    return "#" + hexv


def is_color(token):
    try:
        color(token)
        return True
    except (UnknownClass, ValueError):
        return False


def translate_utility(cls):
    """Translate one class (without state prefix) into a list of (attr, value)."""
    if cls in ("flex", "grid"):
        return [("display", cls)]
    if cls == "hidden":
        return [("display", "none")]
    if cls == "flex-row":
        return [("flex_direction", "row")]
    if cls == "flex-col":
        return [("flex_direction", "column")]
    if cls == "flex-wrap":
        return [("flex_wrap", "wrap")]
    if cls == "flex-1":
        return [("flex_grow", "1"), ("flex_basis", "0px")]
    if cls == "grow":
        return [("flex_grow", "1")]
    if cls == "shrink-0":
        return [("flex_shrink", "0")]
    if cls in ("absolute", "relative"):
        return [("position", cls)]
    if cls == "aspect-square":
        return [("aspect_ratio", "1")]
    if cls == "overflow-hidden":
        return [("overflow", "hidden hidden")]
    if cls == "overflow-y-scroll":
        return [("overflow", "visible scroll")]
    if cls == "font-ui":
        return [("font", "fonts/Inter-Regular.otf")]
    if cls.startswith("text-") and cls[5:] in TEXT_ALIGN:
        return [("text_layout", TEXT_ALIGN[cls[5:]])]
    if cls == "content-start":
        return [("align_content", "flex_start")]
    if cls == "self-center":
        return [("align_self", "center")]
    if cls == "self-start":
        return [("align_self", "flex_start")]
    if cls == "border":
        return [("border", "1px")]
    if cls.startswith("items-") and cls[6:] in ALIGN:
        return [("align_items", ALIGN[cls[6:]])]
    if cls.startswith("justify-") and cls[8:] in JUSTIFY:
        return [("justify_content", JUSTIFY[cls[8:]])]

    m = re.fullmatch(r"gap(-x|-y)?-(.+)", cls)
    if m:
        attr = {None: ("row_gap", "column_gap"), "-x": ("column_gap",), "-y": ("row_gap",)}[m.group(1)]
        return [(a, spacing(m.group(2))) for a in attr]

    m = re.fullmatch(r"(p|m)([xytrbl]?)-(.+)", cls)
    if m:
        kind, side, token = m.groups()
        attr = "padding" if kind == "p" else "margin"
        v = spacing(token)
        idx = {"": (0, 1, 2, 3), "x": (1, 3), "y": (0, 2), "t": (0,), "r": (1,), "b": (2,), "l": (3,)}[side]
        return [(attr, {i: v for i in idx})]  # merged per-side by translate()

    m = re.fullmatch(r"(min-w|max-w|min-h|max-h|w|h)-(.+)", cls)
    if m:
        names = {"w": "width", "h": "height", "min-w": "min_width", "max-w": "max_width",
                 "min-h": "min_height", "max-h": "max_height"}
        return [(names[m.group(1)], size(m.group(2)))]

    m = re.fullmatch(r"(top|left|right|bottom)-(.+)", cls)
    if m:
        return [(m.group(1), size(m.group(2)))]

    m = re.fullmatch(r"grid-cols-(\d+)", cls)
    if m:
        return [("grid_template_columns", f"({m.group(1)}, 1fr)")]
    m = re.fullmatch(r"col-span-(\d+)", cls)
    if m:
        return [("grid_column", f"span({m.group(1)})")]

    m = re.fullmatch(r"rounded(?:-(.+))?", cls)
    if m:
        key = m.group(1) or ""
        if key not in RADII:
            raise UnknownClass(cls)
        return [("border_radius", f"{RADII[key]}px")]

    m = re.fullmatch(r"border-(\d+)", cls)
    if m:
        return [("border", f"{m.group(1)}px")]

    m = re.fullmatch(r"z-(\d+)", cls)
    if m:
        return [("zindex", m.group(1))]

    m = re.fullmatch(r"duration-(\d+)", cls)
    if m:
        # bevy_hui hover/press transitions are driven by `delay` (`duration` is the sprite animation).
        return [("delay", f"{int(m.group(1)) / 1000:g}s")]
    if cls in ("ease-in", "ease-out", "ease-in-out"):
        return [("ease", "cubic_" + cls[5:].replace("-", "_"))]

    if cls.startswith("bg-"):
        return [("background", color(cls[3:]))]
    if cls.startswith("text-"):
        rest = cls[5:]
        if rest in TEXT_SIZES:
            return [("font_size", str(TEXT_SIZES[rest]))]
        return [("font_color", color(rest))]
    if cls.startswith("border-") and is_color(cls[7:]):
        return [("border_color", color(cls[7:]))]
    if cls.startswith("tint-"):
        return [("image_color", color(cls[5:]))]
    raise UnknownClass(cls)


def _finalize_sides(sides):
    """{0..3: value} (top, right, bottom, left) -> hui rect string."""
    parts = [sides.get(i, "0px") for i in range(4)]
    if len(set(parts)) == 1:
        return parts[0]
    if parts[0] == parts[2] and parts[1] == parts[3]:
        return f"{parts[1]} {parts[0]}"  # hui 2-value form is "horizontal vertical"
    return " ".join(parts)


def translate(classes):
    """'p-4 hover:bg-slate-700' -> list of (attr, value); state prefix kept in attr."""
    out = {}
    for cls in classes.split():
        state = None
        if ":" in cls:
            state, cls = cls.split(":", 1)
            if state not in STATE_PREFIXES:
                raise UnknownClass(f"{state}:{cls}")
        for attr, value in translate_utility(cls):
            key = f"{state}:{attr}" if state else attr
            if isinstance(value, dict):  # padding/margin sides accumulate across classes
                out.setdefault(key, {}).update(value)
            else:
                out[key] = value
    return [(k, _finalize_sides(v) if isinstance(v, dict) else v) for k, v in out.items()]
