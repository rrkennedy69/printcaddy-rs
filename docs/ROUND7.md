# Round 7 — plates, Bambu filaments, Bambu 3MF

Approved scope. The slicer page, layer view, and Send to Bambu Studio stay out. The AI helper stays out. Rounds 1–6 and 9 stay as they are. New bodies are still `#8A98A6`.

## Plates

The TypeScript app (`buildPlateStore.ts`, `BuildPlates.tsx`) keeps plate 1 at the origin and adds plates in slots. Slot `k` sits at

`x = (k % 3) * (width + 64)`, `z = floor(k / 3) * (depth + 64)`.

Three plates per row. The next row is toward the viewer, which in this Y-up scene is `+Z`. The plate is 256 mm, so the pitch is 320 mm. Deleting a plate (never the last one) moves its bodies to the floor just to the right of the remaining grid, then slides the later plates over so the slots are packed again. Locking a plate locks the bodies on it so they cannot be selected. A body dropped onto a locked plate becomes locked.

The X/Y/Z triad stays on plate 1. Each plate gets the same grid.

## Filaments

The catalogue is the TypeScript list: 49 Bambu materials and 318 colours, default material PLA Basic (`GFA00`). The colour picker shows **Currently Loaded** first, then a material menu and that material’s colours. A gradient colour uses its solid hex (`h`); the swatch is not a painted blend.

Settings → Import from Bambu Studio reads a filament preset `.json`, a `.3mf` with `Metadata/project_settings.config`, or a `.bbscfg` / `.bbsflmt` / `.zip` of presets. It keeps colour, type, preset name, and id. It does not apply slicer process settings. That import is the slicer’s job and stays out. The list is kept in this browser only.

## Bambu 3MF

Export builds on the round 6 zip writer. The file is a Bambu Studio project:

- `Application` is `BambuStudio-02.00.02.01`
- one mesh object per body
- `Metadata/project_settings.config` lists `filament_colour`, `filament_ids`, and `filament_type`, one slot per unique material and colour, in first-use order
- `Metadata/model_settings.config` sets each body’s `extruder` to that slot
- on-screen plates stay in rows of three; inside the 3MF, plates use Bambu’s own grid (`ceil(sqrt(n))` columns, stride 1.2 × the bed, rows toward −Y)

A missing filament is PLA Basic (`GFA00`) with the body’s colour. The printer line is the X1 Carbon 0.4 nozzle, the fallback the TypeScript app uses when no Bambu printer is chosen. This build does not embed the ~450-key resolved machine profiles from `public/bambu-profiles`. Bambu Studio is expected to fill those from the named system presets (`inherits_group`). The zip entries are stored, not deflated. Import of this file restores bodies, colours, and filament ids. A plain round 6 3MF still imports as before.

## Limits

Same as round 6: 32 MB, 200 000 triangles. A bad file returns a sentence and does not panic.

## Not in this round

Send to Bambu Studio, the slicer page, layer view, print-time estimates, and the full printer-preset list.
