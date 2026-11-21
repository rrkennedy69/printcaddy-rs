# Parity

Round G turns on the live editing tools that were grey.

Copy and paste (Ctrl+C / Ctrl+V) keep a clipboard in the page. Paste moves each body 12 mm in X and Z, and not past 10 mm outside the plate, as one undo step. History lists the steps and a click jumps back to one. The object list selects, renames, locks, and hides. Hide selected opens Hide, Show selected, and Show all. Align lines the other selected bodies up with the last one: left/centre/right, front/middle/back, bottom/middle/top. Centre moves the selection onto the middle of the plate. Drop sets it on the plate or on the solid underneath. Lay flat waits for a click on a face, turns that face down, and drops. Gravity, while on, drops a body at the end of a move. Each of these is one undo step.

Still different: the history list names the step, it does not replay the live sentence for every older tool. Lay flat uses the clicked triangle of this port's mesh. Align does not yet resize bodies or follow a tilted workplane. Paste does not also write the system clipboard.

Shots: `docs/shots/parity/rust-g-objectlist.png`, `rust-g-history.png`, `rust-g-align.png`.

Round F finishes the object inspector and lines the plate up with the live editor.

The Solid and Hole tiles are the live 64 px swatches. Hole is the diagonal hatch. The active tile has the orange rim. Sliders are an orange fill on a light track with a white thumb. Number boxes show the unit inside (`mm`, `in`, or `°`). Section cards collapse. The panel scrolls inside the window at desktop and iPad sizes. Header icons are the larger help, lock, and eye marks. The bed is the live 200 mm default, so Home is the live camera `(118, 96, 118)` at 38°. The axis triad sits on the front-left corner (X right, Y toward the back, Z up). Green ruler labels use the same type size as the red ones. Modify, Arrange, and shape-library items that are not built are grey and do not run.

Still different: most Modify and Arrange tools are not implemented, the shape library still cannot build the live extras, surface texture is only the disabled row, and the numbers are screen text rather than textures on the bed.

Shots: `docs/shots/parity/rust-f-inspector.png`, `rust-f-inspector-bottom.png`, `rust-f-editor.png`, `rust-f-ipad.png` (1180×820).

Round E matches the live edge rulers and the home camera.

The rulers are the live printed scale. In millimetres the ticks are every 1 mm, taller at 5 mm, and tallest at 10 mm. The front edge is numbered in red (`#e5484d`) and the left edge in green (`#3fae5a`). The tick lines are the live dark grey. Inches use eighth-inch ticks and whole-inch numbers. A fresh load and Home use the live view: from `(118, 96, 118)` toward the origin, 38° vertical field. The live default bed is 200 mm and this bed is 256 mm, so that offset is multiplied by 256/200 and the plate fills the window the same way.

Still different after this round: the axis arrows stay in the middle of the plate, the bed is 256 mm rather than 200 mm, and the numbers are screen text lying on the plate rather than textures painted on it.

Shot: `docs/shots/parity/rust-e-editor.png` (on load, 1440×900).

Round D replaces the right-hand number column with a floating inspector that follows the live panel.

| Item | Status |
| --- | --- |
| Inspector place | A card floats on the right of the view, over the plate, instead of a side column. |
| Name | The body name is an editable field at the top. |
| Solid / Hole | Two chips. Solid shows the body colour. Hole marks the body as a cutter. |
| Transparent | A checkbox. The body is drawn see-through and is not a cutter. |
| Lock / Hide | Lock keeps the body from being selected. Hide takes it out of the view. |
| Surface texture and texture mask | Shown, and disabled. They are not built. |
| Properties | Keep proportions, then Length, Width, and Height with a number and a slider. A cylinder uses Diameter and Height. Units follow mm/in. |
| Save as default / Reset defaults | Save remembers that size for the next body of the same shape. Reset restores the library size. |
| Position and Rotation | X Y Z fields, in millimetres or inches, and degrees. |
| Rulers and camera | Unchanged this round. The red and green numbers are still not the live printed scale. |

