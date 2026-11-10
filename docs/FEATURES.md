# PrintCaddy feature inventory

Source of truth: the TypeScript app published at <https://rrkennedy69.github.io/> (source snapshot in that repo's `source/printcad-source.zip`, app name layerling / PrintCaddy). This file is the parity list for the Rust rewrite.

A checked box means the Rust app at <https://rrkennedy69.github.io/printcaddy-rs/> does that thing. Everything else is still to build.

## Milestone 1 (this round)

- [x] 3D viewport in the browser
- [x] Orbit, pan, and zoom with the mouse (drag empty space to orbit, right or middle drag to pan, wheel to zoom)
- [x] Orbit, pan, and zoom with touch (one finger orbits, two fingers pan and pinch-zoom; a second finger puts a one-finger move back)
- [x] Build plate grid (256 × 256 mm, 10 mm lines, 50 mm majors, border, and a short X/Y/Z triad)
- [x] Add a box (20 × 20 × 20 mm)
- [x] Add a cylinder (radius 10 mm, height 20 mm)
- [x] Select a body (click or tap; click empty space clears the selection)
- [x] Move a selected body on the plate by dragging it
- [x] STL import (binary and ASCII; the mesh is centred in XZ and seated on the plate)
- [x] STL export (the selection when one exists, otherwise every body)
- [x] Default body colour `#8A98A6`

## Milestone 2

- [x] Undo and redo (buttons and Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z). One drag is one step. A second finger puts the move back and does not leave an undo step.
- [x] Numeric properties: position X Y Z, rotation X Y Z in degrees about the body's centre, and width, depth, height
- [x] Grid snap while dragging, 1 mm by default, with a switch to turn it off. Typed numbers stay exact.
- [x] Delete and duplicate
- [x] Simple solids at the current app's starting sizes: box 20×20×20, rounded box 40×30×20 with 5 mm corners, cylinder 20×20×20, capsule 40×20×20, ellipse 26×16×20, hexagon polygon, sphere 22, cone, pyramid and wedge 20, round roof 20×20×10, half sphere 22×22×11, paraboloid 22×22×20, torus 22×22×5, tube 20×20×20 with a 4 mm wall, five-point star 40×40×10, heart 40×40×10. New bodies are still `#8A98A6`.

## Milestone 3

- [x] Move arrows on the world axes: red X, green Y, blue Z. A drag snaps with the 1 mm grid and is one undo step.
- [x] Rotation arcs. Red (X) and green (Y) sit on the top of the selection. Blue (Z) sits on the bottom. They are drawn after the mesh, so the body cannot cover the grab point.
- [x] A finished rotation is baked into the vertices and the numeric angles return to 0, 0, 0, including a body that still carried an angle. Handles and dimension lines then follow the plate axes.
- [x] Dimension lines for the selection’s world width, depth, and height.
- [x] View cube in the top-right. Click a face to jump there. Drag it to orbit.
- [x] Multi-select. Shift-click, or Add to selection, toggles a body. A drag moves every selected body together.
- [ ] Resize handles, the on-arc numeric angle editor, and the protractor wheel stay later.

## Shape library

The lines below are the full tool, including every special parameter. Milestone 2 places the simple solids at the library's starting size and edits position, size, and rotation. A line stays open when a special parameter (a taper, a side count, a font) is still missing.

