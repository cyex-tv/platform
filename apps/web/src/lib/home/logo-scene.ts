// The hero's 3D logo: the three open bars of logo.ts extruded into frames, the middle one a step
// forward and lit (emissive accent, a soft inner light and a halo). It turns slowly on its own and
// tilts back as the hero scrolls away; it never follows the pointer (cyex-web).
//
// Loaded on its own (dynamic import, after first paint, capable devices only: see LogoHero),
// so three.js never sits in the page's first bundle. It draws only while the hero is on screen and
// the tab is visible, and gives everything back on dispose.
import {
  AdditiveBlending,
  AmbientLight,
  CanvasTexture,
  Color,
  DirectionalLight,
  ExtrudeGeometry,
  Group,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  NormalBlending,
  Path,
  PerspectiveCamera,
  PlaneGeometry,
  PointLight,
  Scene,
  Shape,
  SRGBColorSpace,
  WebGLRenderer,
  type Material,
  type BufferGeometry,
  type Texture,
} from 'three';
import { BARS, CENTER, GRID, STROKE, type Bar } from './logo';

export type LogoScene = { dispose(): void };

type Options = {
  /** The first frame is on the canvas: fade it in. */
  onReady: () => void;
  /** WebGL went away (context lost, or it couldn't start): keep the flat logo. */
  onFail: () => void;
};

const DEPTH = 1.1; // how deep each frame is, in grid units (a bar is 6 tall)
const LIFT = 0.9; // how far the lit bar stands in front of the others
const FOV = 18; // narrow, so the front-on start matches the flat SVG
const PX = 16; // texture pixels per grid unit for the glows
const PAD = 4; // glow room around a bar, grid units

/** A soft white light texture: a blurred frame line (`line`) or a blurred fill (`fill`), tinted by the material. */
function glow(b: Bar, kind: 'line' | 'fill', blur: number, width = 0): CanvasTexture {
  const w = Math.round((b.w + PAD * 2) * PX);
  const h = Math.round((b.h + PAD * 2) * PX);
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const g = c.getContext('2d')!;
  // The shape is drawn far off the canvas and only its shadow lands on it: a blur every browser has.
  const off = w * 4;
  g.shadowColor = '#ffffff';
  g.shadowBlur = blur * PX;
  g.shadowOffsetX = off;
  g.strokeStyle = g.fillStyle = '#ffffff';
  const x = PAD * PX - off;
  const y = PAD * PX;
  if (kind === 'line') {
    g.lineWidth = width * PX;
    const half = (STROKE / 2) * PX;
    g.strokeRect(x + half, y + half, b.w * PX - half * 2, b.h * PX - half * 2);
  } else {
    const inset = STROKE * 1.6 * PX;
    g.fillRect(x + inset, y + inset, b.w * PX - inset * 2, b.h * PX - inset * 2);
  }
  const t = new CanvasTexture(c);
  t.colorSpace = SRGBColorSpace;
  return t;
}

/** One open bar: a rectangle with a rectangular hole, extruded, centred on its own middle. */
function frame(b: Bar): ExtrudeGeometry {
  const bevel = 0.1;
  // The bevel grows the outline, so the shape is drawn that much smaller to keep the footprint.
  const w = b.w / 2 - bevel;
  const h = b.h / 2 - bevel;
  const iw = b.w / 2 - STROKE + bevel;
  const ih = b.h / 2 - STROKE + bevel;
  const outer = new Shape().moveTo(-w, -h).lineTo(w, -h).lineTo(w, h).lineTo(-w, h).lineTo(-w, -h);
  outer.holes.push(new Path().moveTo(-iw, -ih).lineTo(-iw, ih).lineTo(iw, ih).lineTo(iw, -ih).lineTo(-iw, -ih));
  const geo = new ExtrudeGeometry(outer, { depth: DEPTH - bevel * 2, bevelEnabled: true, bevelThickness: bevel, bevelSize: bevel, bevelSegments: 2, curveSegments: 1 });
  geo.translate(0, 0, -(DEPTH - bevel * 2) / 2);
  return geo;
}

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
const isLight = () => document.documentElement.dataset.theme === 'light';