Shots: `docs/shots/parity/live-d-inspector.png`, `rust-d-inspector.png` (an ellipse, same panel), `rust-d-editor.png`, `rust-d-ipad.png` (Chromium iPad emulation).

Round C makes the build plate a solid grey bed and opens the full basic shape library.

| Item | Status |
| --- | --- |
| Plate fill | The bed is flat grey, about rgb(180, 185, 190), the same family as the live plate. It is no longer a white field with only dark lines. |
| Grid | Fine lines are lighter than the bed. Major lines and the border are darker. The live grid is still a little finer and softer. |
| Camera | At 1440×900 the plate covers roughly the same screen area as the live editor (from just under the ribbon to the bottom edge). The corner the camera looks from is still a little different. |
| Rulers | Red and green 10 mm numbers sit on the plate edges. The live ruler is a printed scale with smaller ticks. |
| Shape library | Shapes opens a four-column BASIC SHAPES grid in the live order, using the live grey pictures. A shape this port cannot build is still shown. Clicking it says it is not in this build yet. |
| Inspector | Still the right-hand number and colour panel. It does not yet match the live inspector. |

Shapes shown but not built: Loft, Vase, Bent tube, Counterbore, Countersink, Teardrop hole, Icon tag, QR code, Thread, Screw, Nut, Spring, Gridfinity, Gear (no separate picture in the icon set), Knurl, Hinge, Dovetail, Crescent, Ruler. Sketch is in the grid and starts sketch mode. Gear has no picture of its own.

Shots: `docs/shots/parity/rust-c-editor.png`, `rust-c-shapes.png`, `rust-c-ipad.png` (Chromium iPad emulation). `live-before-editor.png` and `live-before-shapes.png` are the TypeScript app.

Round B (14 Nov 2026) rebuilds the main editor chrome so a 1440×900 window uses the live app's icon ribbon, header, left rail, grey ruled plate, and bottom bar. Shots: `docs/shots/parity/rust-b-editor.png`, `rust-b-shapes.png`, `rust-b-ipad.png` (Chromium iPad emulation, not a physical iPad). `live-before-editor.png` is the TypeScript editor.

## Round B

| Item | Status |
| --- | --- |
| Header: logo, “PrintCaddy”, “Easy 3D CAD for 3D printing”, 3D EDITOR / SKETCH tabs, design name, System, EN | Looks close. The name field is editable. System opens nothing of its own; themes stay in Settings. EN is the label only. |
| Two-row icon ribbon with HOME, CLIPBOARD, SHAPES, COMBINE, ARRANGE, MODIFY, MANAGE, HELP | Looks close. Icons are the live app's toolbar art. Tools that are not built are grey. |
| Left rounded rail and the view cube above it | Looks close. The cube is a light cube labelled with the three nearest faces. Rail buttons that are not built are grey. |
| Grey plate, fine grid, red and green numbered edge ticks, design-name watermark | Looks closer. Ticks run every 10 mm. The watermark is faint and follows the plate. The live edge ruler is still finer and more printed-looking. |
| Bottom bar: User guide, Bug report, Snap Grid, mm/in, source and license | Looks close. The license line says MIT, which is this port's license. The TypeScript app says AGPL-3.0. |
| Shape library | The Shapes button opens a window of the solids this port can build, with the live grey shape pictures. The live library's extra shapes are not listed there yet. |
| Selected-object inspector | Still the right-hand number and colour panel. It is not the live inspector's layout. |

### Ribbon tools that are not built

These are on the ribbon and grey: Home dashboard, History, Copy, Paste, Object list, Hide selected, Bundle, Hull, Loft sketches, Common part, Align, Drop, Lay flat, Align faces, Center, Mirror, Pivot, Pattern, Hole pattern, Face, Split, Flexi joints, Cookie cutter, Chamfer, Hollow, Offset, Delete face, Simplify, Texture, Note, Attachments, To Bambu, AI, Collision, Full screen, Gravity, Search, Guide, Help.

### Rail tools that are not built

Show labels, orthographic view, second view, double-click zoom, hide workplane, place workplane, tape measure, framing square, section view.