- [x] Box
- [x] Rounded box
- [x] Cylinder
- [ ] Capsule (slot), including one end smaller
- [x] Ellipse
- [ ] Polygon prism, 3 to 24 sides
- [ ] Sphere, with a side-count control
- [x] Cone
- [x] Pyramid
- [x] Wedge
- [x] Round roof
- [x] Half sphere
- [x] Paraboloid
- [x] Torus
- [ ] Loft (square-to-round and similar, solid or walled, straight or eased, twist, tilted top)
- [ ] Vase (body, cross-section including a drawn one, a second section to blend, twist, patterns, drain holes, foot, saucer, lid, solid or hollow)
- [x] Tube
- [ ] Bent tube
- [ ] Counterbore
- [ ] Countersink
- [ ] Double countersink
- [ ] Teardrop hole
- [ ] Text (fill modes, stacked letters, right-to-left Arabic and Hebrew, joining Arabic)
- [ ] Thread (rod, socket screw, countersunk, hex head, hex nut, tapped hole as a cutter)
- [ ] Thread sizes M2–M12, UNC/UNF #4 to 1 inch, Whitworth pipe G1/16–G4, custom diameter and pitch, left-hand, TPI for inch and G, end chamfer
- [ ] Spring
- [ ] Gridfinity bin and baseplate, including half-grid
- [ ] Gear (spur, helical, herringbone, bevel, ring, rack; involute or round teeth; module, tooth count, pressure angle, backlash; centre distance for two selected gears)
- [ ] Knurl (straight, crossed, round)
- [ ] Hinge (print-in-place, screw holes, finger hinge)
- [ ] Dovetail
- [x] Star
- [x] Heart
- [ ] Crescent
- [ ] Ruler shape
- [ ] Name tag (coloured layers, key-ring hole, rounded or chamfered letters)
- [ ] Icon tag (SVG, coloured layers, count, "as many as fit")
- [ ] QR code
- [ ] My shapes / custom shapes, dragged in from the library, including the shared server store
- [ ] Cookie cutter from a sketch, SVG, or footprint (wall, rim, bridges, mirrored outline, cutting edge)

## Sketching

Round 4 draws a polyline on the plate and finishes it. Arcs, curves, constraints, construction lines, and a helical revolve are still only in the TypeScript app.

- [x] Draw a profile by clicking or tapping the plate (drag still orbits; points snap to 1 mm)
- [x] Separate open path for sweep
- [x] Extrude to a typed height, revolve by a typed angle around X = 0, sweep the profile along the path
- [x] Cancel a drawing, and edit the stored profile and path of a sketch body
- [x] Reject a short, flat, duplicated, or self-crossing outline, a revolve that crosses the axis, and a sweep with no path, and say why
- [ ] Sketch mode, separate from the 3D toolbar
- [ ] Line, arc, Bézier, smooth curve
- [ ] Construction lines
- [ ] Draw from the middle
- [ ] Ready-made sketch shapes
- [ ] Length and angle while drawing (`50<30`), Shift snaps to 15°
- [ ] Typed lengths, angles, and sizes, including sums such as `60+12.5` in number fields
- [ ] Corner fillet and chamfer
- [ ] Snap to other bodies' outlines
- [ ] Mirror
- [ ] Select, refine, erase
- [ ] Picture as a tracing template
- [ ] Extrude, revolve (including a pitched helical revolve), sweep along a path — polyline extrude, revolve, and sweep are in; the helical pitch is not
- [ ] Stroke build (frame inside, outside, or centred; open stroke with flat, square, or round ends)
- [ ] Fill and silhouette modes
- [ ] Slanted walls for stamps
- [ ] Sketch undo and redo, copy, paste, duplicate, delete
- [x] Edit an existing sketch body

## Modelling tools

