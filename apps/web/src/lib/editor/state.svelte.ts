// The emote editor's state: the document and its history, the decoded pictures, the tool and its
// settings, playback, and the encoded result. One per upload dialog; it lives as long as the
// dialog does, so closing the dialog and opening it again finds the work where it was.
import { History } from './history';
import { allPass, checkRules, editorLimits, EDITOR_LIMITS, type EditorLimits, type FixId } from './rules';
import { canvasSize, cropNoTaller, cropNoWider, createDoc, dropEveryFrames, exportDoc, exportFrames, fitTo, frozen, limitFpsFrames, outSize, scaleBy, speedOf, type Doc } from './model';
import { shortest, total } from './timing';
import { Cancelled, canEncodeWebp, encodeDoc, importFile, type ImportProgress } from './media';
import { budgetFor } from './sniff';

export type Tool = 'select' | 'crop' | 'size' | 'pen' | 'line' | 'arrow' | 'rect' | 'ellipse' | 'text' | 'pan';

export const TOOLS: { id: Tool; label: string; key: string; icon: string }[] = [
  { id: 'select', label: 'Select and move', key: 'V', icon: 'pointer' },
  { id: 'crop', label: 'Crop', key: 'C', icon: 'crop' },
  { id: 'size', label: 'Resize and stretch', key: 'S', icon: 'stretch' },
  { id: 'pen', label: 'Pen', key: 'P', icon: 'pen' },
  { id: 'line', label: 'Line', key: 'L', icon: 'line' },
  { id: 'arrow', label: 'Arrow', key: 'A', icon: 'arrow' },
  { id: 'rect', label: 'Rectangle', key: 'R', icon: 'square' },
  { id: 'ellipse', label: 'Ellipse', key: 'O', icon: 'circle' },
  { id: 'text', label: 'Text', key: 'T', icon: 'type' },
  { id: 'pan', label: 'Pan (or hold Space)', key: 'H', icon: 'hand' },
];

export const DRAW_TOOLS: Tool[] = ['pen', 'line', 'arrow', 'rect', 'ellipse', 'text'];

export type Brush = { color: string; fill: string; width: number; filled: boolean; outlined: boolean; size: number; scope: 'all' | 'frame' };

/** Above this many pixels in all frames the size is checked on request, not after every edit. */
export const AUTO_ENCODE_PIXELS = 30_000_000;

/** `still` is the frame a frozen animation (speed 0) uploads, else -1. */
type Encoded = { doc: Doc; still: number; format: 'webp' | 'gif'; quality: number; blob: Blob };

export class EditorState {
  doc = $state.raw<Doc | null>(null);
  /** Bumped on every history change, so labels and buttons follow. */
  steps = $state(0);
  history = new History<Doc | null>(null);
  /** The decoded pictures, by source index (not reactive: they never change). */
  sources: ImageBitmap[] = [];
  name = $state('');
  /** "Scaled to … on import" and the like. */
  note = $state<string | null>(null);
  importing = $state.raw<ImportProgress | null>(null);
  importError = $state<string | null>(null);
  /** A file waiting for "Replace your edits?". */
  pending = $state.raw<File | null>(null);

  tool = $state<Tool>('select');
  current = $state(0);
  /** Frame indices picked in the timeline (always includes `current` when not empty). */
  picked = $state.raw<ReadonlySet<number>>(new Set());
  shape = $state<number | null>(null);
  ratio = $state('free');
  /** Width and height change together. Off by default: an emote can be stretched to any shape. */
  keepAspect = $state(false);
  playing = $state(false);
  onion = $state(false);
  brush = $state<Brush>({ color: '#ff4d4d', fill: '#ffffff', width: 6, filled: false, outlined: true, size: 48, scope: 'all' });

  format = $state<'webp' | 'gif'>('webp');
  quality = $state(1);
  webp = $state(true);
  encoded = $state.raw<Encoded | null>(null);
  encoding = $state.raw<{ done: number; total: number } | null>(null);
  encodeError = $state<string | null>(null);
  limits = $state.raw<EditorLimits>(EDITOR_LIMITS);

