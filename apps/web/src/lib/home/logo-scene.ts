// The hero's 3D logo: the three open bars of logo.ts as deep, bevelled frames, seen from above
// and to the side so their tops and sides show. The middle bar stands well in front, its faces lit
// from inside (emissive accent: the front brightest, the sides less), with a glow through its
// opening and around it; its light spills onto the inner faces of the other two.
//
// Motion: it first shows the still pose (the same one as the static image under it), makes one
// slow full turn, then sways and floats on its own, and turns further and lets the lit bar come
// forward as the hero scrolls away. It never follows the pointer (cyex-web).
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
  PMREMGenerator,
  PointLight,
  Scene,
  Shape,
  SRGBColorSpace,
  WebGLRenderer,
  type BufferGeometry,
  type Material,
  type Texture,
} from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { BARS, CENTER, GRID, STROKE, type Bar } from './logo';
import { REST, pose, type Pose } from './logo-motion';

export type LogoScene = { dispose(): void };

type Options = {
  /** The first frame is on the canvas: fade it in. */
  onReady: () => void;
  /** WebGL went away (context lost, or it couldn't start): keep the static logo. */
  onFail: () => void;
  /** Draw the still pose once and never move (reduced motion turned on while it ran). */
  still?: boolean;
};

const DEPTH = 2.6; // how deep each frame is, in grid units (a bar is 6 tall)
const BEVEL = 0.34;
const FOV = 26;
const FIT = GRID * 1.1; // the view holds this many units, so the turned logo and its glow fit
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
    const inset = STROKE * 1.4 * PX;
    g.fillRect(x + inset, y + inset, b.w * PX - inset * 2, b.h * PX - inset * 2);
  }
  const t = new CanvasTexture(c);
  t.colorSpace = SRGBColorSpace;
  return t;
}

/** One open bar: a rectangle with a rectangular hole, extruded with a rounded bevel, centred on its own middle. */
function frame(b: Bar): ExtrudeGeometry {
  // The bevel grows the outline, so the shape is drawn that much smaller to keep the footprint.
  const w = b.w / 2 - BEVEL;
  const h = b.h / 2 - BEVEL;
  const iw = b.w / 2 - STROKE + BEVEL;
  const ih = b.h / 2 - STROKE + BEVEL;
  const outer = new Shape().moveTo(-w, -h).lineTo(w, -h).lineTo(w, h).lineTo(-w, h).lineTo(-w, -h);
  outer.holes.push(new Path().moveTo(-iw, -ih).lineTo(-iw, ih).lineTo(iw, ih).lineTo(iw, -ih).lineTo(-iw, -ih));
  const body = DEPTH - BEVEL * 2;
  const geo = new ExtrudeGeometry(outer, { depth: body, bevelEnabled: true, bevelThickness: BEVEL, bevelSize: BEVEL * 0.9, bevelSegments: 4, curveSegments: 1 });
  geo.translate(0, 0, -body / 2);
  return geo;
}

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
const isLight = () => document.documentElement.dataset.theme === 'light';