Built and clickable: Undo, Redo, Duplicate, Delete, Add shape, Group, Ungroup, Intersect, Cut out, Fillet, Snap to grid, Import, Export, Slice, Layers, Add plate, Settings, Shortcuts, the camera rail (home, fit, zoom, rulers, grid), and the 3D EDITOR / SKETCH tabs.

# Parity round A

Compared on 14 Nov 2026 in desktop Chromium at 1440×900.

- Live TypeScript app: <https://rrkennedy69.github.io/> (opened with Create new 3D design).
- Rust app before this round: <https://rrkennedy69.github.io/printcaddy-rs/> (local preview of the round 9 build).

Screenshots are in `docs/shots/parity/`. `live-*` is the TypeScript app. `rust-before-*` is the Rust app before the parity edits. `rust-after-*` is the Rust app after this round. iPad pictures are Chromium device emulation, not a physical iPad.

The slicer engine (CuraEngine, supports, ironing, gyroid, and the rest) is out of scope. The round 8 contour slicer stays as it is.

## How to read the list

- **Matching** — same job and close to the same look.
- **Looks different** — the action exists, but the chrome, colour, or placement does not match.
- **Works differently** — the control exists, but the result is not the same.
- **Missing** — the live app has it and the Rust app does not.

## Empty editor

| Item | Before | After this round |
| --- | --- | --- |
| Page behind the plate | Looks different. Live is a warm off-white page. Rust was a dark blue viewport. | Light is the default theme. Dark, Graphite, Night, Thermal, X-Ray, Blocks, Forge, and Works stay in Settings. A browser that already saved Dark still opens Dark. |
| Build plate | Looks different. Live is a filled gray 256 mm square with a fine grid and a ruler on the near edge. Rust was white lines on the dark background and no filled bed. | Filled gray bed under the grid lines. The ruler along the edge is still missing. |
| Top ribbon | Looks different. Live groups icon tools under Home, Clipboard, Shapes, Visibility, Combine, Modify, Arrange, Manage, and Help. Rust put every control in a left column. | A top ribbon holds the tools the Rust app actually has: undo, redo, duplicate, delete, shapes, combine, slice, layers, import, export, settings, projects, shortcuts. Icons are words, not the live pictograms. |
| Left rail | Missing. Live has a vertical camera rail (menu, home, zoom, pan, fit). | A short rail jumps Home, Front, Back, Left, Right, Top, and Bottom. Zoom and pan stay on the mouse and the wheel. |
| Right number panel | Looks different. Live opens shape settings on the right when something is selected. Rust stacked colour, shapes, and numbers down the left. | Colour and the numbers sit in a right-hand inspector. The inspector opens when a body is selected, or when Colour is pressed. |
| View cube | Looks different, and works differently. Live’s cube is a line cube at the left of the view; faces, edges, and corners are views. Rust’s cube was a solid cube at the top-right and only the face centres did anything. | The cube sits at the left of the view. A click on a face, an edge, or a corner sets the camera. Dragging the cube still orbits. |
| Snap and units | Looks different. Live shows Snap Grid and mm/in at the bottom right. Rust had a checkbox in the left column and millimetres only. | The bottom bar has the snap switch and an mm/in switch. Inches change the numbers you see and type. The document stays in millimetres, and the snap step stays 1 mm. |
| Project name | Looks different. Live has an editable title in the ribbon. Rust showed “Project Untitled” as a label. | The name is on the ribbon. Renaming is still the Projects window. |

## Shapes