  #importer: AbortController | null = null;
  #encoder: AbortController | null = null;

  constructor() {
    void canEncodeWebp().then((ok) => {
      this.webp = ok;
      if (!ok) this.format = 'gif';
    });
  }

  get loaded() {
    return !!this.doc;
  }
  /** Something would be lost by starting over. */
  get dirty() {
    void this.steps;
    return this.history.depth > 0;
  }
  get canUndo() {
    void this.steps;
    return this.history.canUndo;
  }
  get canRedo() {
    void this.steps;
    return this.history.canRedo;
  }
  get undoLabel() {
    void this.steps;
    return this.history.undoLabel;
  }
  get redoLabel() {
    void this.steps;
    return this.history.redoLabel;
  }

  setLimits(emote: Parameters<typeof editorLimits>[0], caps: Parameters<typeof editorLimits>[1]) {
    this.limits = editorLimits(emote, caps);
  }

  /** A new step. */
  commit(next: Doc, label: string) {
    if (!this.doc || next === this.history.now) {
      this.doc = next;
      return;
    }
    this.history.push(next, label);
    this.doc = next;
    this.steps++;
    this.#clampSelection();
  }

  /** Shows `next` without a step (a drag in progress); `commit` lands it. */
  preview(next: Doc) {
    this.doc = next;
  }

  /** Back to the last step (a cancelled drag). */
  revert() {
    this.doc = this.history.now;
  }

  undo() {
    if (!this.history.canUndo) return;
    this.doc = this.history.undo();
    this.steps++;
    this.#clampSelection();
  }

  redo() {
    if (!this.history.canRedo) return;
    this.doc = this.history.redo();
    this.steps++;
    this.#clampSelection();
  }

