# Round 4 — sketch, extrude, revolve, sweep

Approved scope. Rounds 5–9 stay closed. Orbit, snap, world handles, baked angles, the view cube, and multi-select stay as they are. New bodies are still `#8A98A6`.

## Drawing

Sketch sits on the build plate. X is to the right, Z is depth, and a finished extrude grows up in Y. A click or a tap on the plate adds a point. A drag still orbits, and two fingers still pan and pinch. Points snap with the 1 mm grid when snap is on.

The profile is the outline. The path is a separate open chain, used only by sweep. Close is optional: Finish treats three or more points as a loop. Cancel drops the drawing and leaves an edited body unchanged. Edit sketch reopens the stored profile and path of the selected sketch body.

## The three finishes

- **Extrude.** A closed profile becomes a solid of the typed height (default 10 mm).
- **Revolve.** The profile spins around the plate line X = 0 (the Z axis) by the typed angle (default 360°). The profile must lie on one side of that axis.
- **Sweep.** The closed profile is carried along the path, staying perpendicular to it. A curved path makes a curved tube.

Each finish is one undo step. Redo puts it back.

## What is rejected

The core returns an error, and the page shows that sentence. It does not panic.

- Fewer than three profile points, or a profile that is not a simple loop.
- Zero area, or points that sit on top of each other.
- An edge that crosses another edge.
- A revolve profile that crosses the axis, or that has no radius.
- A sweep with fewer than two path points, a zero-length step, or a frame that collapses.

Caps are triangulated. Every supported solid is closed: each edge belongs to two faces, and normals point outward.

## Bake keeps the sketch

A baked rotation still ends at angles 0, 0, 0 and world-aligned handles. A sketch body keeps its profile, path, and operation. The plane the sketch was drawn in is turned with the body and stored again, then the solid is rebuilt from that record. A plain imported mesh or a library solid still becomes a bare mesh when it is baked. That limit is unchanged. After a bake, Edit sketch still opens the same outline.

## Still deferred

Capsule taper, polygon side counts other than 6, sphere side count, rounded-box radius, tube wall, star points, text, name tags, resize handles, and the protractor wheel. Sketch constraints, arcs, and a construction-line mode like the full TypeScript editor are not this round. The path and the profile are polylines.
