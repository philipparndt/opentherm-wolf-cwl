"""Convert STEP file to STL using FreeCAD."""
import sys
import FreeCAD
import Part
import MeshPart
import Mesh
import Import

step_file = sys.argv[1]
stl_file = sys.argv[2]

FreeCAD.newDocument("temp")
Import.insert(step_file, "temp")
doc = FreeCAD.getDocument("temp")
shapes = [o.Shape for o in doc.Objects if hasattr(o, "Shape")]
compound = Part.makeCompound(shapes)
mesh = MeshPart.meshFromShape(compound, LinearDeflection=0.05, AngularDeflection=0.3)

m = Mesh.Mesh(mesh)
m.removeDuplicatedFacets()
m.removeDuplicatedPoints()
m.harmonizeNormals()

m.write(stl_file)