- [ ] Copy, paste, duplicate, delete
- [x] Undo and redo
- [x] Solid and hole — a hole is see-through until it is grouped with a solid, which cuts it out; ungroup restores the bodies
- [x] Group and ungroup — members move together; a group can contain a group; Edit group (E) / Done is not in yet
- [ ] Bundle (parts move together, export as separate coloured bodies)
- [ ] Multicolour groups that survive holes and export
- [x] Intersection — Join, Subtract, and Intersect replace the selection with one closed mesh; the common-part preview is not in yet
- [ ] Cut out one body from another with a gap
- [ ] Hull
- [ ] Loft two sketches or SVGs
- [ ] Split with a plane, including angled cuts and "pick face"; hollow bodies keep their cavity
- [ ] Mirror
- [ ] Rotation pivot that stays with the body, typed or dragged, snapping to corners and edge midpoints
- [ ] Array (row and circle)
- [ ] Hole pattern on a face (round, square, rectangular, hex, slot; grid or staggered; margin; through or depth)
- [ ] Face tool (push or pocket, inset, corner radius, depth, preview, including on imported STL)
- [ ] Flexi joints (print-in-place links, clearance, swing, joint size)
- [x] Fillet a box — every edge, typed radius from 0.2 mm up to 0.2 mm short of half the smallest side. One clicked edge, chamfer, and editing the fillet later are not in yet
- [ ] Hollow / shell, open on chosen sides or closed
- [ ] Offset (grow or shrink 0.01–10 mm)
- [ ] Delete face
- [ ] Simplify mesh
- [ ] Surface texture (grooves, knurl, honeycomb, bumps, Voronoi, bricks, knitting, rope, basket weave, carbon fibre, bark, wood grain, cracks, image height map, text)
- [ ] Wrap an SVG or text around a cylinder, raised or engraved
- [ ] Align
- [x] Snap to grid
- [ ] Drop to the workplane
- [ ] Lay flat
- [ ] Mate faces (touching or flush, optional gap, hole or pin on an axis)
- [ ] Centre on the workplane
- [ ] Carry onto a face
- [ ] Scale by percent, and scale from the handles with an optional proportion lock
- [ ] Snap to other shapes while moving (edges and centres, guide line; Alt pauses; Shift locks an axis; Alt from the start drags a copy)
- [ ] Set down on the face under the pointer, including sloped faces; C repeats it
- [ ] Typed position from centre or from the left and front edges
- [ ] Park parts beside the plate
- [ ] Hide, show, lock
- [ ] Object list with search
- [ ] Right-click menu (duplicate, hole or solid, group, chamfer, fillet, hollow, mirror, scale, hide, lock, drop, delete)
- [ ] History view (slider through earlier states, export that state or start a project from it) without editing it
- [ ] Gravity
- [ ] Collision

## Move, rotate, and measure handles

These are the behaviours to match, not just the presence of a widget.

- [ ] Corner and edge resize handles, and the top height handle
- [ ] Lift arrow, clamped so it stays near a tall body, with a typed lift after the drag
- [ ] Rotate arrows, one per axis, drawn as an arc of that axis's circle in perspective
- [ ] Arrow placement: each arrow stands out from the face, about 38 screen pixels past the dimension labels, and bows at least a few pixels so an edge-on circle still looks like an arrow
- [ ] Rotation drag about the axis, from the 3D hit on the rotation plane, with a screen-space fallback
- [ ] Snap while rotating: 22.5° inside the protractor wheel, 1° outside it, and the finer Shift steps; changing the step mid-drag does not jump the body
- [ ] Rotation click versus drag: a plain click opens the numeric angle (so an axis can be typed, including back to 0) and does not apply a turn; a real drag suppresses that editor for a moment and bakes the whole turn into one undo step
- [ ] Custom pivot: rotating around it does not grid-snap the centre off the axis
- [ ] A multi-body turn stays rigid (no per-body grid snap that stalls the ones at the edge)
- [ ] Dimension lines for width, depth, and height, including always-visible labels on a single selection and the side that faces the camera
- [ ] Dimension lines that stay on the body (they used to drift up and left)
- [ ] Typed dimensions, including formulas, and the framing-square midpoint mode
- [ ] Move-dimension readout while a body is dragged
- [ ] Tape measure, including marking the middle of a measurement
- [ ] Framing square (place, drag, click to rotate 90°, measure to a centre, snap to a corner)
- [ ] Ruler
- [ ] Notes on the workplane and on sketches
- [ ] Reference points (centre, corners, edge midpoints; not printed; saved; draggable)
- [ ] Reference images

## View