export function mountLogo(canvas: HTMLCanvasElement, host: HTMLElement, { onReady, onFail, still = false }: Options): LogoScene {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'low-power', preserveDrawingBuffer: import.meta.env.DEV });
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

  // A soft studio light for the reflections on the bevels: it is what makes the faces read.
  const pmrem = new PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const env = pmrem.fromScene(room, 0.04).texture;
  texs.push(env);
  room.dispose();
  pmrem.dispose();
  scene.environment = env;

  // ExtrudeGeometry draws its caps (front and back) with the first material, its sides and bevels
  // with the second, so the two can be lit differently.
  const inkCap = own(new MeshStandardMaterial({ roughness: 0.42, metalness: 0.1 }));
  const inkSide = own(new MeshStandardMaterial({ roughness: 0.5, metalness: 0.1 }));
  const litCap = own(new MeshStandardMaterial({ roughness: 0.3, metalness: 0 }));
  const litSide = own(new MeshStandardMaterial({ roughness: 0.3, metalness: 0 }));
  const inkHalo = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));
  const halo = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));
  const core = own(new MeshBasicMaterial({ transparent: true, depthWrite: false }));

  let litBar: Group | null = null;
  for (const b of BARS) {
    const bar = new Group();
    bar.position.set(b.x + b.w / 2 - CENTER.x, CENTER.y - (b.y + b.h / 2), b.lit ? REST.lift : 0);
    const geo = frame(b);
    geos.push(geo);
    bar.add(new Mesh(geo, b.lit ? [litCap, litSide] : [inkCap, inkSide]));

    const plane = new PlaneGeometry(b.w + PAD * 2, b.h + PAD * 2);
    geos.push(plane);
    if (b.lit) {
      // Light around the frame, at its back and its middle, so it reads as a volume when turned;
      // and a fill of light inside its opening.
      const line = glow(b, 'line', 2.4, STROKE * 1.8);
      texs.push(line);
      halo.map = line;
      for (const z of [-DEPTH / 2, 0]) {
        const h = new Mesh(plane, halo);
        h.position.z = z;
        h.renderOrder = 1;
        bar.add(h);
      }
      const fill = glow(b, 'fill', 1.2);
      texs.push(fill);
      core.map = fill;
      for (const z of [-DEPTH / 4, DEPTH / 4]) {
        const c = new Mesh(plane, core);
        c.position.z = z;
        c.renderOrder = 2;
        bar.add(c);
      }
      litBar = bar;
    } else {
      // Both ink bars are the same size: one texture serves the two.
      if (!inkHalo.map) {
        inkHalo.map = glow(b, 'line', 1.2, STROKE);
        texs.push(inkHalo.map);
      }
      const h = new Mesh(plane, inkHalo);
      h.position.z = -DEPTH / 2;
      h.renderOrder = 1;
      bar.add(h);
    }
    logo.add(bar);
  }

  const ambient = new AmbientLight(0xffffff, 0.25);
  // Key from the upper left and in front: the tops and left sides catch it, the fronts less so.
  const key = new DirectionalLight(0xffffff, 2.6);
  key.position.set(-10, 14, 9);
  // Rim from behind on the right, so the far edges separate from the ground.
  const rim = new DirectionalLight(0xffffff, 1.4);
  rim.position.set(12, 4, -10);
  // The lit bar's light falls on the inner edges of the other two.
  const spill = new PointLight(0xffffff, 80, 18, 1.6);
  spill.position.set(3, 0, REST.lift * 0.4);
  scene.add(ambient, key, rim, spill);

  function paint() {
    const light = isLight();
    const accent = new Color(css('--accent') || (light ? '#14e6ff' : '#145aff'));
    const markInk = new Color(css('--mark-ink') || (light ? '#1b1c20' : '#e8e8ea'));
    // Dark: pale grey frames; light: near-black ones whose shape shows in their reflections.
    inkCap.color.copy(markInk).multiplyScalar(light ? 1 : 0.62);
    inkSide.color.copy(markInk).multiplyScalar(light ? 1 : 0.5);
    inkCap.envMapIntensity = inkSide.envMapIntensity = light ? 1.1 : 0.4;
    inkCap.roughness = light ? 0.32 : 0.42;
    // The lit bar: its front face glows most, its sides less, so its depth reads at any angle.
    for (const m of [litCap, litSide]) {
      m.color.copy(accent);
      m.emissive.copy(accent);
    }
    litCap.emissiveIntensity = light ? 0.7 : 1.05;
    litSide.emissiveIntensity = light ? 0.32 : 0.5;
    litSide.color.copy(accent).multiplyScalar(light ? 0.62 : 0.8);
    litCap.envMapIntensity = litSide.envMapIntensity = 0.5;
    spill.color.copy(accent);
    spill.intensity = light ? 30 : 90;
    key.intensity = light ? 2.2 : 2.1;
    ambient.intensity = light ? 0.45 : 0.25;
    for (const m of [halo, core, inkHalo]) m.color.copy(m === inkHalo ? markInk : accent);
    // On dark the glow adds light; on white it has to lay colour down instead.
    halo.blending = core.blending = light ? NormalBlending : AdditiveBlending;
    halo.opacity = light ? 0.5 : 0.7;
    core.opacity = light ? 0.3 : 0.45;
    inkHalo.blending = AdditiveBlending;
    inkHalo.opacity = light ? 0 : 0.1;
    inkHalo.visible = !light;
    for (const m of mats) m.needsUpdate = true;
    draw();
  }

  function fit() {
    const w = Math.max(1, host.clientWidth);
    const h = Math.max(1, host.clientHeight);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    const need = camera.aspect >= 1 ? FIT : FIT / camera.aspect;
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

  function apply(p: Pose) {
    logo.rotation.set(p.pitch, p.yaw, 0);
    logo.position.y = p.float;
    if (litBar) litBar.position.z = p.lift;
  }

  function draw() {
    renderer.render(scene, camera);
  }

  // Time only runs while it's drawn, so a hidden tab or a scrolled-off hero picks up where it was.
  let clock = 0;
  let last = 0;
  let raf = 0;
  let onScreen = true;
  let ready = false;
  const loop = (now: number) => {
    raf = 0;
    const dt = last ? Math.min(0.1, (now - last) / 1000) : 0;
    // Above 60 Hz, every other frame is enough: the motion is slow.
    if (ready && dt > 0 && dt < 0.0125) return schedule();
    last = now;
    clock += dt;
    eased += (scroll - eased) * (1 - Math.exp(-dt * 5));
    apply(pose(clock, eased));
    draw();
    if (!ready) {
      ready = true;
      onReady();
    }
    schedule();
  };
  function schedule() {
    if (!still && !raf && onScreen && !document.hidden) raf = requestAnimationFrame(loop);
  }
  function halt() {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    last = 0;
  }

  const io = new IntersectionObserver(([e]) => {
    onScreen = e.isIntersecting;
    if (onScreen) schedule();
    else halt();
  });
  io.observe(host);
  const ro = new ResizeObserver(fit);
  ro.observe(host);
  const mo = new MutationObserver(paint);
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  const onVisible = () => (document.hidden ? halt() : schedule());
  document.addEventListener('visibilitychange', onVisible);
  if (!still) addEventListener('scroll', readScroll, { passive: true });
  const lost = (e: Event) => {
    e.preventDefault();
    halt();
    onFail();
  };
  canvas.addEventListener('webglcontextlost', lost);

  if (!still) {
    readScroll();
    eased = scroll;
  }
  apply(still ? REST : pose(0, eased));
  fit();
  paint();
  if (still) {
    ready = true;
    onReady();
  } else schedule();

  // Development only: the still pose as a PNG, to remake the static images (static/home/).
  if (import.meta.env.DEV) {
    (window as unknown as { __logoSnap?: () => string }).__logoSnap = () => {
      apply(REST);
      draw();
      return canvas.toDataURL('image/png');
    };
  }

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
