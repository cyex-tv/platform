// The hero's 3D logo as real light tubes: each open bar of logo.ts is a clear glass tube bent into
// a rounded rectangle (logo-tube.ts), its two ends turning back into metal electrode caps. The
// middle tube is lit: a bright core inside the glass, whitest in the middle, with a soft halo and a
// wider bloom around it, and its light tints the glass and cores of the other two. Those two are
// off: clear glass with a dim core in the mark's ink colour. The glass is thin: nearly clear face
// on, with edges that thicken and catch the light (fresnel) and the studio's reflections.
//
// No shadow maps and no floor: the glow is drawn by the tubes themselves (shells around the lit
// tube that fade toward their edge), so it reads from every angle at no extra pass. All of it is
// drawn into the same canvas (transparent), so the page shows through.
//
// Motion: it first shows the still pose (the same one as the static image under it) while the lit
// tube strikes once, makes one slow full turn, then sways and floats on its own, and turns further
// and lets the lit tube come forward as the hero scrolls away. It never follows the pointer (cyex-web).
//
// Loaded on its own (dynamic import, after first paint, capable devices only: see LogoHero),
// so three.js never sits in the page's first bundle. It draws only while the hero is on screen and
// the tab is visible, and gives everything back on dispose.
import {
  AdditiveBlending,
  AmbientLight,
  BackSide,
  CapsuleGeometry,
  Color,
  DataTexture,
  DirectionalLight,
  FrontSide,
  Group,
  Mesh,
  MeshPhysicalMaterial,
  MeshStandardMaterial,
  NormalBlending,
  PerspectiveCamera,
  PMREMGenerator,
  PointLight,
  Scene,
  ShaderMaterial,
  Sprite,
  SpriteMaterial,
  SRGBColorSpace,
  TubeGeometry,
  WebGLRenderer,
  type BufferGeometry,
  type Material,
  type Texture,
} from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { BARS, CENTER, GRID } from './logo';
import { REST, pose, strike, type Pose } from './logo-motion';
import { TUBE, ends, halfSize, look, loopPath, segmentsFor, tubePath } from './logo-tube';

export type LogoScene = { dispose(): void };

type Options = {
  /** The first frame is on the canvas: fade it in. */
  onReady: () => void;
  /** WebGL went away (context lost, or it couldn't start): keep the static logo. */
  onFail: () => void;
  /** Draw the still pose once and never move (reduced motion turned on while it ran). */
  still?: boolean;
};

const FOV = 26;
const FIT = GRID * 1.1; // the view holds this many units, so the turned logo and its glow fit
const RADIAL = 18; // round enough for the glass's edge highlight to stay smooth
/** The glow shells around the lit tube: how much wider than the glass, and how fast they fade out. */
const HALO = { scale: 2.3, power: 2.2 };
const BLOOM = { scale: 3.4, power: 1.4 };

// A shell's light is strongest where it faces the camera (over the tube) and fades to nothing at
// its own outline: a soft halo that follows the tube at any angle.
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
// The lit core: the accent at its sides, running toward white in its middle, as a hot tube does.
const coreFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uHot;
  uniform float uLevel;
  varying vec3 vN;
  varying vec3 vV;
  void main() {
    float f = clamp(dot(normalize(vN), normalize(vV)), 0.0, 1.0);
    gl_FragColor = vec4(mix(uColor, vec3(1.0), uHot * f * f) * uLevel, 1.0);
    #include <colorspace_fragment>
  }