  #clampSelection() {
    const n = this.doc?.frames.length ?? 0;
    if (this.current >= n) this.current = Math.max(0, n - 1);
    const kept = [...this.picked].filter((i) => i < n);
    if (kept.length !== this.picked.size) this.picked = new Set(kept);
    if (this.shape !== null && !this.doc?.shapes.some((s) => s.id === this.shape)) this.shape = null;
  }

  /** Frame indices an action applies to: the picked ones, else the current one. */
  get targets(): number[] {
    return this.picked.size ? [...this.picked].sort((a, b) => a - b) : [this.current];
  }

  // ---- Importing ----

  /** Opens `file`; with edits in the way it asks first (`pending`). */
  open(file: File, force = false) {
    if (this.dirty && !force) {
      this.pending = file;
      return;
    }
    this.pending = null;
    void this.#load(file);
  }

  async #load(file: File) {
    this.cancelImport();
    this.cancelEncode();
    const ctl = new AbortController();
    this.#importer = ctl;
    this.importError = null;
    this.importing = { done: 0, total: 0, kind: null };
    const mem = (navigator as Navigator & { deviceMemory?: number }).deviceMemory;
    try {
      const got = await importFile(file, budgetFor(mem), (p) => {
        if (this.#importer === ctl) this.importing = p;
      }, ctl.signal);
      if (this.#importer !== ctl) {
        for (const b of got.bitmaps) b.close();
        return;
      }
      this.#free();
      this.sources = got.bitmaps;
      const d = createDoc(got.scaledWidth, got.scaledHeight, got.delays);
      this.history.reset(d);
      this.doc = d;
      this.steps++;
      this.name = file.name;
      this.current = 0;
      this.picked = new Set();
      this.shape = null;
      this.playing = false;
      this.encoded = null;
      this.encodeError = null;
      const notes: string[] = [];
      if (got.scaledWidth !== got.width) notes.push(`Opened at ${got.scaledWidth} × ${got.scaledHeight} px (it’s ${got.width} × ${got.height}) so this tab doesn’t run out of memory.`);
      if (got.bitmaps.length < got.frames) notes.push(`Only the first ${got.bitmaps.length} of its ${got.frames} frames were read.`);
      this.note = notes.join(' ') || null;
    } catch (e) {
      if (e instanceof Cancelled || this.#importer !== ctl) return;
      this.importError = e instanceof Error && e.message ? e.message : 'The editor couldn’t open that file.';
    } finally {
      if (this.#importer === ctl) {
        this.#importer = null;
        this.importing = null;
      }
    }
  }

  cancelImport() {
    this.#importer?.abort();
    this.#importer = null;
    this.importing = null;
  }

  #free() {
    for (const b of this.sources) b.close();
    this.sources = [];
  }

  /** Starts over: nothing loaded. */
  discard() {
    this.cancelImport();
    this.cancelEncode();
    this.#free();
    this.history.reset(null);
    this.doc = null;
    this.steps++;
    this.name = '';
    this.note = null;
    this.importError = null;
    this.pending = null;
    this.encoded = null;
    this.playing = false;
    this.picked = new Set();
    this.shape = null;
  }

  // ---- Rules and export ----

  get output() {
    const d = this.doc;
    if (!d) return null;
    const o = outSize(d);
    const frames = exportFrames(d, this.current, this.limits.minDelayMs);
    return { width: o.w, height: o.h, frames: frames.length, shortest: shortest(frames), duration: total(frames), size: this.result ? this.result.size : null };
  }

  get rules() {
    const o = this.output;
    return o ? checkRules(o, this.limits) : [];
  }

  get ready() {
    return allPass(this.rules);
  }

  /** The encoded file for the current document, when it is encoded. */
  get result(): Blob | null {
    const e = this.encoded;
    return e && e.doc === this.doc && e.still === this.still && e.format === this.format && e.quality === this.quality ? e.blob : null;
  }

  /** The frame a frozen animation uploads as its still; -1 when it isn't frozen. */
  get still(): number {
    return this.doc && frozen(this.doc) ? this.current : -1;
  }

  /** The playback speed, 1 as timed; 0 frozen. */
  get speed(): number {
    return this.doc ? speedOf(this.doc) : 1;
  }

  get pixels() {
    const o = this.output;
    return o ? o.width * o.height * o.frames : 0;
  }

  cancelEncode() {
    this.#encoder?.abort();
    this.#encoder = null;
    this.encoding = null;
  }

  /** Encodes the current document (again only when it changed). */
  async encode(): Promise<Blob | null> {
    const d = this.doc;
    if (!d) return null;
    if (this.result) return this.result;
    this.cancelEncode();
    const ctl = new AbortController();
    this.#encoder = ctl;
    const format = this.format;
    const quality = this.quality;
    const still = this.still;
    const out = exportDoc(d, this.current, this.limits.minDelayMs);
    this.encodeError = null;
    this.encoding = { done: 0, total: out.frames.length };
    try {
      const blob = await encodeDoc(out, this.sources, { format, quality }, (done, n) => {
        if (this.#encoder === ctl) this.encoding = { done, total: n };
      }, ctl.signal);
      if (this.#encoder !== ctl) return null;
      this.encoded = { doc: d, still, format, quality, blob };
      return blob;
    } catch (e) {
      if (e instanceof Cancelled || this.#encoder !== ctl) return null;
      if (e instanceof Error && e.message === 'no-webp') {
        this.webp = false;
        this.format = 'gif';
        this.encodeError = 'This browser can’t make WebP files, so it makes a GIF instead.';
      } else this.encodeError = 'Couldn’t encode the emote. Try again, or make it smaller.';
      return null;
    } finally {
      if (this.#encoder === ctl) {
        this.#encoder = null;
        this.encoding = null;
      }
    }
  }

  /** The finished file, named after the original. */
  async finish(): Promise<File | null> {
    const blob = await this.encode();
    if (!blob || !this.ready) return null;
    const base = this.name.replace(/\.[^.]+$/, '') || 'emote';
    return new File([blob], `${base}.${blob.type === 'image/gif' ? 'gif' : 'webp'}`, { type: blob.type });
  }

  /** A rule's one-click fix. */
  fix(id: FixId) {
    const d = this.doc;
    if (!d) return;
    const l = this.limits;
    switch (id) {
      case 'fit':
        return this.commit(fitTo(d, l.maxSide), `Fit to ${l.maxSide} px`);
      case 'drop':
        return this.commit(dropEveryFrames(d, 2), 'Drop every 2nd frame');
      case 'fps':
        return this.commit(limitFpsFrames(d, l.minDelayMs), `Limit to ${Math.round(1000 / l.minDelayMs)} fps`);
      case 'aspect': {
        const next = cropNoWider(d, l.maxAspect);
        // A stretched output can be too wide with a crop that isn't: then the output narrows.
        const o = outSize(next);
        if (o.w / o.h > l.maxAspect) return this.commit({ ...next, sx: (o.h * l.maxAspect) / next.crop.w }, `Crop to ${l.maxAspect}:1`);
        return this.commit(next, `Crop to ${l.maxAspect}:1`);
      }
      case 'tall': {
        const next = cropNoTaller(d, l.maxTall);
        // A stretched output can be too tall with a crop that isn't: then the output gets shorter.
        const o = outSize(next);
        if (o.h / o.w > l.maxTall) return this.commit({ ...next, sy: (o.w * l.maxTall) / next.crop.h }, `Crop to 1:${l.maxTall}`);
        return this.commit(next, `Crop to 1:${l.maxTall}`);
      }
      case 'shrink':
        return this.commit(scaleBy(d, 0.75), 'Scale to 75%');
    }
  }

  get canvas() {
    return this.doc ? canvasSize(this.doc) : { w: 1, h: 1 };
  }

  // ---- Playback ----

  #timer: ReturnType<typeof setTimeout> | undefined;

  /** Whether playback can run: an animation that isn't frozen. */
  get canPlay() {
    const d = this.doc;
    return !!d && d.frames.length > 1 && !frozen(d);
  }

  play() {
    if (!this.canPlay) return;
    this.playing = true;
    this.#tick();
  }

  /** The upload's frames as (frame index, delay), cached per document: playback shows them. */
  #plan: { doc: Doc; min: number; steps: { i: number; delay: number }[] } | null = null;
  #steps(d: Doc) {
    const min = this.limits.minDelayMs;
    if (this.#plan?.doc !== d || this.#plan.min !== min) {
      const index = new Map(d.frames.map((f, i) => [f.id, i]));
      this.#plan = { doc: d, min, steps: exportFrames(d, this.current, min).map((f) => ({ i: index.get(f.id) ?? 0, delay: f.delay })) };
    }
    return this.#plan.steps;
  }

  #tick() {
    clearTimeout(this.#timer);
    const d = this.doc;
    if (!this.playing || !d || !this.canPlay) {
      this.playing = false;
      return;
    }
    // At the playback speed: frames merged at high speeds are skipped, as in the upload.
    const steps = this.#steps(d);
    let k = 0;
    for (let j = 0; j < steps.length && steps[j].i <= this.current; j++) k = j;
    const delay = steps[k]?.delay || 100;
    this.#timer = setTimeout(() => {
      const now = this.doc;
      if (!now || now !== d) return this.#tick();
      const next = steps[(k + 1) % steps.length];
      this.current = next ? next.i : 0;
      this.#tick();
    }, delay);
  }

  pause() {
    clearTimeout(this.#timer);
    this.playing = false;
  }

  toggle() {
    if (this.playing) this.pause();
    else this.play();
  }

  /** Shows frame `i` (stops playback). */
  show(i: number) {
    this.pause();
    const n = this.doc?.frames.length ?? 0;
    this.current = Math.max(0, Math.min(n - 1, i));
  }

  /** The text field to focus next (a text drawing was just placed). */
  focusText = $state(0);
}
