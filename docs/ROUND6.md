# Round 6 — 3MF, OBJ, SVG, DXF, STEP

Approved scope. Rounds 7 and 8 stay closed. The AI helper stays out. Rounds 1–5 and 9 stay as they are. New bodies are still `#8A98A6`.

## Units

Exchange files are millimetres, Z-up, the same way the TypeScript app writes STL, 3MF, and OBJ. The document stays Y-up. A point `(x, y, z)` in the file is `(x, z, −y)` on the plate.

## What each format does

- **3MF.** Import and export. Every body is its own object, with its name and `#rrggbb` colour. The package is a zip; compressed entries are inflated. A missing colour becomes `#8A98A6`.
- **OBJ.** Import and export. A comment says `Z-up, millimetres`. Colour is `v x y z r g b` on each vertex, one file, no material library.
- **SVG.** Import only, as a sketch outline the existing extrude, revolve, and sweep tools already edit. Rectangles, polygons, polylines, and paths of M, L, H, V, and Z are read as millimetres. Curves are flattened to lines. Text and images are ignored. One user unit is one millimetre.
- **DXF.** Import only, as the same kind of sketch. LINE, LWPOLYLINE, and CIRCLE in model space. `$INSUNITS` of inches is scaled to millimetres; a missing unit is millimetres. A circle becomes a 24-sided outline. Text, hatches, and dimensions are skipped.
- **STEP.** Import and export of one subset: triangulated faces, `POLY_LOOP` of `CARTESIAN_POINT`, Z-up. The file names millimetres with `SI_UNIT(.MILLI.,.METRE.)`. That is what this build writes for every mesh. There is no shell or solid entity around the loops. Exact B-rep (NURBS, `ADVANCED_BREP`, threads, gears) is rejected with a sentence. STEP does not carry colour here; an import uses `#8A98A6`.

The TypeScript app writes analytic STEP for boxes, cylinders, spheres, and cones through OpenCascade, and it refuses meshes. This build does the opposite subset: every mesh goes out as triangles, and analytic solids from other CAD do not come in.

## Limits

A file over 32 MB is refused. A mesh over 200 000 triangles is refused. An SVG or DXF over 2 MB, or an outline over 8 000 points, is refused. A bad file returns an error and does not panic.

## Still deferred

Bambu 3MF, the slicer, top-view SVG and DXF export, and analytic STEP.
