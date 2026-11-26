<script lang="ts">
  // The entitlement graph drawn on a canvas: one dot per node in its kind's colour, hairline
  // links, and "Kind: name" labels where they fit (zoomed in, or on the hovered, focused, picked
  // and found nodes). Drag the ground to pan, wheel or pinch to zoom, drag a node to move it
  // (it is pinned while held and springs back into the layout when let go), click to pick,
  // double-click to open. The force layout (d3-force) runs a frame at a time and stops when it
  // settles or after a capped number of ticks; under reduced motion it is computed in one go and
  // drawn once. The radial layout puts each depth on a ring. Keyboard: Tab in, arrows move
  // between nodes, Enter picks, Esc clears, + and - zoom, 0 fits, Home goes to the root.
  import { onMount } from 'svelte';
  import { forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY, type Simulation, type SimulationLinkDatum, type SimulationNodeDatum } from 'd3-force';
  import { fitView, neighbourhood, NODE_KINDS, NODE_LABEL, radialLayout, stepFrom, visible, type Dir, type GNode, type Graph, type NodeKind, type View } from './entitlement-graph';

  type Props = {
    graph: Graph;
    hidden?: ReadonlySet<NodeKind>;
    selected?: string | null;
    /** Search hits: ringed, the rest dimmed a little. */
    matches?: ReadonlySet<string> | null;
    layout?: 'force' | 'radial';
    /** Node size, 1 to 10 (4 is the default). */
    size?: number;
    /** The name after "Kind: " (accounts' names arrive late). */
    labelOf: (n: GNode) => string;
    /** Changes when names arrive, so the labels are drawn again. */
    names?: unknown;
    label: string;
    description: string;
    onselect: (id: string | null) => void;
    onopen?: (n: GNode) => void;
  };
  let { graph, hidden = new Set(), selected = null, matches = null, layout = 'force', size = 4, labelOf, names, label, description, onselect, onopen }: Props = $props();

  type SNode = SimulationNodeDatum & { id: string; n: GNode; r: number; tx?: number; ty?: number };
  type SLink = SimulationLinkDatum<SNode> & { active: boolean };

  let wrap: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let w = 0, h = 0, dpr = 1;
  let view: View = { x: 0, y: 0, k: 1 };
  /** The reader has panned or zoomed: stop fitting the graph on every frame. */
  let moved = false;
  const byId = new Map<string, SNode>();
  let nodes: SNode[] = [];
  let links: SLink[] = [];
  let sim: Simulation<SNode, SLink> | null = null;
  let ticks = 0;
  const MAX_TICKS = 420;
  let tween: { from: Map<string, { x: number; y: number }>; start: number; ms: number } | null = null;
  let viewTween: { from: View; to: View; start: number; ms: number } | null = null;
  let raf = 0;
  let hover = $state<string | null>(null);
  let focusId = $state<string | null>(null);
  let focused = $state(false);
  let colours: Record<string, string> = {};

  const still = () => typeof matchMedia === 'function' && (document.documentElement.dataset.motion === 'reduce' || (matchMedia('(prefers-reduced-motion: reduce)').matches && document.documentElement.dataset.motion !== 'full'));
  const baseR = (n: GNode) => (n.root ? 10 : n.kind === 'role' || n.kind === 'badge' || n.kind === 'paint' || (n.kind === 'account' && !n.root) ? 4.5 : 7);
  const radius = (n: GNode) => baseR(n) * (0.4 + size * 0.15);

  function readColours() {
    if (!canvas) return;
    const cs = getComputedStyle(canvas);
    const v = (name: string) => cs.getPropertyValue(name).trim();
    colours = { bg: v('--bg'), text: v('--text'), muted: v('--muted'), dim: v('--dim'), border: v('--border-strong'), faint: v('--border'), accent: v('--accent-text'), focus: v('--focus-ring'), font: v('--font') };
    for (const k of NODE_KINDS) colours[k] = v(`--node-${k}`);
    draw();
  }

  // ---- The model: rebuilt when the graph, the hidden kinds or the layout change -------------

  function rebuild() {
    const vis = visible(graph, hidden);
    const keep = new Set(vis.nodes.map((n) => n.id));
    for (const id of [...byId.keys()]) if (!keep.has(id)) byId.delete(id);
    const seed = radialLayout(vis.nodes, graph.root);
    const fresh = byId.size === 0;
    nodes = vis.nodes.map((n) => {
      const s = byId.get(n.id);
      const p = seed.get(n.id) ?? { x: 0, y: 0 };
      if (s) {
        s.n = n;
        s.r = radius(n);
        return s;
      }
      // New nodes start at their parent (or their radial place when everything is new), so they grow out of it.
      const at = !fresh && n.parent ? byId.get(n.parent) : null;
      const made: SNode = { id: n.id, n, r: radius(n), x: at ? at.x! + (Math.random() - 0.5) * 8 : p.x, y: at ? at.y! + (Math.random() - 0.5) * 8 : p.y };
      byId.set(n.id, made);
      return made;
    });
    for (const s of nodes) {
      const p = seed.get(s.id)!;
      s.tx = p.x;
      s.ty = p.y;
    }
    // Links hold their nodes (as d3's link force would make them), so the radial layout can draw them too.
    links = vis.edges.map((e) => ({ source: byId.get(e.source)!, target: byId.get(e.target)!, active: e.active }));
    sim?.stop();
    sim = null;
    if (fresh && w) view = fitAll();
    if (layout === 'force') startForce(fresh ? 1 : 0.7);
    else toRadial();
  }

  function startForce(alpha: number) {
    for (const s of nodes) {
      s.fx = null;
      s.fy = null;
    }
    sim = forceSimulation<SNode, SLink>(nodes)
      .stop()
      .alpha(alpha)
      .force('link', forceLink<SNode, SLink>(links).id((d) => d.id)
        .distance((l) => ((l.source as SNode).n.root ? 120 : (l.target as SNode).n.depth >= 2 && isLeaf((l.target as SNode).n) ? 34 : 70)))
      .force('charge', forceManyBody<SNode>().strength((d) => (isLeaf(d.n) ? -40 : -260)).distanceMax(600).theta(0.9))
      .force('collide', forceCollide<SNode>((d) => d.r + 3).iterations(1))
      .force('x', forceX<SNode>(0).strength(0.03))
      .force('y', forceY<SNode>(0).strength(0.03));
    ticks = 0;
    if (still()) {
      sim.tick(300);
      sim.stop();
      if (!moved) view = fitAll();
      draw();
    } else kick();
  }

  const isLeaf = (n: GNode) => n.kind === 'role' || n.kind === 'badge' || n.kind === 'paint' || (n.kind === 'account' && !n.root);

  function toRadial() {
    for (const s of nodes) {
      s.fx = null;
      s.fy = null;
    }
    if (still()) {
      for (const s of nodes) {
        s.x = s.tx;
        s.y = s.ty;
      }
      if (!moved) view = fitAll();
      draw();
      return;
    }
    tween = { from: new Map(nodes.map((s) => [s.id, { x: s.x!, y: s.y! }])), start: performance.now(), ms: 520 };
    kick();
  }

  const points = () => nodes.map((s) => ({ x: s.x!, y: s.y!, r: s.r }));
  /** Fits the graph, leaving room on the right for the labels beside the outer nodes. */
  function fitAll(): View {
    const room = w < 600 ? 40 : 120;
    // Never past 1.6: a small graph stays at a readable size instead of filling the frame with huge dots.
    const v = fitView(points(), w - room, h, w < 600 ? 16 : 40, 1.6);
    return v;
  }

  // ---- The frame loop: only runs while something moves ------------------------------------

  function kick() {
    if (!raf && typeof requestAnimationFrame === 'function') raf = requestAnimationFrame(frame);
  }

  const ease = (t: number) => 1 - Math.pow(1 - t, 3);

  function frame(now: number) {
    raf = 0;
    let more = false;
    if (sim) {
      // Big graphs take two ticks a frame so they settle in about the same time.
      const per = nodes.length > 400 ? 2 : 1;
      for (let i = 0; i < per; i++) sim.tick();
      ticks += per;
      if (sim.alpha() > sim.alphaMin() && (ticks < MAX_TICKS || dragging)) more = true;
    }
    if (tween) {
      const t = Math.min(1, (now - tween.start) / tween.ms);
      const e = ease(t);
      for (const s of nodes) {
        if (s.fx != null) continue;
        const f = tween.from.get(s.id) ?? { x: s.tx!, y: s.ty! };
        s.x = f.x + (s.tx! - f.x) * e;
        s.y = f.y + (s.ty! - f.y) * e;
      }
      if (t < 1) more = true;
      else tween = null;
    }
    if (viewTween) {
      const t = Math.min(1, (now - viewTween.start) / viewTween.ms);
      const e = ease(t);
      const a = viewTween.from, b = viewTween.to;
      view = { x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, k: a.k + (b.k - a.k) * e };
      if (t < 1) more = true;
      else viewTween = null;
    } else if (!moved && (sim || tween)) view = fitAll();
    draw();
    if (more) kick();
  }

  // ---- Drawing ------------------------------------------------------------------------------

  function draw() {
    if (!ctx || !w) return;
    const c = ctx;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    c.fillStyle = colours.bg || '#000';
    c.fillRect(0, 0, w, h);
    const focusSet = selected && byId.has(selected) ? neighbourhood(links.map((l) => ({ source: (l.source as SNode).id, target: (l.target as SNode).id, active: l.active })), selected) : null;
    const hits = matches && matches.size ? matches : null;
    const lit = (id: string) => (focusSet ? focusSet.has(id) : hits ? hits.has(id) : true);
    const k = view.k;
    c.setTransform(dpr * k, 0, 0, dpr * k, dpr * view.x, dpr * view.y);

    // Links: the dimmed first, then the lit ones over them.
    c.lineWidth = 1 / k;
    for (const pass of [false, true]) {
      c.beginPath();
      for (const l of links) {
        const s = l.source as SNode, t = l.target as SNode;
        const on = focusSet ? focusSet.has(s.id) && focusSet.has(t.id) && (s.id === selected || t.id === selected) : !hits || (hits.has(s.id) && hits.has(t.id));
        if (on !== pass) continue;
        c.moveTo(s.x!, s.y!);
        c.lineTo(t.x!, t.y!);
      }
      c.globalAlpha = pass ? 1 : focusSet || hits ? 0.35 : 1;
      c.strokeStyle = pass && (focusSet || hits) ? colours.muted : colours.border;
      c.lineWidth = (pass && focusSet ? 1.5 : 1) / k;
      c.stroke();
    }
    c.globalAlpha = 1;

    // Nodes, batched by kind; something taken away is a ring, not a dot.
    for (const dim of [true, false]) {
      c.globalAlpha = dim ? 0.18 : 1;
      for (const s of nodes) {
        if (lit(s.id) === dim) continue;
        const col = colours[s.n.kind] || colours.text;
        c.beginPath();
        c.arc(s.x!, s.y!, s.r, 0, Math.PI * 2);
        if (s.n.active || s.n.root) {
          c.fillStyle = col;
          c.fill();
          c.lineWidth = 1.5 / k;
          c.strokeStyle = colours.bg;
          c.stroke();
        } else {
          c.fillStyle = colours.bg;
          c.fill();
          c.lineWidth = Math.max(1.5 / k, s.r * 0.35);
          c.strokeStyle = col;
          c.stroke();
        }
      }
    }
    c.globalAlpha = 1;
    const ring = (id: string | null, colour: string, gap: number, width: number, dash = false) => {
      const s = id ? byId.get(id) : null;
      if (!s) return;
      c.beginPath();
      c.arc(s.x!, s.y!, s.r + gap / k, 0, Math.PI * 2);
      c.setLineDash(dash ? [3 / k, 3 / k] : []);
      c.lineWidth = width / k;
      c.strokeStyle = colour;
      c.stroke();
      c.setLineDash([]);
    };
    if (hits && !focusSet) for (const id of hits) ring(id, colours.text, 3, 1.5);
    ring(selected, colours.accent, 3.5, 2.5);
    if (focused) ring(focusId, colours.focus, 7, 2, true);

    // Labels in screen space (crisp at any zoom), the important ones first; one that would overlap
    // a label already drawn is left out.
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    const font = colours.font || 'system-ui, sans-serif';
    c.textBaseline = 'middle';
    c.lineJoin = 'round';
    const taken: [number, number, number, number][] = [];
    const prio: SNode[] = [];
    const add = (id: string | null | undefined) => {
      const s = id ? byId.get(id) : null;
      if (s && !prio.includes(s)) prio.push(s);
    };
    add(hover);
    add(selected);
    if (focused) add(focusId);
    add(graph.root);
    if (focusSet) for (const id of focusSet) add(id);
    if (hits) for (const id of [...hits].slice(0, 80)) add(id);
    const sources = nodes.filter((s) => !isLeaf(s.n) && !s.n.root);
    const leaves = nodes.filter((s) => isLeaf(s.n));
    const all = [...prio, ...(k >= 0.45 ? sources : []), ...(k * 4.5 * (0.4 + size * 0.15) >= 5.5 ? leaves : [])];
    const done = new Set<string>();
    let drawn = 0;
    for (const s of all) {
      if (done.has(s.id) || drawn > 450) continue;
      done.add(s.id);
      if (!lit(s.id) && !prio.includes(s)) continue;
      const sx = view.x + s.x! * k, sy = view.y + s.y! * k;
      if (sx < -200 || sx > w + 200 || sy < -20 || sy > h + 20) continue;
      const strong = s.id === selected || s.id === hover || s.n.root;
      const kind = `${NODE_LABEL[s.n.kind]}: `;
      const name = labelOf(s.n) || '…';
      c.font = `500 12px ${font}`;
      const kw = c.measureText(kind).width;
      c.font = `${strong ? 600 : 500} 12px ${font}`;
      const nw = c.measureText(name).width;
      const x = sx + s.r * k + 5, y = sy;
      const box: [number, number, number, number] = [x - 2, y - 8, x + kw + nw + 2, y + 8];
      const forced = s.id === hover || s.id === selected || (focused && s.id === focusId);
      if (!forced && taken.some((b) => box[0] < b[2] && box[2] > b[0] && box[1] < b[3] && box[3] > b[1])) continue;
      taken.push(box);
      drawn++;
      c.lineWidth = 3;
      c.strokeStyle = colours.bg;
      c.font = `500 12px ${font}`;
      c.strokeText(kind, x, y);
      c.fillStyle = colours.dim;
      c.fillText(kind, x, y);
      c.font = `${strong ? 600 : 500} 12px ${font}`;
      c.strokeText(name, x + kw, y);
      c.fillStyle = s.n.active || s.n.root ? colours.text : colours.muted;
      c.fillText(name, x + kw, y);
    }
  }

  // ---- Pointer: pan, zoom, drag, pick --------------------------------------------------------

  const toWorld = (px: number, py: number) => ({ x: (px - view.x) / view.k, y: (py - view.y) / view.k });
  function hit(px: number, py: number): SNode | null {
    const p = toWorld(px, py);
    let best: SNode | null = null;
    let bd = Infinity;
    for (const s of nodes) {
      const d = Math.hypot(s.x! - p.x, s.y! - p.y);
      const reach = s.r + 6 / view.k;
      if (d < reach && d < bd) {
        bd = d;
        best = s;
      }
    }
    return best;
  }
  const pos = (e: PointerEvent | WheelEvent | MouseEvent) => {
    const b = canvas.getBoundingClientRect();
    return { x: e.clientX - b.left, y: e.clientY - b.top };
  };

  const pointers = new Map<number, { x: number; y: number }>();
  let dragging: SNode | null = null;
  let panFrom: { x: number; y: number; vx: number; vy: number } | null = null;
  let pinch: { d: number; k: number; mx: number; my: number; vx: number; vy: number } | null = null;
  let downAt: { x: number; y: number; node: SNode | null } | null = null;
  let travelled = 0;

  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0 && e.pointerType === 'mouse') return;
    canvas.setPointerCapture(e.pointerId);
    const p = pos(e);
    pointers.set(e.pointerId, p);
    viewTween = null;
    if (pointers.size === 2) {
      // Two fingers: pinch to zoom around their middle (a node being dragged is let go).
      release();
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a.x - b.x, a.y - b.y) || 1, k: view.k, mx: (a.x + b.x) / 2, my: (a.y + b.y) / 2, vx: view.x, vy: view.y };
      panFrom = null;
      downAt = null;
      return;
    }
    travelled = 0;
    const s = hit(p.x, p.y);
    downAt = { ...p, node: s };
    if (s) {
      dragging = s;
      s.fx = s.x;
      s.fy = s.y;
      if (layout === 'force') {
        if (!sim) startForce(0.1);
        sim?.alphaTarget(0.25);
        kick();
      }
    } else panFrom = { ...p, vx: view.x, vy: view.y };
  }

  function onpointermove(e: PointerEvent) {
    const p = pos(e);
    if (!pointers.has(e.pointerId)) {
      // Hover: the node under the pointer shows its label.
      const s = hit(p.x, p.y);
      const id = s?.id ?? null;
      canvas.style.cursor = s ? 'pointer' : 'grab';
      if (id !== hover) {
        hover = id;
        draw();
      }
      return;
    }
    const last = pointers.get(e.pointerId)!;
    travelled += Math.hypot(p.x - last.x, p.y - last.y);
    pointers.set(e.pointerId, p);
    if (pinch && pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      const k = clampK(pinch.k * (Math.hypot(a.x - b.x, a.y - b.y) / pinch.d));
      const mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2;
      view = { k, x: mx - ((pinch.mx - pinch.vx) / pinch.k) * k, y: my - ((pinch.my - pinch.vy) / pinch.k) * k };
      moved = true;
      draw();
      return;
    }
    if (dragging && travelled > 3) {
      const wp = toWorld(p.x, p.y);
      dragging.fx = wp.x;
      dragging.fy = wp.y;
      dragging.x = wp.x;
      dragging.y = wp.y;
      canvas.style.cursor = 'grabbing';
      if (layout === 'force') kick();
      else draw();
    } else if (panFrom) {
      view = { ...view, x: panFrom.vx + p.x - panFrom.x, y: panFrom.vy + p.y - panFrom.y };
      if (travelled > 3) {
        moved = true;
        canvas.style.cursor = 'grabbing';
      }
      draw();
    }
  }

  /** Lets go of a dragged node: back into the force layout, or back to its ring slot. */
  function release() {
    const s = dragging;
    dragging = null;
    if (!s) return;
    s.fx = null;
    s.fy = null;
    if (layout === 'force') {
      sim?.alphaTarget(0);
      if (sim && sim.alpha() < 0.12) sim.alpha(0.12);
      ticks = Math.min(ticks, MAX_TICKS - 120);
      kick();
    } else if (still()) {
      s.x = s.tx;
      s.y = s.ty;
      draw();
    } else {
      tween = { from: new Map(nodes.map((n) => [n.id, { x: n.x!, y: n.y! }])), start: performance.now(), ms: 420 };
      kick();
    }
  }

  function onpointerup(e: PointerEvent) {
    pointers.delete(e.pointerId);
    if (pinch) {
      if (pointers.size < 2) pinch = null;
      return;
    }
    const click = downAt && travelled <= 3;
    const node = downAt?.node ?? null;
    release();
    panFrom = null;
    downAt = null;
    canvas.style.cursor = node ? 'pointer' : 'grab';
    if (click) {
      focusId = node?.id ?? focusId;
      onselect(node ? node.id : null);
    }
  }

  function ondblclick(e: MouseEvent) {
    const p = pos(e);
    const s = hit(p.x, p.y);
    if (s && !s.n.root) onopen?.(s.n);
  }

  const clampK = (k: number) => Math.max(0.05, Math.min(6, k));
  function zoomAt(f: number, px = w / 2, py = h / 2, animate = false) {
    const k = clampK(view.k * f);
    const to = { k, x: px - ((px - view.x) / view.k) * k, y: py - ((py - view.y) / view.k) * k };
    moved = true;
    setView(to, animate);
  }
  function setView(to: View, animate: boolean) {
    if (animate && !still()) {
      viewTween = { from: view, to, start: performance.now(), ms: 320 };
      kick();
    } else {
      view = to;
      draw();
    }
  }
  function onwheel(e: WheelEvent) {
    e.preventDefault();
    const p = pos(e);
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
    zoomAt(Math.exp(-dy * (e.ctrlKey ? 0.01 : 0.0015)), p.x, p.y);
  }

  // ---- What the page can ask -----------------------------------------------------------------

  /** Fits the whole graph in view (and goes back to following the layout). */
  export function fit() {
    moved = false;
    setView(fitAll(), true);
  }
  /** Zooms around the middle by a factor. */
  export function zoom(f: number) {
    zoomAt(f, w / 2, h / 2, true);
  }
  /** Centres a node, zoomed in enough to read its neighbours. */
  export function centre(id: string) {
    const s = byId.get(id);
    if (!s) return;
    focusId = id;
    const k = Math.max(view.k, 1.4);
    moved = true;
    setView({ k, x: w / 2 - s.x! * k, y: h / 2 - s.y! * k }, true);
  }

  // ---- Keyboard ------------------------------------------------------------------------------

  const KEYS: Record<string, Dir> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
  function onkeydown(e: KeyboardEvent) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const d = KEYS[e.key];
    if (d) {
      e.preventDefault();
      const to = stepFrom(nodes.map((s) => ({ id: s.id, x: s.x!, y: s.y! })), focusId ?? graph.root, d);
      if (to) moveFocus(to);
    } else if (e.key === 'Home') {
      e.preventDefault();
      moveFocus(graph.root);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (focusId) onselect(focusId);
    } else if (e.key === 'Escape') {
      if (selected) {
        e.preventDefault();
        onselect(null);
      }
    } else if (e.key === '+' || e.key === '=') {
      e.preventDefault();
      zoom(1.3);
    } else if (e.key === '-' || e.key === '_') {
      e.preventDefault();
      zoom(1 / 1.3);
    } else if (e.key === '0') {
      e.preventDefault();
      fit();
    }
  }
  function moveFocus(id: string) {
    focusId = id;
    const s = byId.get(id);
    // Keep the focused node in view.
    if (s) {
      const sx = view.x + s.x! * view.k, sy = view.y + s.y! * view.k;
      if (sx < 40 || sx > w - 40 || sy < 40 || sy > h - 40) {
        moved = true;
        setView({ ...view, x: w / 2 - s.x! * view.k, y: h / 2 - s.y! * view.k }, true);
      }
    }
    draw();
  }
  const focusNode = $derived(focusId ? graph.byId.get(focusId) ?? null : null);
  const announce = $derived(
    focused && focusNode
      ? `${NODE_LABEL[focusNode.kind]}: ${labelOf(focusNode) || 'unnamed'}${focusNode.root ? '' : focusNode.active ? ', held now' : ', taken away'}, ${graph.edges.filter((e) => e.source === focusNode.id || e.target === focusNode.id).length} links${focusNode.id === selected ? ', picked' : ''}`
      : '',
  );

  // ---- Wiring --------------------------------------------------------------------------------

  function resize() {
    if (!wrap || !canvas) return;
    const b = wrap.getBoundingClientRect();
    dpr = Math.min(3, window.devicePixelRatio || 1);
    w = Math.max(1, Math.round(b.width));
    h = Math.max(1, Math.round(b.height));
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    if (!moved) view = fitAll();
    draw();
  }

  onMount(() => {
    ctx = canvas.getContext('2d');
    resize();
    readColours();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);
    const mo = new MutationObserver(readColours);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'data-motion'] });
    const mq = matchMedia('(prefers-color-scheme: dark)');
    mq.addEventListener?.('change', readColours);
    canvas.addEventListener('wheel', onwheel, { passive: false });
    return () => {
      ro.disconnect();
      mo.disconnect();
      mq.removeEventListener?.('change', readColours);
      canvas.removeEventListener('wheel', onwheel);
      if (raf) cancelAnimationFrame(raf);
      sim?.stop();
    };
  });

  // The structure: a new graph starts over; hiding a kind or switching layout reflows what is there.
  let lastGraph: Graph | null = null;
  $effect(() => {
    void hidden;
    void layout;
    if (graph !== lastGraph) {
      byId.clear();
      lastGraph = graph;
      moved = false;
      focusId = selected ?? graph.root;
    }
    rebuild();
  });
  // Size changes the radii only.
  $effect(() => {
    void size;
    for (const s of nodes) s.r = radius(s.n);
    if (sim) {
      sim.force('collide', forceCollide<SNode>((d) => d.r + 3).iterations(1));
      if (layout === 'force' && sim.alpha() < 0.1) {
        sim.alpha(0.2);
        ticks = 0;
        kick();
      }
    }
    draw();
  });
  // Picks, hits and names only redraw.
  $effect(() => {
    void selected;
    void matches;
    void names;
    if (selected) focusId = selected;
    draw();
  });
