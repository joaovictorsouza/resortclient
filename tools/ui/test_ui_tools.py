"""Testes do gerador de UI: `python3 -m unittest discover -s tools/ui -p "test_*.py"`."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))

import build_ui  # noqa: E402
import tailwind as tw  # noqa: E402


class TailwindTests(unittest.TestCase):
    def d(self, classes):
        return dict(tw.translate(classes))

    def test_spacing_and_sizes(self):
        self.assertEqual(self.d("p-4"), {"padding": "16px"})
        self.assertEqual(self.d("px-2 py-1"), {"padding": "8px 4px"})
        self.assertEqual(self.d("px-5 pt-1 py-4")["padding"], "20px 16px")
        self.assertEqual(self.d("w-full h-[300px]"), {"width": "100%", "height": "300px"})
        self.assertEqual(self.d("gap-2"), {"row_gap": "8px", "column_gap": "8px"})
        self.assertEqual(self.d("pt-2"), {"padding": "8px 0px 0px 0px"})

    def test_colors_and_opacity(self):
        self.assertEqual(self.d("bg-slate-900"), {"background": "#0f172a"})
        self.assertEqual(self.d("bg-slate-900/50")["background"], "#0f172a80")
        self.assertEqual(self.d("text-white")["font_color"], "#ffffff")
        self.assertEqual(self.d("border-indigo-500")["border_color"], "#6366f1")
        self.assertEqual(self.d("bg-[{primary}]"), {"background": "{primary}"})

    def test_align_content(self):
        self.assertEqual(self.d("content-start"), {"align_content": "flex_start"})

    def test_text_size_vs_color(self):
        self.assertEqual(self.d("text-sm"), {"font_size": "14"})

    def test_layout(self):
        self.assertEqual(self.d("flex flex-col items-center justify-between"),
                         {"display": "flex", "flex_direction": "column", "align_items": "center",
                          "justify_content": "space_between"})
        self.assertEqual(self.d("grid grid-cols-4")["grid_template_columns"], "(4, 1fr)")
        self.assertEqual(self.d("rounded-xl border border-2")["border"], "2px")
        self.assertEqual(self.d("rounded-xl")["border_radius"], "12px")

    def test_state_prefix(self):
        self.assertEqual(self.d("hover:bg-slate-700"), {"hover:background": "#334155"})
        self.assertEqual(self.d("active:border-indigo-500"), {"active:border_color": "#6366f1"})

    def test_unknown_class_is_an_error(self):
        for bad in ("banana", "bg-slate-123", "hover2:p-1", "rounded-9xl"):
            with self.assertRaises(tw.UnknownClass, msg=bad):
                tw.translate(bad)


class BuildTests(unittest.TestCase):
    def test_class_is_expanded_in_place(self):
        out = build_ui.convert('<node class="p-4 bg-white" id="x">\n  <text>hi</text>\n</node>')
        self.assertNotIn("class=", out)
        self.assertIn('padding="16px"', out)
        self.assertIn('background="#ffffff"', out)
        self.assertIn('id="x"', out)
        self.assertIn("<text>hi</text>", out)

    def test_self_closing_tag(self):
        out = build_ui.convert('<image class="w-6 h-6" src="a.png"/>')
        self.assertTrue(out.endswith("/>"))
        self.assertIn('width="24px"', out)

    def test_conflict_with_explicit_attribute(self):
        with self.assertRaises(ValueError):
            build_ui.convert('<node class="p-4" padding="1px">')

    def test_tags_without_class_untouched(self):
        src = '<node id="a"><text font_size="12">x</text></node>'
        self.assertEqual(build_ui.convert(src), src)


if __name__ == "__main__":
    unittest.main()
