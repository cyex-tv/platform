// The paint studio: an editor for cyex.tv name paints (gradients and animated pictures).
// Built on the owner's cyex paints tool: its animated engine lives in ./engine; the gradient
// model, checks, CSS, import and export are cyex.tv's own (./format, ./css, ./interop).
export { default as PaintStudio } from './PaintStudio.svelte';
export { apiBackend, fromApi, toInput, pictureUrl, type SavedPaint, type StudioBackend } from './backend';
export { LIMITS, blankPaint, validatePaint, type StudioPaint } from './format';
export { default as AdminStudio } from './AdminStudio.svelte';
