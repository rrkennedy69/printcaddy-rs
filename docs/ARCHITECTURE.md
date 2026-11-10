# Architecture

PrintCaddy is being rewritten as a Rust workspace that compiles to WebAssembly and is served as a static site. The document model does not know about the browser. The interface and the viewport are also Rust, so a drag, a pinch, and a mesh upload stay in one program instead of crossing into JavaScript on every move.

## Choice

**egui and eframe, drawn with glow (WebGL2), packed by Trunk.**

The core crate (`printcaddy-core`) holds the document, colours, primitive meshes, ray picking, and STL import and export. It is ordinary Rust and is tested with `cargo test` on the host. Later rounds add undo, the sketch solver, booleans, and the slicer here, still without a graphics dependency.

The app crate (`printcaddy-app`) is an eframe program. A side panel is egui. The viewport is a glow paint callback: a small WebGL2 Lambert shader for bodies and a line shader for the plate grid. Trunk emits the HTML, JS, and wasm that GitHub Actions deploys.

### Why this and not the other Rust UI options

The page has to run in desktop Chromium and in iPad Safari, with fingers as a first-class input, and it has to stay fast once meshes get large.

**WebGL2 rather than WebGPU.** iPad Safari has had WebGL2 for years. WebGPU there is newer and still uneven, and wgpu's own WebGL fallback has a history of Safari-only failures (depth textures, shader translation, missing extensions). glow is a thin binding over the WebGL2 context the browser already created. That is the same graphics floor the current three.js app relies on. The viewport code lives in `renderer.rs` only. A wgpu backend can replace that file later, when Safari's WebGPU is boring, without moving the document or the panels.

**egui rather than Leptos.** A CAD view is a continuous input and paint loop. Leptos owns the DOM, so the 3D view would still be a custom canvas and the tools would be HTML. That is two input paths to keep in agreement, which is where the current app's tablet bugs live (a pinch must not also nudge a part). egui has one pointer stream for the buttons and the viewport. Touch phases are explicit, so one finger can orbit or move a body and two fingers can pan and pinch, and a second finger can put a one-finger move back.

**Not Bevy.** Bevy is a wgpu game engine. It is a large wasm download, a slower startup, and a poor fit for dense tool panels. The slicer and booleans want a document crate, not an entity scene.

**Not a Rust core behind the existing TypeScript UI.** The point of the rewrite is that the page itself is the Rust program. Keeping three.js would freeze the Safari input problems in place.

Trunk is the packager because the output is a static directory. GitHub Pages can host it with no server. The build does not need cross-origin isolation or `SharedArrayBuffer`, which Pages will not send headers for.

## How a frame works

1. egui lays out the tool panel and the viewport rectangle.
2. Pointer and touch events update the camera or call `Document::move_object_to` / `hit_test`.
3. The app builds a draw list: view-projection matrix, per-body colour and translation, and an `Arc` of the mesh bytes (uploaded to the GPU once).
4. During paint, the glow callback scissor-clears the viewport, draws the grid, then draws each body.

New bodies use `#8A98A6`. Local meshes sit with their bottom on Y = 0 and their footprint centred on X and Z. The plate is 256 mm on a side, centred on the origin, with lines every 10 mm and a stronger line every 50 mm. Y is up.

Mouse: drag empty space to orbit, drag a body to move it on the plate, right or middle button to pan, wheel to zoom. Touch: one finger does the orbit or the move, two fingers pan and pinch. The page sets `touch-action: none` and cancels Safari gesture events so a pinch zooms the camera and not the page.

STL bytes are parsed and written in the core crate. The browser build uses a file input and a download link. Nothing is uploaded anywhere.

## Milestone rounds

Checked work is in the app today. Everything else is still only in the TypeScript app. See `docs/FEATURES.md` for the full inventory.

1. **Viewport.** Orbit, pan, zoom, plate grid, box, cylinder, select, move, STL in and out, default colour. Done.
2. **Arrange.** Undo and redo, delete, duplicate, typed position, size and rotation, 1 mm snap, and the simple solids at the current app's starting sizes. Done. Multi-select, text, and name tags are still open.
3. **Handles.** Move arrows, rotate arcs that bake back to 0/0/0, dimension lines, the view cube, and multi-select. Done. Resize handles and the protractor wheel stay later.
4. **Sketches.** A plate polyline can be extruded, revolved, or swept. The solid is built in the core crate, and a baked rotation keeps the profile. Arcs, curves, and constraints are still later.
5. **Booleans.** Join, subtract, and intersect on closed meshes, groups, see-through holes that cut when grouped, and a typed fillet on a box. Done for those cases. Bundle, hull, split, chamfer, shell, and a fillet on one clicked edge stay later.
6. **Exchange.** 3MF and OBJ with colour, SVG and DXF outlines as sketches, and triangulated STEP. Done for that subset. Analytic STEP, Bambu 3MF, and top-view SVG/DXF export stay later.
7. **Bambu.** Extra plates in rows of three, the filament catalogue and loaded-filament import, Bambu 3MF export, Send to Bambu Studio.
8. **Slice.** The slicer page (prepare and preview), materials, and the layer-view cutaway.
9. **Chrome.** Themes including X-Ray, Settings with Mobile mode, Check for updates, and browser projects. Done for those. Google Drive uses the existing client and waits on that client's allowed origin. Languages stay later. The AI helper is not part of this rewrite.

Each round should keep the wasm build on GitHub Pages and the core tests green. The TypeScript site is not modified and stays the production app until the boxes in `docs/FEATURES.md` are checked.
