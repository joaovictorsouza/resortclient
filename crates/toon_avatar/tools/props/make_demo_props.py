"""
make_demo_props.py
------------------
Blender-only: writes two small demo props, a cup and a can, as GLB files for the demonstrator.
Props follow the convention the avatar's grip expects: authored in metres, origin at the centre of
the object, +Y up (glTF), so a prop sits in the hand without any per-prop adjustment.

Usage: Blender -b --python make_demo_props.py -- <out_dir>
"""
import math
import sys

import bpy


def material(name, rgba, metallic=0.0, roughness=0.6):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = rgba
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def cylinder(name, radius, depth, z, mat, verts=32, radius_top=None):
    bpy.ops.mesh.primitive_cone_add(vertices=verts, radius1=radius, radius2=radius_top if radius_top is not None else radius, depth=depth, location=(0, 0, z))
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)
    bpy.ops.object.shade_smooth()
    return obj


def export(path):
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_yup=True, use_selection=False)


def clear():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def main():
    out = sys.argv[sys.argv.index("--") + 1]
    # Cup: 9 cm tall, slightly wider at the top, with a handle. Blender is Z up; the exporter turns it into Y up.
    clear()
    cup_mat = material("cup", (0.95, 0.95, 0.92, 1.0))
    cylinder("cup_body", 0.030, 0.090, 0.0, cup_mat, radius_top=0.036)
    bpy.ops.mesh.primitive_torus_add(location=(0.040, 0, 0.005), rotation=(math.radians(90), 0, 0), major_radius=0.022, minor_radius=0.005)
    handle = bpy.context.object
    handle.data.materials.append(cup_mat)
    coffee = material("coffee", (0.12, 0.06, 0.03, 1.0))
    cylinder("cup_liquid", 0.033, 0.002, 0.040, coffee)
    export(f"{out}/cup.glb")
    # Can: 12.2 cm tall, 6.6 cm wide, aluminium.
    clear()
    can_mat = material("can", (0.75, 0.75, 0.78, 1.0), metallic=0.9, roughness=0.35)
    band = material("can_band", (0.8, 0.1, 0.1, 1.0), roughness=0.5)
    cylinder("can_body", 0.033, 0.122, 0.0, can_mat)
    cylinder("can_band", 0.0332, 0.040, 0.0, band)
    cylinder("can_top", 0.029, 0.004, 0.061, can_mat)
    export(f"{out}/can.glb")
    print("wrote", out)


if __name__ == "__main__":
    main()
