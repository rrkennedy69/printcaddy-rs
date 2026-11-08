# Round 3 — handles, dimensions, view cube, multi-select

Approved scope. Rounds 4–9 stay closed. Empty-space drag still orbits, and two-finger pan and pinch stay as they are. The default body colour stays `#8A98A6`.

## What this round adds

- World-axis move arrows on the selection: red along X, green along Y, blue along Z.
- Rotation arcs: red (X) and green (Y) at the top of the selection, blue (Z) at the bottom. They are drawn in the screen overlay after the mesh, so a body cannot cover them.
- Dimension lines for the selection’s world-axis width, depth, and height, in millimetres.
- A view cube in the top-right of the viewport. A click on a face jumps the camera. A drag on the cube orbits.
- Multi-select. Shift-click toggles a body. The “Add to selection” switch does the same for a finger. A plain click replaces the selection. A plain click on empty space clears it. Dragging one selected body moves the whole selection.

## Rotation is baked back to zero

Any committed rotation is written into the vertices and the numeric angles return to 0, 0, 0. Handles and dimension lines then sit on the plate axes again, around the new world-aligned box.

This includes a document that still carries a non-zero angle from an older in-memory state: after undo, redo, or when the app adopts a document, `bake_rotations` runs. A body with an angle becomes a mesh whose bottom-centre is the world box, and its angles are zero. During a drag the angle is shown live. The bake runs on release, inside the same undo step as the drag. Undo restores the body from before that rotation.

Typed rotation works the same way. The number is live while the field is dragged, and the release bakes it.

Move drags and axis drags still snap to the 1 mm grid when snap is on. One drag is one undo step. A second finger still puts a move back and drops that step.

## What stays deferred

These round-2 gaps stay open: a capsule with one smaller end, polygon side counts other than 6, a sphere side-count control, editable rounded-box radius, tube wall, and star point count, plus text and name tags. Resize handles, the numeric angle editor on the arc, and the protractor wheel are not this round.