- [ ] Perspective and orthographic (O), keeping direction and framing
- [ ] View cube: drag to orbit, click a face to jump, click a corner or edge for that view, keys 1–6 and Shift+1–6 to frame the selection
- [ ] View cube position: any corner, and the camera controls move to the vacated top-left
- [ ] Four saved views per design (Shift+7/8/9/0 to save, the key alone to restore), stored in the project file
- [ ] Hide the plate to look from below
- [ ] Second view in its own window, with its own camera, cube, and camera bar, remembering where it was
- [ ] Section view along X, Y, or Z with coarse and fine sliders; export is uncut; section SVG at 1:1; measure wall thickness on the cut
- [ ] Layer view: a vertical slider that cuts away everything above a height, Bambu-style, readout in mm and approximate layer number, without changing the model
- [ ] Workplane on a face (W), on a corner, along an edge, or on the empty plate; the eye hides the plane while it still applies
- [ ] Grid and snapping, including imperial fractions, custom snap units (whole, half, quarter), and darker lines every centimetre in their own colour
- [ ] Axis arrows at the plate's front-left, matching Bambu Studio / OrcaSlicer, with a setting to hide them
- [ ] Light and shadows: contrast, strength, softness, direction and height, including straight above; a Tinkercad-look preset
- [ ] Fast mode (no shadows, no edge lines, no camera inertia, screen-resolution drawing) and a separate inertia switch
- [ ] Overhang hatching at 45° or the printer's angle
- [ ] Double-click zoom
- [ ] Zoom toward the pointer
- [ ] Panels that drag by the title bar and dock on double-click
- [ ] Full screen
- [ ] Command search (Ctrl+K) in English and German, with shortcuts
- [ ] Interactive help (Shift+F1, then click a control)

## Files, import, and export

- [ ] STL import
- [ ] STL export
- [x] OBJ import and export, Z-up millimetres, vertex colours. A Tinkercad ZIP and `.mtl` are not read
- [x] 3MF import and export, one object per body with its name and colour. Bambu project 3MF is not this file
- [x] STEP import and export of triangulated `POLY_LOOP` faces, millimetres, Z-up, no colour. Exact B-rep is refused
- [x] SVG import as a sketch outline (rectangles, polygons, polylines, and paths; curves are flattened). Stroke styles and cookie-cutters are not in
- [ ] SVG export of the top view, with optional stencil bridges
- [x] DXF import of LINE, LWPOLYLINE, and CIRCLE as a sketch, inches scaled to millimetres. True arcs and holes-as-holes stay later
- [ ] DXF export with true arcs and circles
- [ ] PNG of the view, optional 2× and transparent background
- [ ] Drawing export to PDF or SVG: front, top, and right, first-angle, bordered sheet, title block, overall dimensions, hidden edges, hole tables
- [ ] Bambu 3MF export (plates, colours, filaments, slice settings)
- [ ] Send to Bambu Studio: build that 3MF and open the share sheet on iPad or download it on a computer
- [x] Projects in this browser: save, open, rename, delete, and export or import a project file. The file keeps bodies, sketches, groups, holes, and colours. Undo history is not in the file.
- [ ] Project file `.lyl` (history, sketches, groups), with Ctrl+S back to the chosen file in Chrome and Edge
- [ ] Older `.skf` files still open
- [ ] Design folders on the start page, search, multi-delete
- [ ] Thumbnails
- [ ] Attachments inside the project (md, txt, png, jpg, webp, pdf; not svg)
- [ ] Backup of all designs, including custom shapes
- [ ] Fonts: TTF, OTF, WOFF, and in Chrome and Edge a font installed on the computer; the design stores only the outlines it uses
- [ ] Mesh simplify after import
- [ ] Coloured import split into one body per colour
- [ ] Bug-report ZIP (the `.lyl` plus version, browser, and recent messages)

## Plates, filaments, materials, slicer

