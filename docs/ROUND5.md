# Round 5 — booleans, groups, holes, fillets

Approved scope. Rounds 6–9 stay closed. Rounds 1–4 stay as they are: numeric transforms, 1 mm snap, world handles, baked angles, dimension lines, the view cube, multi-select, sketches, and STL. New bodies are still `#8A98A6`. Each new operation is one undo step.

## What the page does

The TypeScript app treats a body as a solid or a cut-out. A cut-out is drawn see-through. Grouping a cut-out with a solid removes that material, which is how a hole is made. Ungroup brings the original bodies back. Join, Subtract, and Intersect are separate tools that replace the selection with one mesh.

- **Hole.** Marks the selection as a cut-out, or clears that mark. The stored colour stays `#8A98A6`. The view draws a cut-out glassy and blue-tinted so it is obvious before it cuts anything.
- **Group.** Two or more bodies move, rotate, duplicate, and delete together. A click selects the group. Ungroup restores the members where they now sit. If any member is a hole, the group shows the solids with the holes cut out, and ungroup restores the uncut bodies. A group of only holes is rejected.
- **Join.** Union of the selected bodies. They are replaced by one mesh.
- **Subtract.** The other selected bodies are cut out of the primary (the one in the properties). The inputs are replaced by one mesh.
- **Intersect.** The volume they share. No overlap is an error, not an empty object.
- **Fillet.** A typed radius rounds every edge of a box. The radius has to be at least 0.2 mm and at least 0.2 mm short of half the smallest side. Anything that is not a box is rejected. One edge, a cylinder, a sketch, and an imported mesh are not filleted in this round.

## Geometry

Booleans run in the core crate on closed triangle meshes (the same BSP clip used by the small csg.js kernel). The result must be a closed solid. An open input, a body with no triangles, a result that is not closed, a result with no volume, or a body over 12 000 triangles is an error string. Nothing panics.

A baked rotation still ends at 0, 0, 0. A sketch inside a group keeps its profile. A box, a library solid, or an imported mesh still becomes a bare mesh when its own rotation is baked. That limit is unchanged.

## Still deferred

Bundle, multicolour groups, Edit group (E) / Done, hull, loft, split, mirror, chamfer, shell, offset, hole patterns, and a fillet on a single clicked edge. Nested groups are not a separate mode: a group can be grouped again, and ungroup peels one level. Rounds 6–9 are not started.
