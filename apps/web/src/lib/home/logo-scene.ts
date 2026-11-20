// The hero's 3D logo: the three open bars of logo.ts as deep, bevelled frames, seen from above
// and to the side so their tops and sides show, made of neon. The middle bar stands well in
// front and glows: it gives light rather than taking it, in the accent, running white along the
// middle of its wall (a hot core, on every face), with a soft halo and a wide, faint glow around
// it; its light spills onto the other two. Those two are the same bars, off: the mark's ink,
// lit by the room and tinted by the lit bar.
//
// No shadow maps and no floor: the glow is drawn by the bar itself (shells along its wall that
// fade toward their outline, and a soft sprite), so it reads from every angle with no extra pass.
// All of it is drawn into the same canvas (transparent), so the page shows through.
//
// Motion: it first shows the still pose (the same one as the static image under it) while the lit
// bar strikes once, makes one slow full turn, then sways and floats on its own, and turns further
// and lets the lit bar come forward as the hero scrolls away. It never follows the pointer (cyex-web).
//
// Loaded on its own (dynamic import, after first paint, capable devices only: see LogoHero),
// so three.js never sits in the page's first bundle. It draws only while the hero is on screen and
// the tab is visible, and gives everything back on dispose.
import {
  AdditiveBlending,
  AmbientLight,
  Color,
  DataTexture,
  DirectionalLight,
  ExtrudeGeometry,
  Group,
  Mesh,
  MeshStandardMaterial,
  NormalBlending,
  Path,
  PerspectiveCamera,
  PMREMGenerator,
  PointLight,
  Scene,
  ShaderMaterial,
  Shape,
  Sprite,
  SpriteMaterial,
  SRGBColorSpace,
  TubeGeometry,
  Vector2,
  WebGLRenderer,
  type BufferGeometry,
  type Material,
  type Texture,
} from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { BARS, CENTER, GRID, STROKE, type Bar } from './logo';
import { look, loopPath, halfSize, segmentsFor, shellLimit } from './logo-glow';
import { REST, pose, strike, type Pose } from './logo-motion';

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

/** A soft round falloff (close to a gaussian), white with the light in its alpha: the wide glow. */
function softDot(n = 64): DataTexture {
  const data = new Uint8Array(n * n * 4);
  const floor = Math.exp(-4.5);
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      const dx = (x + 0.5) / n - 0.5;
      const dy = (y + 0.5) / n - 0.5;
      const i = (y * n + x) * 4;
      data[i] = data[i + 1] = data[i + 2] = 255;
      data[i + 3] = Math.round((255 * Math.max(0, Math.exp(-(dx * dx + dy * dy) * 18) - floor)) / (1 - floor));
    }
  const t = new DataTexture(data, n, n);
  t.needsUpdate = true;
  return t;
}

// The lit rod: its light is the accent at its edges, running white along the middle of its wall.
// `vLocal` is the bar's own space: on its front and back the core is where the wall's middle line
// is (across the wall); on its sides, top and bottom, where its depth's middle is.
const rodVertex = /* glsl */ `
  varying vec3 vLocal;
  varying vec3 vN;
  varying vec3 vV;
  void main() {
    vLocal = position;
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    vN = normalMatrix * normal;
    vV = -mv.xyz;
    gl_Position = projectionMatrix * mv;
  }
`;
const rodFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform vec2 uHalf;   // half size of the wall's middle line
  uniform float uWall;  // half the wall's width
  uniform float uDepth; // half the bar's depth
  uniform float uHot;
  uniform float uCore;
  uniform float uLevel;
  varying vec3 vLocal;
  varying vec3 vN;
  varying vec3 vV;
  void main() {
    vec2 q = abs(vLocal.xy) - uHalf;
    float across = abs(length(max(q, 0.0)) + min(max(q.x, q.y), 0.0)) / uWall;
    float deep = abs(vLocal.z) / uDepth;
    float core = 1.0 - smoothstep(0.0, uCore, min(across, deep));
    float facing = clamp(dot(normalize(vN), normalize(vV)), 0.0, 1.0);
    vec3 c = mix(uColor, vec3(1.0), uHot * core * (0.55 + 0.45 * facing));
    // Faces turned away are a little deeper in colour, so the rod's shape still reads.
    c *= mix(0.72, 1.0, facing);
    gl_FragColor = vec4(c * uLevel, 1.0);
    #include <colorspace_fragment>
  }
