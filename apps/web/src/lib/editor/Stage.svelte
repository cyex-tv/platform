<script lang="ts">
  // The editor's stage: the current frame on a transparent checkerboard, zoomed and panned, with
  // the crop box and the drawings on top. Mouse, pen and touch all work (one pointer draws, two
  // pinch to zoom); the wheel (and a trackpad pinch, Ctrl + wheel) zooms at the pointer, Shift +
  // wheel pans sideways, Space + drag pans with any tool. The resize tool shows the output as it
  // uploads (cropped and stretched) with handles that stretch it to any shape; Shift keeps the
  // shape. Keyboard: arrows move the picked drawing (or the crop box), Shift by 10 px; with the
  // resize tool they change the width and height; Delete removes the drawing; Esc lets go of it.
  import { onMount, untrack } from 'svelte';
  import EditorIcon from './EditorIcon.svelte';
  import { DRAW_TOOLS, type EditorState } from './state.svelte';
  import { HANDLES, MAX_ZOOM, chain, clamp, dragCrop, fitView, handlePoint, minZoom, segmentDistance, stretchBy, wheelZoom, zoomAt, type Handle, type Rect } from './geometry';
  import { addShape, deleteShape, fitShape, moveShape, outSize, outputTransform, resize, setCrop, shapeBounds, shapesOn, updateShape, type Doc, type Shape } from './model';
  import { ratioText, shapeOk } from './rules';
  import { count } from '$lib/format';
  import { drawShape, measureText, paintFrame } from './render';
  import { RATIOS } from './geometry';

  let { ed }: { ed: EditorState } = $props();

  let box: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let surface: HTMLDivElement;
  let ctx: CanvasRenderingContext2D;
  let vw = $state(1);
  let vh = $state(1);
  let view = $state({ zoom: 1, x: 0, y: 0 });
  /** Follows the stage size until you zoom or pan yourself. */
  let auto = $state(true);
  let space = $state(false);
  let draft = $state.raw<Shape | null>(null);
  let panning = $state(false);

  const doc = $derived(ed.doc);
  /** The resize tool: the stage shows the output, not the canvas. */
  const sizing = $derived(ed.tool === 'size');
  /** What the stage shows: the output with the resize tool, else the whole canvas. */
  const content = () => (sizing && ed.doc ? outSize(ed.doc) : ed.canvas);
  const ratio = $derived(RATIOS.find((r) => r.id === ed.ratio)?.ratio ?? null);
  const cursor = $derived(
    panning ? 'grabbing' : space || ed.tool === 'pan' || sizing ? 'grab' : ed.tool === 'select' ? 'default' : ed.tool === 'text' ? 'text' : 'crosshair',
  );

  const fitted = () => {
    const c = content();
    return fitView(c.w, c.h, vw, vh, vw < 480 ? 12 : 32, 8);
  };
  /** The zoom range: down to a quarter of the fitted zoom, up to 3200%. */
  const lowest = () => minZoom(fitted().zoom);

  export function fit() {
    view = fitted();
    auto = true;
  }

  export function zoomBy(k: number) {
    auto = false;
    const z = clamp(view.zoom * k, lowest(), MAX_ZOOM);
    view = zoomAt(view, z, vw / 2, vh / 2);
  }

  export function actual() {
    auto = false;
    view = zoomAt(view, 1, vw / 2, vh / 2);
  }

  // Fit again when a new picture comes in or the stage changes size.
  let fittedFor: Doc | null = null;
  $effect(() => {
    void vw;
    void vh;
    const d = ed.doc;
    void ed.canvas;
    void sizing;
    untrack(() => {
      const fresh = !!d && (!fittedFor || fittedFor.sw !== d.sw || fittedFor.sh !== d.sh || ed.history.depth === 0);
      // Not while a handle is dragged: the box follows the pointer, the view stays.
      if (d && (fresh || (auto && !drag))) {
        view = fitted();
        if (fresh) auto = true;
      }
      fittedFor = d;
    });
  });
  // Into or out of the resize tool: the stage shows something else, so it fits again.
  let wasSizing = false;
  $effect(() => {
    const s = sizing;
    untrack(() => {
      if (s !== wasSizing && ed.doc) fit();
      wasSizing = s;
    });
  });

  onMount(() => {
    ctx = canvas.getContext('2d')!;
    const ro = new ResizeObserver(([e]) => {
      vw = Math.max(1, e.contentRect.width);
      vh = Math.max(1, e.contentRect.height);
    });
    ro.observe(box);
    // The wheel needs a listener that may prevent scrolling the dialog.
    surface.addEventListener('wheel', wheel, { passive: false });
    // Theme changes repaint the checkerboard and the crop's scrim.
    const mo = new MutationObserver(() => (themeTick += 1));
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class'] });
    return () => {
      ro.disconnect();
      mo.disconnect();
      surface.removeEventListener('wheel', wheel);
    };
  });

  let themeTick = $state(0);
  let raf = 0;
  $effect(() => {
    // Everything the picture depends on.
    void [ed.doc, ed.current, ed.tool, ed.shape, ed.onion, ed.playing, view.zoom, view.x, view.y, vw, vh, draft, themeTick];
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  });

  function tokens() {
    const s = getComputedStyle(box);
    return {
      a: s.getPropertyValue('--checker-a').trim() || '#fff',
      b: s.getPropertyValue('--checker-b').trim() || '#ddd',
      size: parseFloat(s.getPropertyValue('--checker-size')) || 16,
      scrim: s.getPropertyValue('--scrim').trim() || 'rgb(0 0 0 / .55)',
      accent: s.getPropertyValue('--accent').trim() || '#f5a524',
      bad: s.getPropertyValue('--red').trim() || '#e5484d',
      ink: s.getPropertyValue('--accent-ink').trim() || '#000',
      line: s.getPropertyValue('--text').trim() || '#fff',
    };
  }

  let pattern: { key: string; p: CanvasPattern } | null = null;
  function checker(t: ReturnType<typeof tokens>, dpr: number) {
    const key = `${t.a}${t.b}${t.size}${dpr}`;
    if (pattern?.key === key) return pattern.p;
    const n = Math.round(t.size * dpr);
    const c = document.createElement('canvas');
    c.width = c.height = n * 2;
    const x = c.getContext('2d')!;
    x.fillStyle = t.a;
    x.fillRect(0, 0, n * 2, n * 2);
    x.fillStyle = t.b;
    x.fillRect(n, 0, n, n);
    x.fillRect(0, n, n, n);
    pattern = { key, p: ctx.createPattern(c, 'repeat')! };
    return pattern.p;
  }

  function draw() {
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const W = Math.round(vw * dpr);
    const H = Math.round(vh * dpr);
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, W, H);
    const d = ed.doc;
    if (!d) return;
    const t = tokens();
    const z = view.zoom * dpr;
    const ox = view.x * dpr;
    const oy = view.y * dpr;
    if (sizing) return drawOutput(d, t, dpr, z, ox, oy);
    const c = ed.canvas;
    // The checkerboard behind the picture, in screen pixels so it stays the same size at any zoom.
    ctx.fillStyle = checker(t, dpr);
    ctx.fillRect(Math.round(ox), Math.round(oy), Math.round(c.w * z), Math.round(c.h * z));
    // Zoomed in: crisp pixels, so you see what you crop.
    ctx.imageSmoothingEnabled = view.zoom < 2;
    ctx.imageSmoothingQuality = 'high';
    const vt = [z, 0, 0, z, ox, oy] as const;
    const frame = d.frames[ed.current];
    if (frame) paintFrame(ctx, d, ed.current, ed.sources[frame.src], vt);
    if (ed.onion && !ed.playing && ed.current > 0) {
      const prev = d.frames[ed.current - 1];
      paintFrame(ctx, d, ed.current - 1, ed.sources[prev.src], vt, { alpha: 0.35 });
    }
    if (draft) {
      ctx.save();
      ctx.setTransform(...vt);
      drawShape(ctx, draft);
      ctx.restore();
    }
    // Outside the crop: dimmed (strongly while cropping).
    const cr = d.crop;
    const full = cr.x === 0 && cr.y === 0 && cr.w === c.w && cr.h === c.h;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    if (!full || ed.tool === 'crop') {
      ctx.save();
      ctx.globalAlpha = ed.tool === 'crop' ? 1 : 0.75;
      ctx.fillStyle = t.scrim;
      ctx.beginPath();
      ctx.rect(ox, oy, c.w * z, c.h * z);
      ctx.rect(ox + cr.x * z, oy + cr.y * z, cr.w * z, cr.h * z);
      ctx.fill('evenodd');
      ctx.restore();
    }
    if (ed.tool === 'crop') outline(screenRect(cr, dpr), t, dpr, true, true);
    const sel = ed.shape !== null ? d.shapes.find((s: Shape) => s.id === ed.shape) : null;
    if (sel && ed.tool === 'select') outline(screenRect(pad(shapeBounds(sel), sel), dpr), t, dpr, false, true);
  }

  /** The resize tool: the output as it uploads, stretched, with handles (red when off the rules). */
  function drawOutput(d: Doc, t: ReturnType<typeof tokens>, dpr: number, z: number, ox: number, oy: number) {
    const o = outSize(d);
    const r = { x: Math.round(ox), y: Math.round(oy), w: Math.round(o.w * z), h: Math.round(o.h * z) };
    ctx.fillStyle = checker(t, dpr);
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.save();
    ctx.beginPath();
    ctx.rect(r.x, r.y, r.w, r.h);
    ctx.clip();
    ctx.imageSmoothingEnabled = view.zoom < 2;
    ctx.imageSmoothingQuality = 'high';
    const frame = d.frames[ed.current];
    if (frame) paintFrame(ctx, d, ed.current, ed.sources[frame.src], chain([z, 0, 0, z, ox, oy], outputTransform(d)));
    ctx.restore();
    outline(screenRect({ x: 0, y: 0, w: o.w, h: o.h }, dpr), shapeOk(o.w, o.h, ed.limits) ? t : { ...t, accent: t.bad }, dpr, false, true);
  }

  const pad = (r: Rect, s: Shape): Rect => {
    const p = s.kind === 'text' ? 0 : s.width / 2;
    return { x: r.x - p, y: r.y - p, w: r.w + 2 * p, h: r.h + 2 * p };
  };

  const screenRect = (r: Rect, dpr = 1): Rect => ({ x: (view.x + r.x * view.zoom) * dpr, y: (view.y + r.y * view.zoom) * dpr, w: r.w * view.zoom * dpr, h: r.h * view.zoom * dpr });

  function outline(r: Rect, t: ReturnType<typeof tokens>, dpr: number, thirds: boolean, handles: boolean) {
    ctx.save();
    ctx.lineWidth = dpr;
    if (thirds && r.w > 60 * dpr && r.h > 60 * dpr) {
      ctx.strokeStyle = 'rgb(255 255 255 / .35)';
      ctx.beginPath();
      for (const k of [1, 2]) {
        ctx.moveTo(r.x + (r.w * k) / 3, r.y);
        ctx.lineTo(r.x + (r.w * k) / 3, r.y + r.h);
        ctx.moveTo(r.x, r.y + (r.h * k) / 3);
        ctx.lineTo(r.x + r.w, r.y + (r.h * k) / 3);
      }
      ctx.stroke();
    }
    ctx.strokeStyle = t.accent;
    ctx.lineWidth = 1.5 * dpr;
    ctx.strokeRect(r.x, r.y, r.w, r.h);
    if (handles) {
      const s = 8 * dpr;
      for (const h of HANDLES) {
        const [x, y] = handlePoint(r, h);
        ctx.fillStyle = t.accent;
        ctx.fillRect(x - s / 2, y - s / 2, s, s);
        ctx.strokeStyle = t.ink;
        ctx.lineWidth = dpr;
        ctx.strokeRect(x - s / 2, y - s / 2, s, s);
      }
    }
    ctx.restore();
  }

  // ---- Pointer ----

  type Drag =
    | { kind: 'pan'; sx: number; sy: number; vx: number; vy: number }
    | { kind: 'crop'; handle: Handle; start: Rect; px: number; py: number; fresh: boolean }
    | { kind: 'move'; id: number; start: Shape; px: number; py: number; moved: boolean }
    | { kind: 'scale'; id: number; start: Shape; bounds: Rect; handle: Handle; px: number; py: number }
    | { kind: 'draw'; px: number; py: number; moved: boolean }
    | { kind: 'size'; handle: Exclude<Handle, 'move'>; w: number; h: number; sx: number; sy: number; vx: number; vy: number; auto: boolean };
  let drag: Drag | null = null;
  const pointers = new Map<number, { x: number; y: number }>();
  let pinch: { dist: number; zoom: number; mx: number; my: number; vx: number; vy: number } | null = null;

  function local(e: PointerEvent | WheelEvent) {
    const r = canvas.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }
  const toCanvas = (p: { x: number; y: number }) => ({ x: (p.x - view.x) / view.zoom, y: (p.y - view.y) / view.zoom });

  function handleAt(r: Rect, p: { x: number; y: number }): Handle | null {
    const s = screenRect(r);
    const reach = matchMedia('(pointer: coarse)').matches ? 16 : 9;
    for (const h of HANDLES) {
      const [x, y] = handlePoint(s, h);
      if (Math.abs(p.x - x) <= reach && Math.abs(p.y - y) <= reach) return h;
    }
    if (p.x >= s.x && p.x <= s.x + s.w && p.y >= s.y && p.y <= s.y + s.h) return 'move';
    return null;
  }

  function hit(s: Shape, x: number, y: number): boolean {
    const tol = 6 / view.zoom + s.width / 2;
    if (s.kind === 'pen' || s.kind === 'line' || s.kind === 'arrow') {
      if (s.pts.length <= 2) return Math.hypot(x - s.pts[0], y - s.pts[1]) <= tol;
      for (let i = 0; i + 3 < s.pts.length; i += 2) if (segmentDistance(x, y, s.pts[i], s.pts[i + 1], s.pts[i + 2], s.pts[i + 3]) <= tol) return true;
      return false;
    }
    const b = shapeBounds(s);
    return x >= b.x - tol && x <= b.x + b.w + tol && y >= b.y - tol && y <= b.y + b.h + tol;
  }

  function newShape(kind: Shape['kind'], x: number, y: number): Shape {
    const b = ed.brush;
    const frame = b.scope === 'frame' && doc ? doc.frames[ed.current].id : null;
    const base = { id: -1, kind, color: b.color, fill: b.fill, width: b.width, filled: b.filled, outlined: b.outlined, frame };
    if (kind === 'pen') return { ...base, pts: [x, y] };
    if (kind === 'line' || kind === 'arrow') return { ...base, pts: [x, y, x, y] };
    return { ...base, pts: [x, y, 0, 0] };
  }

  function down(e: PointerEvent) {
    if (!doc || ed.importing) return;
    // Focus for the keys, without the keyboard ring a click shouldn't show.
    surface.focus({ preventScroll: true, focusVisible: false } as FocusOptions);
    surface.setPointerCapture(e.pointerId);
    const p = local(e);
    pointers.set(e.pointerId, p);
    if (pointers.size === 2) {
      // A second finger: pinch and pan instead of whatever the first started.
      cancelDrag();
      const [a, b] = [...pointers.values()];
      pinch = { dist: Math.hypot(a.x - b.x, a.y - b.y) || 1, zoom: view.zoom, mx: (a.x + b.x) / 2, my: (a.y + b.y) / 2, vx: view.x, vy: view.y };
      return;
    }
    if (pointers.size > 2) return;
    ed.pause();
    const c = toCanvas(p);
    if (space || ed.tool === 'pan' || e.button === 1) {
      drag = { kind: 'pan', sx: p.x, sy: p.y, vx: view.x, vy: view.y };
      panning = true;
      return;
    }
    if (e.button !== 0) return;
    if (sizing) {
      const o = outSize(doc);
      const h = handleAt({ x: 0, y: 0, w: o.w, h: o.h }, p);
      if (h && h !== 'move') {
        drag = { kind: 'size', handle: h, w: o.w, h: o.h, sx: p.x, sy: p.y, vx: view.x, vy: view.y, auto };
        auto = false;
      } else {
        // The picture itself: drag to move the view.
        drag = { kind: 'pan', sx: p.x, sy: p.y, vx: view.x, vy: view.y };
        panning = true;
      }
      return;
    }
    if (ed.tool === 'crop') {
      const h = handleAt(doc.crop, p);
      if (h) drag = { kind: 'crop', handle: h, start: doc.crop, px: c.x, py: c.y, fresh: false };
      else {
        const cs = ed.canvas;
        const x = clamp(c.x, 0, cs.w - 1);
        const y = clamp(c.y, 0, cs.h - 1);
        drag = { kind: 'crop', handle: 'se', start: { x, y, w: 1, h: 1 }, px: x, py: y, fresh: true };
      }
      return;
    }
    if (ed.tool === 'select') {
      const sel = ed.shape !== null ? doc.shapes.find((s: Shape) => s.id === ed.shape) : null;
      if (sel) {
        const h = handleAt(pad(shapeBounds(sel), sel), p);
        if (h && h !== 'move') {
          drag = { kind: 'scale', id: sel.id, start: sel, bounds: shapeBounds(sel), handle: h, px: c.x, py: c.y };
          return;
        }
      }
      const on = shapesOn(doc, doc.frames[ed.current].id);
      const target = [...on].reverse().find((s: Shape) => hit(s, c.x, c.y));
      ed.shape = target?.id ?? null;
      if (target) drag = { kind: 'move', id: target.id, start: target, px: c.x, py: c.y, moved: false };
      return;
    }
    if (DRAW_TOOLS.includes(ed.tool)) {
      ed.shape = null;
      draft = newShape(ed.tool as Shape['kind'], c.x, c.y);
      drag = { kind: 'draw', px: c.x, py: c.y, moved: false };
    }
  }

  function move(e: PointerEvent) {
    if (!pointers.has(e.pointerId)) return;
    const p = local(e);
    pointers.set(e.pointerId, p);
    if (pinch && pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      const dist = Math.hypot(a.x - b.x, a.y - b.y) || 1;
      const mx = (a.x + b.x) / 2;
      const my = (a.y + b.y) / 2;
      const z = clamp((pinch.zoom * dist) / pinch.dist, lowest(), MAX_ZOOM);
      const base = zoomAt({ zoom: pinch.zoom, x: pinch.vx, y: pinch.vy }, z, pinch.mx, pinch.my);
      view = { zoom: z, x: base.x + (mx - pinch.mx), y: base.y + (my - pinch.my) };
      auto = false;
      return;
    }
    if (!drag || !doc) return;
    const c = toCanvas(p);
    switch (drag.kind) {
      case 'pan':
        view = { ...view, x: drag.vx + p.x - drag.sx, y: drag.vy + p.y - drag.sy };
        auto = false;
        return;
      case 'size': {
        // Free by default; the proportions lock (or Shift, which flips it) keeps the shape.
        const lock = ed.keepAspect !== e.shiftKey;
        const s = stretchBy(drag.handle, drag.w, drag.h, (p.x - drag.sx) / view.zoom, (p.y - drag.sy) / view.zoom, lock);
        ed.preview(resize(ed.history.now ?? doc, s.w, s.h));
        // The opposite side stays where it is.
        view = { ...view, x: drag.handle.includes('w') ? drag.vx + (drag.w - s.w) * view.zoom : drag.vx, y: drag.handle.includes('n') ? drag.vy + (drag.h - s.h) * view.zoom : drag.vy };
        return;
      }
      case 'crop': {
        const cs = ed.canvas;
        let handle = drag.handle;
        let dx = c.x - drag.px;
        let dy = c.y - drag.py;
        if (drag.fresh) {
          // A new box grows toward wherever the pointer goes.
          handle = `${dy < 0 ? 'n' : 's'}${dx < 0 ? 'w' : 'e'}` as Handle;
          if (dx >= 0) dx -= 1;
          if (dy >= 0) dy -= 1;
        }
        const lock = ratio ?? (e.shiftKey && handle.length === 2 ? drag.start.w / drag.start.h : null);
        ed.preview(setCrop(doc, dragCrop(drag.start, handle, dx, dy, cs.w, cs.h, lock)));
        return;
      }
      case 'move': {
        const dx = c.x - drag.px;
        const dy = c.y - drag.py;
        if (!drag.moved && Math.hypot(dx, dy) * view.zoom < 3) return;
        drag.moved = true;
        ed.preview(updateShape(ed.history.now ?? doc, drag.id, { pts: moveShape(drag.start, dx, dy).pts }));
        return;
      }
      case 'scale': {
        const b = drag.bounds;
        let x1 = b.x;
        let y1 = b.y;
        let x2 = b.x + b.w;
        let y2 = b.y + b.h;
        const dx = c.x - drag.px;
        const dy = c.y - drag.py;
        if (drag.handle.includes('w')) x1 += dx;
        if (drag.handle.includes('e')) x2 += dx;
        if (drag.handle.includes('n')) y1 += dy;
        if (drag.handle.includes('s')) y2 += dy;
        if (e.shiftKey || drag.start.kind === 'text') {
          // Keep the shape's proportions.
          const k = Math.max(Math.abs(x2 - x1) / (b.w || 1), Math.abs(y2 - y1) / (b.h || 1));
          if (drag.handle.includes('w')) x1 = x2 - b.w * k;
          else x2 = x1 + b.w * k;
          if (drag.handle.includes('n')) y1 = y2 - b.h * k;
          else y2 = y1 + b.h * k;
        }
        const to = { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.max(1, Math.abs(x2 - x1)), h: Math.max(1, Math.abs(y2 - y1)) };
        const next = fitShape(drag.start, b, to);
        ed.preview(updateShape(ed.history.now ?? doc, drag.id, { pts: next.pts, size: next.size }));
        return;
      }
      case 'draw': {
        if (!draft) return;
        let { x, y } = c;
        const dx = x - drag.px;
        const dy = y - drag.py;
        if (!drag.moved && Math.hypot(dx, dy) * view.zoom < 2) return;
        drag.moved = true;
        if (draft.kind === 'pen') {
          const last = draft.pts.length - 2;
          if (Math.hypot(x - draft.pts[last], y - draft.pts[last + 1]) * view.zoom < 1.5) return;
          draft = { ...draft, pts: [...draft.pts, x, y] };
        } else if (draft.kind === 'line' || draft.kind === 'arrow') {
          if (e.shiftKey) {
            // Snap to 45°.
            const a = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
            const len = Math.hypot(dx, dy);
            x = drag.px + Math.cos(a) * len;
            y = drag.py + Math.sin(a) * len;
          }
          draft = { ...draft, pts: [drag.px, drag.py, x, y] };
        } else {
          let w = dx;
          let h = dy;
          if (e.shiftKey) {
            const s = Math.max(Math.abs(w), Math.abs(h));
            w = Math.sign(w || 1) * s;
            h = Math.sign(h || 1) * s;
          }
          draft = { ...draft, pts: [drag.px, drag.py, w, h] };
        }
        return;
      }
    }
  }

  function up(e: PointerEvent) {
    pointers.delete(e.pointerId);
    if (pinch) {
      if (pointers.size < 2) pinch = null;
      return;
    }
    const d = drag;
    drag = null;
    panning = false;
    if (!d || !doc) return;
    if (d.kind === 'size') {
      ed.commit(doc, ed.keepAspect ? 'Resize' : 'Stretch');
      // Fitted before, or grown past the stage: fit again, so every handle stays in reach.
      const o = outSize(doc);
      const r = screenRect({ x: 0, y: 0, w: o.w, h: o.h });
      if (d.auto || r.x < 0 || r.y < 0 || r.x + r.w > vw || r.y + r.h > vh) fit();
    } else if (d.kind === 'crop') ed.commit(doc, 'Crop');
    else if (d.kind === 'move' && d.moved) ed.commit(doc, 'Move drawing');
    else if (d.kind === 'scale') ed.commit(doc, 'Resize drawing');
    else if (d.kind === 'draw' && draft) {
      let s = draft;
      draft = null;
      if (s.kind === 'text') {
        const text = 'Text';
        const m = measureText(ctx, text, ed.brush.size);
        s = { ...s, text, size: ed.brush.size, pts: [s.pts[0], s.pts[1], m.w, m.h] };
      } else if (!d.moved && s.kind !== 'pen') return; // a click with a shape tool draws nothing
      if ((s.kind === 'rect' || s.kind === 'ellipse') && (Math.abs(s.pts[2]) < 1 || Math.abs(s.pts[3]) < 1)) return;
      const { id: _, ...rest } = s;
      const made = addShape(doc, rest);
      // Shapes are kept with positive sizes, so their handles behave.
      const b = shapeBounds({ ...rest, id: made.id });
      const next = s.kind === 'rect' || s.kind === 'ellipse' ? updateShape(made.doc, made.id, { pts: [b.x, b.y, b.w, b.h] }) : made.doc;
      ed.commit(next, `Draw ${labelOf(s.kind)}`);
      if (s.kind === 'text') {
        ed.shape = made.id;
        ed.focusText++;
      }
    }
  }

  const labelOf = (k: Shape['kind']) => ({ pen: 'with the pen', line: 'a line', arrow: 'an arrow', rect: 'a rectangle', ellipse: 'an ellipse', text: 'text' })[k];

  function cancelDrag() {
    if (drag && drag.kind !== 'pan') ed.revert();
    drag = null;
    draft = null;
    panning = false;
  }

  function wheel(e: WheelEvent) {
    if (!doc) return;
    // The stage zooms; the dialog and the page behind it never scroll from here.
    e.preventDefault();
    auto = false;
    const p = local(e);
    const pinch = e.ctrlKey || e.metaKey;
    if (e.shiftKey && !pinch) {
      view = { ...view, x: view.x - (e.deltaX || e.deltaY) };
      return;
    }
    view = zoomAt(view, wheelZoom(view.zoom, e.deltaY, e.deltaMode, pinch, lowest(), MAX_ZOOM, vh), p.x, p.y);
  }

  function keydown(e: KeyboardEvent) {
    if (!doc) return;
    if (e.key === ' ' && !e.repeat) {
      space = true;
      e.preventDefault();
      return;
    }
    if (e.key === 'Escape') {
      if (drag) {
        cancelDrag();
        e.preventDefault();
      } else if (ed.shape !== null) {
        ed.shape = null;
        e.preventDefault();
      }
      return;
    }
    const arrows: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    const step = arrows[e.key];
    if (step && !e.altKey && !e.ctrlKey && !e.metaKey) {
      const k = e.shiftKey ? 10 : 1;
      if (sizing) {
        // → ← wider and narrower, ↑ ↓ taller and shorter.
        e.preventDefault();
        const o = outSize(doc);
        const s = stretchBy(step[0] ? 'e' : 's', o.w, o.h, step[0] * k, -step[1] * k, ed.keepAspect);
        ed.commit(resize(doc, s.w, s.h), ed.keepAspect ? 'Resize' : 'Stretch');
        return;
      }
      const sel = ed.shape !== null ? doc.shapes.find((s: Shape) => s.id === ed.shape) : null;
      if (sel) {
        e.preventDefault();
        ed.commit(updateShape(doc, sel.id, { pts: moveShape(sel, step[0] * k, step[1] * k).pts }), 'Move drawing');
      } else if (ed.tool === 'crop') {
        e.preventDefault();
        const c = ed.canvas;
        ed.commit(setCrop(doc, dragCrop(doc.crop, 'move', step[0] * k, step[1] * k, c.w, c.h)), 'Move crop');
      }
      return;
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && ed.shape !== null) {
      e.preventDefault();
      e.stopPropagation();
      ed.commit(deleteShape(doc, ed.shape), 'Delete drawing');
      ed.shape = null;
    }
  }

  function keyup(e: KeyboardEvent) {
    if (e.key === ' ') space = false;
  }

  const zoomLabel = $derived(`${Math.round(view.zoom * 100)}%`);
  const out = $derived(doc ? outSize(doc) : { w: 1, h: 1 });
  const fits = $derived(shapeOk(out.w, out.h, ed.limits));
  const label = $derived(
    doc
      ? `Stage, frame ${ed.current + 1} of ${doc.frames.length}. ${sizing ? `Output ${out.w} by ${out.h} px. Arrow keys change the width and height.` : ed.tool === 'crop' ? 'Arrow keys move the crop.' : 'Arrow keys move the picked drawing, Delete removes it.'}`
      : 'Stage',
  );
