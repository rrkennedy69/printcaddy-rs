# Round 8 — slicer page and layer view

Approved scope. Send to Bambu Studio stays out. The AI helper stays out. Rounds 1–7 and 9 stay as they are. New bodies are still `#8A98A6`.

The TypeScript app (`SlicerPanel.tsx`, `slicer.ts`, `WorkplaneViewport.tsx`) is the reference.

## What matches

- A **Slice** button and a **Layers** button on the viewport toolbar, in the same roles as the TypeScript work-plane toolbar.
- A slicer page with Prepare and Preview, Back, and a settings column.
- Presets **0.28 mm Extra Draft**, **0.20 mm Standard**, and **0.12 mm Fine**, with the same layer height, wall count, and infill.
- Process groups: quality, strength, speed, support, and others. The fields are layer height, initial layer height, seam, ironing, wall loops, top and bottom shells, infill density and pattern, speeds, support on/off, support type, threshold angle, build-plate-only, and skirt / brim / none.
- One filament slot taken from the painted body (or PLA, 220 °C / 55 °C, if the body has no filament). Nozzle 0.4 mm. Bed 256 × 256 × 256 mm.
- Slice refuses an empty plate and a body that does not fit the bed.
- Preview draws the toolpaths in the same colours the TypeScript app uses (outer wall, inner wall, skin, infill, skirt). A layer slider shows layers up to the chosen one. The readout is layer number and height in millimetres.
- After a slice: print time, filament length, and weight, from the paths that were actually generated. Export 3MF uses the existing Bambu project writer. Download G-code saves the paths. Send to Bambu Studio is not on the page.
- **Layers** opens a vertical slider. It hides everything above the chosen height. The model is not edited. The readout is millimetres and an approximate layer number at 0.20 mm, the same `round(height / 0.2)` the TypeScript slider uses. Closing it restores the full bodies.

## What does not match

The TypeScript Slice button fetches CuraEngine (`cura-wasm`, about 4 MB, AGPL) and runs it in a worker. This build does not ship that engine.

The engine here slices each body with horizontal planes and writes:

- wall loops, inset from the layer outline by the nozzle width
- grid, line, or concentric infill; gyroid, cubic, triangles, zigzag, tri-hexagon, quarter cubic, and lightning are stored and sliced as grid
- a skirt or a brim on the first layer
- solid skins for the top and bottom shell counts

It does not build support towers, tree supports, ironing passes, or Cura’s seam. Those controls stay on the page and are named in the slice note. Print time is path length divided by the typed speeds, not Cura’s planner. Importing a Bambu settings file into the slicer applies the keys this page already edits (layer height, walls, infill, temperatures, support on/off). The other keys are not copied into a Cura definition.
