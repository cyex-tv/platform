# 0017: The paint studio (staff editor for name paints)

- Status: accepted
- Date: 2026/11/17

## Context
Staff make name paints (ADR 0015) through `createPaint` / `updatePaint` and paint pictures. The owner already has an editor for paints, the cyex paints tool (`cyex-app/cyex-paint-tool`, the owner's own code): gradient paints, and animated paints built from layers, scene elements and effects, exported as GIF, APNG or WebP. He asked for it to be the baseline of cyex.tv's editor.

The tool works in the integer-colour paint format other paint tools use, and two of its small functions (the gradient CSS builder and the integer-to-`rgba()` formatter) were close ports of the 7TV browser extension (Apache-2.0 with the Commons Clause), which AGENTS.md rules out.

## Decision
- The studio is a self-contained module, `apps/web/src/lib/paint-studio/`, used by the admin paint pages (`/admin/cosmetics/paints/studio` for a new paint, `/admin/cosmetics/paints/<id>/studio` to edit one, both needing `cosmetics.manage`) and a development page (`/dev/paint-studio`, a 404 in production builds).
- **Engine:** the tool's animated engine (layers, effects, scene elements, keyframes, the GIF, APNG and WebP encoders) is brought in as plain JS under `paint-studio/engine/`, with the owner's permission, unchanged except for: ES module wrappers instead of a global, the 7TV-derived colour formatter removed, and the "custom code" items (people's own JavaScript, run with `new Function`) removed, because staff pages must not run code from files and the site's CSP has no `unsafe-eval`. Types for what the studio uses are in `engine/index.d.ts`.
- **Gradient paints, checks, CSS and import/export are cyex.tv's own TypeScript** (`format.ts`, `css.ts`, `interop.ts`), written from the API's description of a paint, not from the tool's 7TV-derived code. Colours are `#RRGGBB(AA)`; the API's limits (ADR 0015 and `crates/media`: 2 to 10 stops, 4 shadows within ±16 / 0 to 16 px, names 1 to 32 characters, pictures at least 64 px tall, at most 1.000 px a side and 5 MB) are checked as people edit, with a message per field.
- **Interop:** paint JSON with integer colours (the public single-gradient shape) and the tool's project files import into the cyex.tv model, with a list of what had to change (extra stops, shadows out of range, layered paints reduced to their first layer); export writes cyex.tv JSON, integer-colour JSON and CSS.
- **Animated paints** render and encode in a module Web Worker (a page fallback renders in small steps). Saving renders the chosen format, uploads it as a paint picture, waits for the picture to be ready, then saves an `IMAGE` paint. cyex.tv keeps only the picture; the layers behind a saved paint stay in the browser that made it (local storage, by paint id) so it can be edited again there, and can be downloaded as a project file.
- Previews of animated paints pause under reduced motion, with a play button.

## Consequences
- The engine (about 6.000 lines of the owner's JS) lives in the AGPL repo; it is loaded only when the animated editor opens.
- Editing an animated paint on another browser starts from its picture, unless the project file is imported.
- The studio draws previews through the site's paint renderer (`lib/cosmetics/paint.ts`), so a paint looks the same in the studio as on profiles and in admin lists; only local pictures (a fresh render or a picked file) bypass it, with the same declarations.