</script>

<svelte:window onkeyup={keyup} onblur={() => (space = false)} />

<div class="stage" bind:this={box}>
  <!-- An application surface: pointer and keys are its whole point (the panel has the same actions as fields). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="hit"
    bind:this={surface}
    tabindex="0"
    role="application"
    aria-roledescription="editor stage"
    aria-label={label}
    style:cursor
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={(e) => {
      pointers.delete(e.pointerId);
      pinch = null;
      cancelDrag();
    }}
    onkeydown={keydown}
    oncontextmenu={(e) => e.preventDefault()}
  ><canvas bind:this={canvas} aria-hidden="true"></canvas></div>
  {#if doc && sizing}
    <p class="readout" class:bad={!fits}>
      <span class="ui-mono">{count(out.w)} × {count(out.h)} px · {ratioText(out.w, out.h)}</span>
      {#if fits}<span class="ok">within 1:{ed.limits.maxTall} to {ed.limits.maxAspect}:1</span>
      {:else}<span class="no"><EditorIcon name="fail" size={14} /> outside 1:{ed.limits.maxTall} to {ed.limits.maxAspect}:1, can’t upload</span>{/if}
    </p>
  {/if}
  {#if doc}
    <div class="zoom" role="group" aria-label="Zoom">
      <button type="button" class="zb" aria-label="Zoom out" title="Zoom out (−)" onclick={() => zoomBy(1 / 1.25)}><EditorIcon name="zoom-out" size={16} /></button>
      <button type="button" class="zl ui-mono" title="Actual size (1)" aria-label="Zoom {zoomLabel}, show actual size" onclick={actual}>{zoomLabel}</button>
      <button type="button" class="zb" aria-label="Zoom in" title="Zoom in (+)" onclick={() => zoomBy(1.25)}><EditorIcon name="zoom-in" size={16} /></button>
      <button type="button" class="zb" aria-label="Fit to stage" title="Fit to stage (0)" aria-pressed={auto} onclick={fit}><EditorIcon name="fit" size={16} /></button>
    </div>
  {/if}
</div>

<style>
  .stage { position: relative; min-width: 0; min-height: 0; overflow: hidden; background: var(--bg); }
  .hit { position: absolute; inset: 0; touch-action: none; outline: none; }
  .hit:focus-visible { box-shadow: inset 0 0 0 2px var(--focus-ring); }
  canvas { position: absolute; inset: 0; width: 100%; height: 100%; display: block; pointer-events: none; }
  .readout { position: absolute; left: 10px; top: 10px; display: flex; flex-wrap: wrap; align-items: center; gap: 4px 10px; max-width: calc(100% - 20px); margin: 0; padding: 5px 10px;
    background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-btn); box-shadow: var(--shadow-pop);
    font-size: var(--t-fine); color: var(--text); pointer-events: none; }
  .readout .ok { color: var(--dim); }
  .readout .no { display: inline-flex; align-items: center; gap: 4px; color: var(--red); font-weight: 600; }
  .zoom { position: absolute; right: 10px; bottom: 10px; display: flex; align-items: center; gap: 2px; padding: 2px;
    background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-btn); box-shadow: var(--shadow-pop); }
  .zb, .zl { display: inline-grid; place-items: center; height: 30px; min-width: 30px; padding: 0 6px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-fine); cursor: pointer; transition: color var(--dur-color), background-color var(--dur-color); }
  .zl { min-width: 52px; font-variant-numeric: tabular-nums; }
  .zb[aria-pressed='true'] { color: var(--accent-text); }
  .zb:active, .zl:active { transform: scale(var(--press)); }
  .zb:focus-visible, .zl:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .zb:hover, .zl:hover { color: var(--text); background: var(--surface-hover); } }
  @media (pointer: coarse) { .zb, .zl { height: 40px; min-width: 40px; } }
</style>