export function mountLogo(canvas: HTMLCanvasElement, host: HTMLElement, { onReady, onFail }: Options): LogoScene {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.setClearColor(0x000000, 0);

  const scene = new Scene();
  const camera = new PerspectiveCamera(FOV, 1, 1, 400);
  const logo = new Group();
  scene.add(logo);

  const geos: BufferGeometry[] = [];
  const mats: Material[] = [];
  const texs: Texture[] = [];
  const own = <T extends Material>(m: T) => (mats.push(m), m);

  const ink = own(new MeshStandardMaterial({ roughness: 0.55, metalness: 0.15 }));
  const lit = own(new MeshStandardMaterial({ roughness: 0.35, metalness: 0, emissiveIntensity: 0.85 }));
  const inkHalo = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));
  const halo = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));
  const core = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));

  let litBar: Group | null = null;
  for (const b of BARS) {
    const bar = new Group();
    bar.position.set(b.x + b.w / 2 - CENTER.x, CENTER.y - (b.y + b.h / 2), b.lit ? LIFT : 0);
    const geo = frame(b);
    geos.push(geo);
    bar.add(new Mesh(geo, b.lit ? lit : ink));

    const plane = new PlaneGeometry(b.w + PAD * 2, b.h + PAD * 2);
    geos.push(plane);
    if (b.lit) {
      const line = glow(b, 'line', 2.2, STROKE * 1.8);
      texs.push(line);
      halo.map = line;
      const h = new Mesh(plane, halo);
      h.position.z = -DEPTH / 2;
      h.renderOrder = 1;
      const fill = glow(b, 'fill', 1);
      texs.push(fill);
      core.map = fill;
      const c = new Mesh(plane, core);
      c.position.z = -DEPTH / 2 + 0.01;
      c.renderOrder = 2;
      bar.add(h, c);
      litBar = bar;
    } else {
      // Both ink bars are the same size: one texture serves the two.
      if (!inkHalo.map) {
        inkHalo.map = glow(b, 'line', 1, STROKE);
        texs.push(inkHalo.map);
      }
      const h = new Mesh(plane, inkHalo);
      h.position.z = -DEPTH / 2;
      h.renderOrder = 1;
      bar.add(h);
    }
    logo.add(bar);
  }

  scene.add(new AmbientLight(0xffffff, 0.9));
  const key = new DirectionalLight(0xffffff, 1.8);
  key.position.set(-8, 10, 14);
  const fill = new DirectionalLight(0xffffff, 0.5);
  fill.position.set(10, -6, 8);
  // The lit bar's light falls on the inner edges of the other two.
  const spill = new PointLight(0xffffff, 60, 16, 1.6);
  spill.position.set(4, 0, LIFT + 1.5);
  scene.add(key, fill, spill);

  function paint() {
    const light = isLight();
    const accent = new Color(css('--accent') || (light ? '#14e6ff' : '#145aff'));
    const markInk = new Color(css('--mark-ink') || (light ? '#1b1c20' : '#e8e8ea'));
    ink.color.copy(markInk);
    ink.emissive.copy(markInk).multiplyScalar(light ? 0 : 0.35);
    lit.color.copy(accent);
    lit.emissive.copy(accent);
    spill.color.copy(accent);
    spill.intensity = light ? 25 : 60;
    for (const m of [halo, core, inkHalo]) m.color.copy(m === inkHalo ? markInk : accent);
    // On dark the glow adds light; on white it has to lay colour down instead.
    halo.blending = core.blending = light ? NormalBlending : AdditiveBlending;
    halo.opacity = light ? 0.75 : 0.95;
    core.opacity = light ? 0.4 : 0.55;
    inkHalo.blending = AdditiveBlending;
    inkHalo.opacity = light ? 0 : 0.14;
    inkHalo.visible = !light;
    for (const m of mats) m.needsUpdate = true;
    draw();
  }

  function fit() {
    const w = Math.max(1, host.clientWidth);
    const h = Math.max(1, host.clientHeight);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    // Fit the grid like the SVG does (meet): the shorter side holds all 32 units.
    const need = camera.aspect >= 1 ? GRID : GRID / camera.aspect;
    camera.position.set(0, 0, need / 2 / Math.tan((FOV * Math.PI) / 360));
    camera.updateProjectionMatrix();
    draw();
  }

  // Scroll: 0 with the hero in place, 1 once it has scrolled its own height away. Eased toward.
  let scroll = 0;
  let eased = 0;
  const readScroll = () => {
    const r = host.getBoundingClientRect();
    scroll = Math.min(1, Math.max(0, -r.top / Math.max(1, r.height)));
  };

  const start = performance.now();
  let last = start;
  function pose(now: number) {
    const t = (now - start) / 1000;
    // The turn grows in over the first seconds, so the fade from the flat logo shows no jump.
    const k = Math.min(1, t / 3);
    const grow = k * k * (3 - 2 * k);
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    eased += (scroll - eased) * (1 - Math.exp(-dt * 6));
    logo.rotation.y = grow * 0.34 * Math.sin((t / 18) * Math.PI * 2);
    logo.rotation.x = grow * 0.08 * Math.sin((t / 26) * Math.PI * 2) - eased * 0.5;
    if (litBar) litBar.position.z = LIFT + eased * 1.6;
  }

  function draw() {
    renderer.render(scene, camera);
  }

  let raf = 0;
  let onScreen = true;
  let ready = false;
  // The turn is slow, so 30 frames a second look the same as 60 and cost half the GPU time.
  let drawn = 0;
  const loop = (now: number) => {
    raf = 0;
    if (ready && now - drawn < 32) return schedule();
    drawn = now;
    pose(now);
    draw();
    if (!ready) {
      ready = true;
      onReady();
    }
    schedule();
  };
  function schedule() {
    if (!raf && onScreen && !document.hidden) {
      raf = requestAnimationFrame(loop);
    }
  }
  function halt() {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }

  const io = new IntersectionObserver(([e]) => {
    onScreen = e.isIntersecting;
    if (onScreen) {
      last = performance.now();
      schedule();
    } else halt();
  });
  io.observe(host);
  const ro = new ResizeObserver(fit);
  ro.observe(host);
  const mo = new MutationObserver(paint);
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  const onVisible = () => {
    if (document.hidden) halt();
    else {
      last = performance.now();
      schedule();
    }
  };
  document.addEventListener('visibilitychange', onVisible);
  addEventListener('scroll', readScroll, { passive: true });
  const lost = (e: Event) => {
    e.preventDefault();
    halt();
    onFail();
  };
  canvas.addEventListener('webglcontextlost', lost);

  readScroll();
  eased = scroll;
  fit();
  paint();
  schedule();

  return {
    dispose() {
      halt();
      io.disconnect();
      ro.disconnect();
      mo.disconnect();
      document.removeEventListener('visibilitychange', onVisible);
      removeEventListener('scroll', readScroll);
      canvas.removeEventListener('webglcontextlost', lost);
      for (const g of geos) g.dispose();
      for (const m of mats) m.dispose();
      for (const t of texs) t.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
    },
  };
}