`;

/** A soft round falloff (close to a gaussian), white with the light in its alpha: the wide glow. */
function softDot(n = 64): DataTexture {
  const data = new Uint8Array(n * n * 4);
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      const dx = (x + 0.5) / n - 0.5;
      const dy = (y + 0.5) / n - 0.5;
      const i = (y * n + x) * 4;
      data[i] = data[i + 1] = data[i + 2] = 255;
      data[i + 3] = Math.round(255 * Math.max(0, Math.exp(-(dx * dx + dy * dy) * 18) - Math.exp(-4.5)) / (1 - Math.exp(-4.5)));
    }
  const t = new DataTexture(data, n, n);
  t.needsUpdate = true;
  return t;
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
  const stage = new Group();
  const logo = new Group();
  stage.add(logo);
  scene.add(stage);

  const geos: BufferGeometry[] = [];
  const mats: Material[] = [];
  const texs: Texture[] = [];
  const own = <T extends Material>(m: T) => (mats.push(m), m);
  const geo = <T extends BufferGeometry>(g: T) => (geos.push(g), g);

  // A soft studio for the glass to reflect: it is what makes it read as glass.
  const pmrem = new PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const env = pmrem.fromScene(room, 0.04).texture;
  texs.push(env);
  room.dispose();
  pmrem.dispose();
  scene.environment = env;

  // Thin glass: nearly clear face on; toward its outline it grows opaque and catches light (or,
  // on white, darkens), as the light crosses more glass there.
  const edge = { uEdge: { value: new Color() }, uEdgeAlpha: { value: 0 }, uEdgePower: { value: 2 } };
  const makeGlass = (side: typeof FrontSide | typeof BackSide) => {
    const m = own(new MeshPhysicalMaterial({ transparent: true, depthWrite: false, side, roughness: 0.06, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.04, ior: 1.5, specularIntensity: 1 }));
    m.onBeforeCompile = (sh) => {
      Object.assign(sh.uniforms, edge);
      sh.fragmentShader = sh.fragmentShader
        .replace('void main() {', 'uniform vec3 uEdge; uniform float uEdgeAlpha; uniform float uEdgePower;\nvoid main() {')
        .replace(
          '#include <opaque_fragment>',
          `float fres = pow(1.0 - clamp(dot(normalize(vViewPosition), normal), 0.0, 1.0), uEdgePower);
           outgoingLight = outgoingLight + uEdge * fres;
           diffuseColor.a = mix(diffuseColor.a, uEdgeAlpha, fres);
           #include <opaque_fragment>`,
        );
    };
    return m;
  };
  // The far wall of each tube first, then what's inside, then the near wall.
  const glassBack = makeGlass(BackSide);
  const glassFront = makeGlass(FrontSide);
  // The lit tube's glass carries some of its light.
  const litBack = makeGlass(BackSide);
  const litFront = makeGlass(FrontSide);

  const shellUniforms = () => ({ uColor: { value: new Color() }, uOpacity: { value: 0 }, uPower: { value: 2 } });
  const shell = (power: number) => {
    const m = own(new ShaderMaterial({ vertexShader: shellVertex, fragmentShader: glowFragment, uniforms: shellUniforms(), transparent: true, depthWrite: false }));
    m.uniforms.uPower.value = power;
    return m;
  };
  const halo = shell(HALO.power);
  const bloom = shell(BLOOM.power);
  const wide = own(new SpriteMaterial({ transparent: true, depthWrite: false }));
  const core = own(new ShaderMaterial({ vertexShader: shellVertex, fragmentShader: coreFragment, uniforms: { uColor: { value: new Color() }, uHot: { value: 0 }, uLevel: { value: 1 } } }));
  const inkCore = own(new MeshStandardMaterial({ roughness: 0.75, metalness: 0 }));
  const electrode = own(new MeshStandardMaterial({ roughness: 0.45, metalness: 0.85 }));

  const capGeo = geo(new CapsuleGeometry(TUBE.cap, TUBE.capLength, 4, 14));
  capGeo.rotateX(Math.PI / 2);

  let litBar: Group | null = null;
  for (const b of BARS) {
    const bar = new Group();
    bar.position.set(b.x + b.w / 2 - CENTER.x, CENTER.y - (b.y + b.h / 2), b.lit ? REST.lift : 0);
    const path = tubePath(b);
    const n = segmentsFor(path);
    const tube = (radius: number, radial: number) => geo(new TubeGeometry(path, n, radius, radial, false));

    const glass = tube(TUBE.radius, RADIAL);
    const back = new Mesh(glass, b.lit ? litBack : glassBack);
    back.renderOrder = 1;
    const inner = new Mesh(tube(b.lit ? TUBE.core : TUBE.coreOff, 10), b.lit ? core : inkCore);
    const front = new Mesh(glass, b.lit ? litFront : glassFront);
    front.renderOrder = 3;
    bar.add(back, inner, front);

    // The electrodes, behind the bottom edge where the ends turn back.
    const { hh } = halfSize(b);
    for (const x of Object.values(ends(b))) {
      const cap = new Mesh(capGeo, electrode);
      cap.position.set(x, -hh, -TUBE.back - TUBE.capLength / 2);
      bar.add(cap);
    }

    if (b.lit) {
      const loop = loopPath(b);
      for (const [m, scale, order] of [[halo, HALO.scale, 4], [bloom, BLOOM.scale, 5]] as const) {
        const s = new Mesh(geo(new TubeGeometry(loop, segmentsFor(loop), TUBE.radius * scale, 12, true)), m);
        s.renderOrder = order;
        bar.add(s);
      }
      // The wide, faint light the tube throws into the air around it, always toward the camera.
      wide.map = softDot();
      texs.push(wide.map);
      const air = new Sprite(wide);
      air.scale.set(b.w * 1.7, b.h * 3.2, 1);
      air.position.z = -TUBE.radius;
      air.renderOrder = 0;
      bar.add(air);
      litBar = bar;
    }
    logo.add(bar);
  }

  const ambient = new AmbientLight(0xffffff, 0.3);
  // Key from the upper left and in front, rim from behind on the right: the glass's highlights.
  const key = new DirectionalLight(0xffffff, 1.6);
  key.position.set(-12, 31, 14);
  const rim = new DirectionalLight(0xffffff, 1.4);
  rim.position.set(12, 4, -10);
  // The lit tube's light, falling on the glass and cores of the other two. It rides with the lit
  // tube as it comes forward.
  const spill = new PointLight(0xffffff, 60, 20, 1.4);
  spill.position.set(-1.2, 0, 0);
  litBar?.add(spill);
  scene.add(ambient, key, rim);

  let level = 1; // the strike: how lit the lit tube is right now
  let spillBase = 0;
  let haloBase = 0;
  let bloomBase = 0;
  let wideBase = 0;
  let coreBase = 1;

  function paint() {
    const light = isLight();
    const L = look(light);
    const accent = new Color(css('--accent') || (light ? '#14e6ff' : '#145aff'));
    const markInk = new Color(css('--mark-ink') || (light ? '#1b1c20' : '#e8e8ea'));

    for (const m of [glassBack, glassFront, litBack, litFront]) {
      // On white the edges have to darken to show: a dark tint does that through normal blending.
      m.color.set(L.edgeDark ? 0x363944 : 0xffffff);
      m.opacity = L.glassOpacity;
      m.envMapIntensity = L.reflect;
      m.emissive.set(0x000000);
    }
    // The lit tube's glass glows faintly from the core inside it.
    for (const m of [litBack, litFront]) m.emissive.copy(accent).multiplyScalar(light ? 0.18 : 0.35);
    edge.uEdge.value.set(light ? 0x000000 : 0xffffff).multiplyScalar(L.edgeLight);
    edge.uEdgeAlpha.value = L.edgeAlpha;
    edge.uEdgePower.value = L.edgePower;

    core.uniforms.uColor.value.copy(accent);
    core.uniforms.uHot.value = L.coreHot;
    coreBase = L.coreLevel;
    for (const m of [halo, bloom]) {
      m.uniforms.uColor.value.copy(accent);
      m.blending = L.additive ? AdditiveBlending : NormalBlending;
    }
    wide.color.copy(accent);
    wide.blending = L.additive ? AdditiveBlending : NormalBlending;
    wideBase = L.air;
    haloBase = L.glow;
    bloomBase = L.bloom;

    inkCore.color.copy(markInk).multiplyScalar(L.inkCore);
    inkCore.emissive.copy(markInk).multiplyScalar(L.inkGlow);
    electrode.color.set(light ? 0x4a4e58 : 0x5a5f6a);
    electrode.envMapIntensity = light ? 1.2 : 0.7;

    spill.color.copy(accent);
    spillBase = L.spill;
    ambient.intensity = L.ambient;
    key.intensity = L.key;
    for (const m of mats) m.needsUpdate = true;
    setLevel(level);
    draw();
  }

  /** Set how lit the lit tube is (the strike), 0-1. */
  function setLevel(k: number) {
    level = k;
    core.uniforms.uLevel.value = coreBase * (0.35 + 0.65 * k);
    halo.uniforms.uOpacity.value = haloBase * k;
    bloom.uniforms.uOpacity.value = bloomBase * k * k;
    wide.opacity = wideBase * k * k;
    spill.intensity = spillBase * k;
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