- [ ] Printer presets (the current list is more than 220) and a plate of that size
- [ ] Printer name and build volume in the corner, with a switch to hide them and the design name
- [ ] Warning when a body crosses the plate edge
- [ ] Extra build plates, laid out 3 per row (`PLATES_PER_ROW = 3`), 64 mm gap, plate 4 under plate 1 toward the viewer
- [ ] Legacy one-row 20 mm layout migrated onto the grid once per design
- [ ] Per-plate delete (objects move to the floor beside the plates, later plates slide over) and lock
- [ ] Objects dropped onto a locked plate become locked
- [ ] Bambu filament catalogue and colours, used by the colour picker, the slicer slots, and 3MF export
- [ ] Default filament PLA Basic (`GFA00`) where a Bambu material is required
- [ ] Loaded filaments: import the filaments actually loaded on the printer (colour, type, preset name, id) from a Bambu settings file, edited under Settings → Loaded filaments
- [ ] Materials with nozzle, bed, density, and fan: PLA, PETG, PET, ABS, ASA, TPU, PC, PA, PVA
- [ ] Slicer page in the Bambu Studio prepare / preview layout
- [ ] Presets 0.28 mm extra draft, 0.20 mm standard, 0.12 mm fine
- [ ] Process tabs: quality, strength, speed, support, others (layer height, walls, infill and pattern, seam, speeds, tree or normal supports, brim or skirt, ironing, nozzle, retraction)
- [ ] Import a Bambu profile or settings file into the slicer
- [ ] Slice with the wasm slicer engine, then preview toolpaths and a layer slider
- [ ] Print time, filament length, and weight
- [ ] Slice, then Send to Bambu Studio or export the 3MF from the slicer page
- [ ] Volume, weight, and filament length in the export window before slicing

## Themes, settings, mobile, updates

- [x] Themes: Light, Dark, Graphite, Night Vision, Thermal, X-Ray, Blocks, Forge, Works. System theme follows the browser later.
- [x] X-Ray palette on the panels, the plate, and see-through bodies
- [ ] Settings sections: Appearance, Measurement, Workplane, Shape defaults, History, Mobile & touch, Loaded filaments, Keyboard shortcuts — theme and Mobile mode are in; the other sections are not
- [x] Mobile mode enlarges the handles and the buttons. Long-press delay and auto-open properties stay later.
- [ ] Simple mode
- [ ] Pencil / stylus mode
- [ ] Units: millimetres, or inches as fractions or decimals, with the grid drawn in inches
- [ ] Keyboard shortcut editor
- [ ] View-cube corner, light, fast mode, snap, and the other appearance and workplane switches
- [ ] Language menu: English, Czech, Danish, German, Spanish, French, Italian, Hungarian, Dutch, Polish, Portuguese (Portugal and Brazil), Slovenian, Turkish, Russian, Arabic, Japanese, Chinese (simplified and traditional), plus the in-app guide in those languages
- [x] Check for updates in the bottom-left corner. It compares this page with the published build and can reload. The installed iPad app's update row is not this button.
- [ ] Update notice on the published site, offering save and reload and listing what is new
- [ ] Install as an app / service worker / offline
- [ ] Tablet behaviour from the guide: one finger works like the left button, two fingers belong to the view, a second finger cancels the first finger's edit, an on-screen orbit switch, number fields select their value, a larger finger reach than the mouse, stylus preferred
- [ ] Touch multi-select
- [ ] Welcome, what's new, and the user guide

## Accounts

- [ ] Google Drive save and open is in the Projects window and uses the TypeScript site's existing client. Sign-in stays blocked until that client allows https://rrkennedy69.github.io as a JavaScript origin. Merge and a settings slot are not in.
- [ ] Shared designs on a server folder
- [ ] Start page, design tabs, and home

## Explicitly out of the Rust app until the matching round

The live TypeScript site is unchanged and remains the production app. The AI helper is not part of this rewrite.