</script>

<div class="wrap" bind:this={wrap}>
  <!-- The focusable graph: keyboard on the wrapper (a canvas can't take the application role), the
       pointer on the canvas. An application is a widget that takes its own keys; the checker doesn't list it. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="kb"
    tabindex="0"
    role="application"
    aria-roledescription="graph"
    aria-label={label}
    aria-describedby="graph-desc"
    onkeydown={onkeydown}
    onfocus={() => { focused = true; focusId ??= selected ?? graph.root; draw(); }}
    onblur={() => { focused = false; draw(); }}
  >
    <canvas
      bind:this={canvas}
      aria-hidden="true"
      onpointerdown={onpointerdown}
      onpointermove={onpointermove}
      onpointerup={onpointerup}
      onpointercancel={onpointerup}
      onpointerleave={() => { if (hover && !pointers.size) { hover = null; draw(); } }}
      ondblclick={ondblclick}
    ></canvas>
  </div>
  <p id="graph-desc" class="sr-only">{description} Arrow keys move between nodes, Enter picks one, Escape clears the pick, plus and minus zoom, 0 fits the graph.</p>
  <p class="sr-only" aria-live="polite">{announce}</p>
</div>

<style>
  .wrap { position: absolute; inset: 0; }
  .kb { position: absolute; inset: 0; outline: none; }
  .kb:focus-visible { box-shadow: inset 0 0 0 2px var(--focus-ring); border-radius: 0; }
  canvas { display: block; width: 100%; height: 100%; touch-action: none; cursor: grab; }
</style>
