#!/usr/bin/env python3
"""Convert STEP files to L3D format (OBJ mesh inside ZIP).

Usage:
    python3 scripts/step_to_l3d.py input.step [output.l3d] [--tolerance 1.0] [--name "Product"]

Requires: cadquery (pip install cadquery)
"""

import argparse
import io
import sys
import zipfile
from datetime import datetime, timezone
from pathlib import Path

try:
    import cadquery as cq
    from OCP.BRep import BRep_Tool
    from OCP.BRepMesh import BRepMesh_IncrementalMesh
    from OCP.TopAbs import TopAbs_FACE
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopLoc import TopLoc_Location
    from OCP.TopoDS import TopoDS
except ImportError:
    print("Error: cadquery + OCP required. Install with: pip install cadquery", file=sys.stderr)
    sys.exit(1)


def tessellate_step(step_path: str, tolerance: float = 1.0):
    """Load STEP file and tessellate to vertices + triangles."""
    shape = cq.importers.importStep(step_path).val()

    # Tessellate
    mesh = BRepMesh_IncrementalMesh(shape.wrapped, tolerance, False, tolerance * 0.5, True)
    mesh.Perform()

    vertices = []
    faces = []
    vertex_offset = 0

    explorer = TopExp_Explorer(shape.wrapped, TopAbs_FACE)
    while explorer.More():
        face = TopoDS.Face_s(explorer.Current())
        location = TopLoc_Location()
        triangulation = BRep_Tool.Triangulation_s(face, location)

        if triangulation is not None:
            trsf = location.Transformation()
            n_verts = triangulation.NbNodes()

            # Extract vertices (OCC uses 1-based indexing)
            for i in range(1, n_verts + 1):
                pt = triangulation.Node(i)
                # Apply location transform
                pt.Transform(trsf)
                vertices.append((pt.X(), pt.Y(), pt.Z()))

            # Extract triangles
            n_tris = triangulation.NbTriangles()
            for i in range(1, n_tris + 1):
                tri = triangulation.Triangle(i)
                i1, i2, i3 = tri.Get()
                # OBJ uses 1-based indexing; offset by accumulated vertices
                faces.append((
                    i1 + vertex_offset,
                    i2 + vertex_offset,
                    i3 + vertex_offset,
                ))

            vertex_offset += n_verts

        explorer.Next()

    return vertices, faces


def compute_normals(vertices, faces):
    """Compute per-vertex normals via area-weighted face normal averaging."""
    import math

    normals = [[0.0, 0.0, 0.0] for _ in vertices]

    for i1, i2, i3 in faces:
        # OBJ indices are 1-based
        v0 = vertices[i1 - 1]
        v1 = vertices[i2 - 1]
        v2 = vertices[i3 - 1]
        # Edge vectors
        e1 = (v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2])
        e2 = (v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2])
        # Cross product (area-weighted normal)
        nx = e1[1] * e2[2] - e1[2] * e2[1]
        ny = e1[2] * e2[0] - e1[0] * e2[2]
        nz = e1[0] * e2[1] - e1[1] * e2[0]
        for idx in (i1 - 1, i2 - 1, i3 - 1):
            normals[idx][0] += nx
            normals[idx][1] += ny
            normals[idx][2] += nz

    # Normalize
    for n in normals:
        length = math.sqrt(n[0] ** 2 + n[1] ** 2 + n[2] ** 2)
        if length > 0:
            n[0] /= length
            n[1] /= length
            n[2] /= length
        else:
            n[2] = 1.0  # default up

    return normals


def write_obj(vertices, faces) -> str:
    """Write OBJ content as string with normals. Coordinates in mm."""
    normals = compute_normals(vertices, faces)
    lines = [f"# Tessellated STEP geometry ({len(vertices)} vertices, {len(faces)} faces)"]
    for x, y, z in vertices:
        lines.append(f"v {x:.6f} {y:.6f} {z:.6f}")
    for nx, ny, nz in normals:
        lines.append(f"vn {nx:.6f} {ny:.6f} {nz:.6f}")
    for i1, i2, i3 in faces:
        lines.append(f"f {i1}//{i1} {i2}//{i2} {i3}//{i3}")
    lines.append("")
    return "\n".join(lines)


def make_structure_xml(name: str) -> str:
    """Generate L3D structure.xml."""
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    return f"""\
<?xml version="1.0" encoding="UTF-8"?>
<Luminaire>
  <Header>
    <Name>{name}</Name>
    <CreatedWithApplication>step_to_l3d.py</CreatedWithApplication>
    <CreationTimeCode>{timestamp}</CreationTimeCode>
    <FormatVersion major="0" minor="11"/>
  </Header>
  <GeometryDefinitions>
    <GeometryFileDefinition id="geom_1" filename="housing.obj" units="mm"/>
  </GeometryDefinitions>
  <Structure>
    <Geometry partName="Housing">
      <Position x="0" y="0" z="0"/>
      <Rotation x="0" y="0" z="0"/>
      <GeometryReference geometryId="geom_1"/>
    </Geometry>
  </Structure>
</Luminaire>
"""


def build_l3d(obj_content: str, name: str) -> bytes:
    """Package OBJ + structure.xml into L3D ZIP."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("structure.xml", make_structure_xml(name))
        zf.writestr("geom_1/housing.obj", obj_content)
    return buf.getvalue()


def main():
    parser = argparse.ArgumentParser(description="Convert STEP to L3D (OBJ mesh in ZIP)")
    parser.add_argument("input", help="Input STEP file path")
    parser.add_argument("output", nargs="?", help="Output L3D file path (default: {stem}.l3d)")
    parser.add_argument("--tolerance", type=float, default=1.0,
                        help="Tessellation tolerance in mm (default: 1.0)")
    parser.add_argument("--name", default=None,
                        help="Product name for L3D header (default: input filename stem)")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    output_path = Path(args.output) if args.output else input_path.with_suffix(".l3d")
    name = args.name or input_path.stem

    print(f"Loading STEP: {input_path}")
    vertices, faces = tessellate_step(str(input_path), args.tolerance)
    print(f"Tessellated: {len(vertices)} vertices, {len(faces)} faces (tolerance={args.tolerance}mm)")

    if not vertices:
        print("Error: no geometry extracted from STEP file", file=sys.stderr)
        sys.exit(1)

    obj_content = write_obj(vertices, faces)
    l3d_data = build_l3d(obj_content, name)

    output_path.write_bytes(l3d_data)
    print(f"Written: {output_path} ({len(l3d_data)} bytes)")


if __name__ == "__main__":
    main()