`;
// A glow shell is strongest where it faces the camera (over the rod) and fades to nothing at its
// own outline: a soft halo that follows the bar at any angle.
const shellVertex = /* glsl */ `
  varying vec3 vN;
  varying vec3 vV;
  void main() {
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    vN = normalMatrix * normal;
    vV = -mv.xyz;
    gl_Position = projectionMatrix * mv;
  }
`;
const glowFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uOpacity;
  uniform float uPower;
  varying vec3 vN;
  varying vec3 vV;
  void main() {
    float f = clamp(dot(normalize(vN), normalize(vV)), 0.0, 1.0);
    gl_FragColor = vec4(uColor, pow(f, uPower) * uOpacity);
    #include <colorspace_fragment>
  }
`;

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
const isLight = () => document.documentElement.dataset.theme === 'light';

export function mountLogo(canvas: HTMLCanvasElement, host: HTMLElement, { onReady, onFail, still = false }: Options): LogoScene {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'low-power', preserveDrawingBuffer: import.meta.env.DEV });
  renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.setClearColor(0x000000, 0);

  const scene = new Scene();
  const camera = new PerspectiveCamera(FOV, 1, 1, 400);
  // The stage is tilted like the logo (its pitch); the mark turns inside it.
  const stage = new Group();
  const logo = new Group();
  stage.add(logo);
  scene.add(stage);

  const geos: BufferGeometry[] = [];
  const mats: Material[] = [];
  const texs: Texture[] = [];
  const own = <T extends Material>(m: T) => (mats.push(m), m);
  const geo = <T extends BufferGeometry>(g: T) => (geos.push(g), g);

  // A soft studio light for the reflections on the unlit bars' bevels: it is what makes them read.
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
  const rod = own(
    new ShaderMaterial({
      vertexShader: rodVertex,
      fragmentShader: rodFragment,
      uniforms: {
        uColor: { value: new Color() },
        uHalf: { value: new Vector2() },
        uWall: { value: STROKE / 2 },
        uDepth: { value: DEPTH / 2 },
        uHot: { value: 0 },
        uCore: { value: 0.5 },
        uLevel: { value: 1 },
      },
    }),
  );
  const shell = (power: number) =>
    own(new ShaderMaterial({ vertexShader: shellVertex, fragmentShader: glowFragment, uniforms: { uColor: { value: new Color() }, uOpacity: { value: 0 }, uPower: { value: power } }, transparent: true, depthWrite: false }));
  const halo = shell(1.6);
  const air = own(new SpriteMaterial({ transparent: true, depthWrite: false }));

  let litBar: Group | null = null;
  for (const b of BARS) {
    const bar = new Group();
    bar.position.set(b.x + b.w / 2 - CENTER.x, CENTER.y - (b.y + b.h / 2), b.lit ? REST.lift : 0);
    const body = new Mesh(geo(frame(b)), b.lit ? rod : [inkCap, inkSide]);
    bar.add(body);

    if (b.lit) {
      const { hw, hh } = halfSize(b);
      rod.uniforms.uHalf.value.set(hw, hh);
      // The halo: a shell along the wall's middle, as wide as its corners allow.
      const loop = loopPath(b);
      const h = new Mesh(geo(new TubeGeometry(loop, segmentsFor(loop), shellLimit(b) * 0.98, 14, true)), halo);
      h.renderOrder = 2;
      bar.add(h);
      // The wide, faint light it throws into the air around it, always toward the camera.
      air.map = softDot();
      texs.push(air.map);
      const a = new Sprite(air);
      a.scale.set(b.w * 1.8, b.h * 3.4, 1);
      a.position.z = -DEPTH / 2;
      a.renderOrder = 1;
      bar.add(a);
      litBar = bar;
    }
    logo.add(bar);
  }

  const ambient = new AmbientLight(0xffffff, 0.25);
  // Key from the upper left and in front: the unlit bars' tops and left sides catch it.
  const key = new DirectionalLight(0xffffff, 1.3);
  key.position.set(-12, 31, 14);
  // Rim from behind on the right, so their far edges separate.
  const rim = new DirectionalLight(0xffffff, 1.4);
  rim.position.set(12, 4, -10);
  // The lit bar's light falls on the inner edges of the other two; it rides with the lit bar.
  const spill = new PointLight(0xffffff, 80, 20, 1.5);
  spill.position.set(-2, 0, -DEPTH * 0.6);
  litBar?.add(spill);
  scene.add(ambient, key, rim);

  let level = 1; // the strike: how lit the lit bar is right now
  let base = { level: 1, glow: 0, air: 0, spill: 0 };

  function paint() {
    const light = isLight();
    const L = look(light);
    const accent = new Color(css('--accent') || (light ? '#14e6ff' : '#145aff'));
    const markInk = new Color(css('--mark-ink') || (light ? '#1b1c20' : '#e8e8ea'));
    // Dark: pale grey frames; light: near-black ones whose shape shows in their reflections.
    inkCap.color.copy(markInk).multiplyScalar(light ? 1 : 0.62);
    inkSide.color.copy(markInk).multiplyScalar(light ? 1 : 0.5);
    for (const m of [inkCap, inkSide]) m.emissive.copy(markInk).multiplyScalar(L.inkGlow);
    // The room's reflections (three applies the scene's strength to every lit material): soft on
    // dark so the unlit bars stay grey beside the lit one, full on white where they shape the bars.
    scene.environmentIntensity = light ? 1.1 : 0.45;
    inkCap.roughness = light ? 0.32 : 0.42;
    rod.uniforms.uColor.value.copy(accent);
    rod.uniforms.uHot.value = L.hot;
    rod.uniforms.uCore.value = L.coreWidth;
    halo.uniforms.uColor.value.copy(accent);
    air.color.copy(accent);
    // On dark the glow adds light; on white it has to lay colour down instead.
    halo.blending = air.blending = L.additive ? AdditiveBlending : NormalBlending;
    spill.color.copy(accent);
    key.intensity = light ? 2.2 : 1.3;
    ambient.intensity = light ? 0.45 : 0.18;
    base = { level: L.level, glow: L.glow, air: L.air, spill: L.spill };
    for (const m of mats) m.needsUpdate = true;
    setLevel(level);
    draw();
  }

  /** How lit the lit bar is (the strike), 0-1. */
  function setLevel(k: number) {
    level = k;
    rod.uniforms.uLevel.value = base.level * (0.4 + 0.6 * k);
    halo.uniforms.uOpacity.value = base.glow * k;
    air.opacity = base.air * k * k;
    spill.intensity = base.spill * k;
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
  // The scroll handler only marks it stale; the layout is read once per frame in the loop, so a
  // burst of scroll events never forces layout between style writes.
  let stale = false;
  const measureScroll = () => {
    stale = false;
    const r = host.getBoundingClientRect();
    scroll = Math.min(1, Math.max(0, -r.top / Math.max(1, r.height)));
  };
  const readScroll = () => {
    stale = true;
  };

  function apply(p: Pose) {
    stage.rotation.x = p.pitch;
    logo.rotation.y = p.yaw;
    logo.position.y = p.float;
    if (litBar) litBar.position.z = p.lift;
  }

  const perf = import.meta.env.DEV && location.search.includes('logoperf');
  function draw() {
    if (!perf) return renderer.render(scene, camera);
    // Development only: how long one draw takes on the GPU too (waits for it), kept on window.
    const t = performance.now();
    renderer.render(scene, camera);
    renderer.getContext().finish();
    ((window as unknown as { __logoDraw?: number[] }).__logoDraw ??= []).push(performance.now() - t);
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
    if (stale) measureScroll();
    eased += (scroll - eased) * (1 - Math.exp(-dt * 5));
    apply(pose(clock, eased));
    setLevel(strike(clock));
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
    measureScroll();
    eased = scroll;
  }
  apply(still ? REST : pose(0, eased));
  fit();
  paint();
  if (still) {
    ready = true;
    onReady();
  } else schedule();

  // Development only: the mean time of `n` full draws, waiting for the GPU (frame cost, not the display's rate).
  if (import.meta.env.DEV) {
    (window as unknown as { __logoBench?: (n: number) => number }).__logoBench = (n) => {
      const gl = renderer.getContext();
      const px = new Uint8Array(4);
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      const t = performance.now();
      for (let i = 0; i < n; i++) {
        apply(pose(i * 0.016 + 6, 0));
        renderer.render(scene, camera);
      }
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      return (performance.now() - t) / n;
    };
  }

  // Development only: the still pose, fully lit, as a PNG, to remake the static images (static/home/).
  if (import.meta.env.DEV) {
    (window as unknown as { __logoSnap?: () => string }).__logoSnap = () => {
      apply(REST);
      setLevel(1);
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