| Item | Before | After this round |
| --- | --- | --- |
| Shape library window | Looks different. Live is a picture grid (basic, holes, text, mechanical). Rust was a wrap of text buttons. | Shapes opens a window of the solids the Rust app can build, including Text and Name tag. Pictures, holes, threads, gears, and the rest of the live library are still missing. |
| Box, cylinder, cone, pyramid, wedge, round roof, half sphere, paraboloid, torus, ellipse, heart | Matching at the library starting sizes. | Unchanged. |
| Rounded box corner radius | Works differently. The radius was fixed at 5 mm. | The inspector edits the radius. |
| Capsule taper | Missing. Both ends were the same. | Taper scales one end from 0.25 to 1. |
| Polygon sides | Missing. Always 6. | Sides from 3 to 24. |
| Sphere sides | Missing. Fixed stacks and slices. | Side count from 8 to 48. |
| Tube wall | Works differently. The wall was fixed at 4 mm. | The inspector edits the wall. |
| Star points | Missing. Always 5. | Points from 3 to 12. |
| Cylinder sides | Works differently. Fixed at 32 and not shown. | The inspector edits the segment count. |
| Text | Missing. | A Latin bitmap extrusion of the inspector text. No fonts, no stacked letters, no right-to-left joining. |
| Name tag | Missing. | A plate, raised letters, and a hole at one end. No coloured layers, no chamfered letters, no key-ring options beyond that hole. |
| Loft, vase, bent tube, counterbore, countersink, teardrop, thread, spring, gridfinity, gear, knurl, hinge, dovetail, crescent, ruler, icon tag, QR | Missing. | Still missing. |

## Selection chrome

| Item | Before | After this round |
| --- | --- | --- |
| Move arrows and rotation arcs | Matching in behaviour (red X, green Y, blue Z, snap, one undo step, baked rotation). | Unchanged. On the light plate the dimension text sits on a light chip so it stays readable. |
| Dimension lines | Matching in what they measure. | The readout follows mm or inches. |
| Resize handles | Missing. | Corner handles drag the footprint. Vertical pointer movement changes the height. The opposite corner stays put. |
| Protractor | Missing. | While a rotation arc is dragged, a degree wheel is drawn around the pivot. There is no separate on-arc number field. |
| Name labels | Missing. | The selected body’s name is drawn above it. |
| Colour | Matching for the default `#8A98A6` and the Bambu swatches. | The swatches moved into the right inspector. |
| Multi-select, group, hole, fillet, sketch | Matching for the tools that exist. | Hidden while Simple mode is on. Simple mode is a checkbox on the ribbon, not the live app’s full simple layout. |

## Plates, files, and shortcuts

| Item | Before | After this round |
| --- | --- | --- |
| Extra plates | Matching for the three-per-row layout, add, and delete. | Unchanged. |
| Floor tile for delete and lock | Missing. The buttons were in the left column. | Each plate has a small tile on the bed with its number, Lock, and Delete. Plate 1 cannot be deleted. |
| Keyboard camera | Missing. | Keys 1–6 jump Front, Back, Left, Right, Top, Bottom, unless a text field has focus. The Shortcuts window can rebind those six keys for this visit. |
| Undo, redo, delete keys | Matching. Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z, Delete, Backspace. | Unchanged. The shortcut window lists them. It does not rebind them. |
| Languages | Missing. Live has a long language menu. | The ribbon menu switches English, German, French, and Spanish for the toolbar words only. |
| `.lyl` | Missing. Live opens a zipped `project.json` design. | Still missing. Rust projects stay the JSON saved from the Projects window. |
| STL, 3MF, OBJ, STEP, SVG, DXF, Bambu 3MF | Matching for the round 6 and round 7 subset. | Unchanged. |
| Slicer page and layer cutaway | Matching for the round 8 contour slicer. | Unchanged. Not the live CuraEngine slicer. |
| Check for updates, projects in the browser, themes, Google Drive sign-in | Matching for the round 9 subset. | Unchanged. No new OAuth client. |
| AI helper, Send to Bambu Studio | Not in this rewrite, on purpose. | Still absent. |

## Still different after this round

- The ribbon is words rather than the live icon set, and most Modify / Arrange / Help tools are not implemented.
- No edge ruler, no picture shape library, no full text system, no `.lyl` reader.
- Languages cover four toolbar vocabularies, not the live manual’s language list.
- The shortcut editor rebinds the six camera keys only.
- Resize is corner dragging, not the live handle set, and the protractor has no typed angle.
- Simple mode hides the advanced ribbon groups. It does not restyle the whole app.
- The slicer is still the round 8 contour slicer.
