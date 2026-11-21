// The hero's 3D logo: the three open bars of logo.ts as deep frames with rounded edges, seen from
// above and to the side so their tops and sides show, through a lens with some perspective so the
// depth reads even at rest.
//
// The middle bar is a neon rod: a core that gives light (accent at its edges, running white along
// the middle of its wall) inside a clear sleeve that catches the room's reflections, with a real
// bloom around it and a faint haze. Its light falls off with distance (inverse square) onto the
// other two, in its colour. Those two are the unlit bars, a dense polymer with a clear coat: a
// fine grain made in code varies their roughness and normals, they reflect a studio room, and
// they darken where light reaches them less (inside their openings: logo-ao.ts). Key, fill and
// rim lights separate their faces. Colour goes through AgX tone mapping, except the rod's light,
// which keeps the accent exact.
//
// No shadow maps and no floor. The bloom is drawn from a separate render of the rod alone (the
// other bars black, so they hide it where they're in front), blurred and laid over the frame: added
// as light on dark, laid down as colour on white, so the canvas stays transparent and the page
// shows through. Low-power devices, and any device whose frames run long, step down to a glow drawn
// by the rod itself instead (logo-quality.ts).
//
// Motion: it first shows the still pose (the same one as the static image under it) while the rod
// strikes once, makes one slow full turn, then sways and floats on its own, and turns further and
// lets the rod come forward as the hero scrolls away. It never follows the pointer (cyex-web).
//
// Loaded on its own (dynamic import, after first paint, capable devices only: see LogoHero),
// so three.js never sits in the page's first bundle. It draws only while the hero is on screen and
// the tab is visible, and gives everything back on dispose.
import {
  AgXToneMapping,
  AmbientLight,
  BackSide,
  Color,
  CustomBlending,
  ZeroFactor,
  SrcAlphaFactor,
  DataTexture,
  DirectionalLight,
  ExtrudeGeometry,
  FrontSide,
  Group,
  HalfFloatType,
  Mesh,
  MeshBasicMaterial,
  MeshPhysicalMaterial,
  NormalBlending,
  OneFactor,
  OneMinusSrcAlphaFactor,
  OrthographicCamera,
  Path,
  PerspectiveCamera,
  PlaneGeometry,
  PMREMGenerator,
  PointLight,
  RepeatWrapping,
  Scene,
  ShaderMaterial,
  Shape,
  Sprite,
  SpriteMaterial,
  SRGBColorSpace,
  TubeGeometry,
  Vector2,
  WebGLRenderer,
  WebGLRenderTarget,
  type BufferGeometry,
  type Material,
  type Texture,
} from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { BARS, CENTER, GRID, STROKE, type Bar } from './logo';
import { cavityGlsl } from './logo-ao';
import { look, loopPath, halfSize, segmentsFor, shellLimit } from './logo-glow';
import { REST, pose, strike, type Pose } from './logo-motion';
import { frameBudget, startQuality, type Quality } from './logo-quality';
import { grain, normalBytes, roughnessBytes } from './logo-surface';

export type LogoScene = { dispose(): void };

type Options = {
  /** The first frame is on the canvas: fade it in. */
  onReady: () => void;
  /** WebGL went away (context lost, or it couldn't start): keep the static logo. */
  onFail: () => void;
  /** Draw the still pose once and never move (reduced motion turned on while it ran). */
  still?: boolean;
};

export const DEPTH = 4.2; // how deep each frame is, in grid units (a bar is 6 tall)
const BEVEL = 0.46; // the rounded edge: big and finely divided, so it catches a line of light
const BEVEL_STEPS = 8;
const CORE_INSET = 0.2; // the rod's core sits this far inside its clear sleeve
const FOV = 36;
const FIT = GRID * 1.14; // the view holds this many units, so the turned logo and its glow fit
const GRAIN = 128; // texture size of the bars' grain, and how many grid units one tile covers
const GRAIN_TILE = 5;

/** One open bar: a rectangle with a rectangular hole, extruded with a rounded bevel, centred on its own middle; `inset` shrinks it all round. */
function frame(b: Bar, inset = 0): ExtrudeGeometry {
  // The bevel grows the outline, so the shape is drawn that much smaller to keep the footprint.
  const bevel = BEVEL - inset * 0.5;
  const w = b.w / 2 - inset - bevel;
  const h = b.h / 2 - inset - bevel;
  const iw = b.w / 2 - STROKE + inset + bevel;
  const ih = b.h / 2 - STROKE + inset + bevel;
  const outer = new Shape().moveTo(-w, -h).lineTo(w, -h).lineTo(w, h).lineTo(-w, h).lineTo(-w, -h);
  outer.holes.push(new Path().moveTo(-iw, -ih).lineTo(-iw, ih).lineTo(iw, ih).lineTo(iw, -ih).lineTo(-iw, -ih));
  const body = DEPTH - inset * 2 - bevel * 2;
  const geo = new ExtrudeGeometry(outer, { depth: body, bevelEnabled: true, bevelThickness: bevel, bevelSize: bevel * 0.92, bevelSegments: BEVEL_STEPS, curveSegments: 1 });
  geo.translate(0, 0, -body / 2);
  return geo;
}

/** A soft round falloff (close to a gaussian), white with the light in its alpha: the haze. */
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

function tiled(bytes: Uint8Array, n: number): DataTexture {
  const t = new DataTexture(bytes, n, n);
  t.wrapS = t.wrapT = RepeatWrapping;
  t.repeat.set(1 / GRAIN_TILE, 1 / GRAIN_TILE);
  t.generateMipmaps = true;
  t.needsUpdate = true;
  return t;
}

// The rod's core: its light is the accent at its edges, running white along the middle of its
// wall. `vLocal` is the bar's own space: on its front and back the core is where the wall's middle
// line is (across the wall); on its sides, top and bottom, where its depth's middle is.
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
// A glow shell (the light step's stand-in for bloom) is strongest where it faces the camera and
// fades to nothing at its own outline.
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
// The bloom laid over the frame (premultiplied alpha): on dark it adds light and leaves the
// canvas's alpha alone; on white it lays the glow's colour down with an alpha from its strength.
const compositeVertex = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;
const compositeFragment = /* glsl */ `
  uniform sampler2D tBloom;
  uniform float uAdd;
  uniform float uGain;
  uniform float uKeep; // added light also written to alpha (for a still image, which can't hold alpha-free light)
  varying vec2 vUv;
  void main() {
    // Fades out toward the canvas's edges, so the glow never shows the canvas's square.
    vec2 e = min(vUv, 1.0 - vUv);
    vec3 b = texture2D(tBloom, vUv).rgb * smoothstep(0.0, 0.2, min(e.x, e.y));
    float m = max(max(b.r, b.g), max(b.b, 1e-4));
    // The faintest tail is dropped on white, where it would show as a pale veil to the canvas's edge.
    float a = smoothstep(0.02, 0.5, m * uGain) * 0.6 * (1.0 - uAdd);
    // Added as it is (not encoded): encoding a faint glow on its own would lift it far too much.
    vec3 added = b * 1.4;
    vec3 laid = sRGBTransferOETF(vec4(b / m, 1.0)).rgb * a;
    float kept = clamp(m * 1.2, 0.0, 0.45) * uKeep * uAdd;
    gl_FragColor = vec4(mix(laid, added, uAdd), a + kept);
  }
`;

/**
 * On dark, a glow adds light and leaves the canvas's alpha alone (three's additive mode would raise
 * the alpha too, and the page behind would darken there); on white it is laid down normally.
 */
function addLight(m: Material, additive: boolean) {
  if (!additive) {
    m.blending = NormalBlending;
    return;
  }
  m.blending = CustomBlending;
  m.blendSrc = SrcAlphaFactor;
  m.blendDst = OneFactor;
  m.blendSrcAlpha = ZeroFactor;
  m.blendDstAlpha = OneFactor;
}

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
const isLight = () => document.documentElement.dataset.theme === 'light';

type Nav = Navigator & { deviceMemory?: number };

export function mountLogo(canvas: HTMLCanvasElement, host: HTMLElement, { onReady, onFail, still = false }: Options): LogoScene {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'low-power', preserveDrawingBuffer: import.meta.env.DEV });
  const ratio = Math.min(devicePixelRatio || 1, 2);
  renderer.setPixelRatio(ratio);
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.toneMapping = AgXToneMapping;
  renderer.setClearColor(0x000000, 0);

  const nav = navigator as Nav;
  let quality: Quality = startQuality({
    memory: typeof nav.deviceMemory === 'number' ? nav.deviceMemory : undefined,
    cores: typeof nav.hardwareConcurrency === 'number' && nav.hardwareConcurrency > 0 ? nav.hardwareConcurrency : undefined,
    forceLow: location.search.includes('logoquality=low'),
  });
  const budget = frameBudget();

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
  const tex = <T extends Texture>(t: T) => (texs.push(t), t);

  // A studio room to reflect: what the clear coat and the sleeve show.
  const pmrem = new PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const env = tex(pmrem.fromScene(room, 0.04).texture);
  room.dispose();
  pmrem.dispose();
  scene.environment = env;

  // The unlit bars: a dense polymer with a clear coat, a fine grain in its roughness and normals,
  // darker where the room's light reaches less. ExtrudeGeometry draws its caps (front and back)
  // with the first material and its sides and bevels with the second.
  const height = grain(GRAIN);
  const grainNormal = tex(tiled(normalBytes(height, GRAIN), GRAIN));
  const grainRough = tex(tiled(roughnessBytes(height, GRAIN, 1, 0.18), GRAIN));
  const inkBar = BARS.find((b) => !b.lit)!;
  const cavityUniforms = { uHole: { value: new Vector2(inkBar.w / 2 - STROKE, inkBar.h / 2 - STROKE) }, uHalfDepth: { value: DEPTH / 2 } };
  const makeInk = () => {
    const m = own(
      new MeshPhysicalMaterial({
        metalness: 0,
        roughness: 0.5,
        roughnessMap: grainRough,
        normalMap: grainNormal,
        normalScale: new Vector2(0.18, 0.18),
        clearcoat: 0.6,
        clearcoatRoughness: 0.14,
        specularIntensity: 0.5,
      }),
    );
    m.onBeforeCompile = (sh) => {
      Object.assign(sh.uniforms, cavityUniforms);
      sh.vertexShader = sh.vertexShader.replace('void main() {', 'varying vec3 vHeroLocal;\nvoid main() {').replace('#include <begin_vertex>', '#include <begin_vertex>\nvHeroLocal = position;');
      sh.fragmentShader = sh.fragmentShader.replace('void main() {', `varying vec3 vHeroLocal;\n${cavityGlsl}\nvoid main() {`).replace(
        '#include <aomap_fragment>',
        `float heroAO = heroCavity(vHeroLocal);
         reflectedLight.indirectDiffuse *= heroAO;
         reflectedLight.indirectSpecular *= heroAO;
         reflectedLight.directDiffuse *= mix(1.0, heroAO, 0.6);
         reflectedLight.directSpecular *= mix(1.0, heroAO, 0.6);
         #if defined( USE_CLEARCOAT )
           clearcoatSpecularIndirect *= heroAO;
         #endif
         #include <aomap_fragment>`,
      );
    };
    return m;
  };
  const inkCap = makeInk();
  const inkSide = makeInk();
  // In the bloom's own render the unlit bars are black: they hide the rod's light where they're in front.
  const occluder = own(new MeshBasicMaterial({ color: 0x000000 }));

  // The rod's clear sleeve: thin, nearly clear face on; its edges grow opaque and catch light.
  const edge = { uEdge: { value: new Color() }, uEdgeAlpha: { value: 0 } };
  const makeSleeve = (side: typeof FrontSide | typeof BackSide) => {
    // Not tone mapped: AgX would pull the accent in the sleeve toward cyan.
    const m = own(new MeshPhysicalMaterial({ transparent: true, depthWrite: false, toneMapped: false, side, roughness: 0.05, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.03, ior: 1.5, specularIntensity: 1 }));
    m.onBeforeCompile = (sh) => {
      Object.assign(sh.uniforms, edge);
      sh.fragmentShader = sh.fragmentShader.replace('void main() {', 'uniform vec3 uEdge; uniform float uEdgeAlpha;\nvoid main() {').replace(
        '#include <opaque_fragment>',
        `float fres = pow(1.0 - clamp(dot(normalize(vViewPosition), normal), 0.0, 1.0), 2.5);
         outgoingLight = outgoingLight + uEdge * fres;
         diffuseColor.a = mix(diffuseColor.a, uEdgeAlpha, fres);
         #include <opaque_fragment>`,
      );
    };
    return m;
  };
  const sleeveBack = makeSleeve(BackSide);
  const sleeveFront = makeSleeve(FrontSide);

  const rod = own(
    new ShaderMaterial({
      vertexShader: rodVertex,
      fragmentShader: rodFragment,
      uniforms: {
        uColor: { value: new Color() },
        uHalf: { value: new Vector2() },
        uWall: { value: STROKE / 2 - CORE_INSET },
        uDepth: { value: DEPTH / 2 - CORE_INSET },
        uHot: { value: 0 },
        uCore: { value: 0.5 },
        uLevel: { value: 1 },
      },
    }),
  );
  const halo = own(new ShaderMaterial({ vertexShader: shellVertex, fragmentShader: glowFragment, uniforms: { uColor: { value: new Color() }, uOpacity: { value: 0 }, uPower: { value: 1.6 } }, transparent: true, depthWrite: false }));
  const haze = own(new SpriteMaterial({ transparent: true, depthWrite: false }));

  const inkMeshes: Mesh[] = [];
  const glassMeshes: Mesh[] = [];
  let litBar: Group | null = null;
  let haloMesh: Mesh | null = null;
  let hazeSprite: Sprite | null = null;
  const lamps: PointLight[] = [];
  for (const b of BARS) {
    const bar = new Group();
    bar.position.set(b.x + b.w / 2 - CENTER.x, CENTER.y - (b.y + b.h / 2), b.lit ? REST.lift : 0);
    if (!b.lit) {
      const body = new Mesh(geo(frame(b)), [inkCap, inkSide]);
      inkMeshes.push(body);
      bar.add(body);
    } else {
      const { hw, hh } = halfSize(b);
      rod.uniforms.uHalf.value.set(hw, hh);
      bar.add(new Mesh(geo(frame(b, CORE_INSET)), rod));
      const sleeve = geo(frame(b));
      const back = new Mesh(sleeve, sleeveBack);
      back.renderOrder = 1;
      const front = new Mesh(sleeve, sleeveFront);
      front.renderOrder = 3;
      glassMeshes.push(back, front);
      bar.add(back, front);
      // The light step's glow: a shell along the wall's middle, as wide as its corners allow.
      const loop = loopPath(b);
      haloMesh = new Mesh(geo(new TubeGeometry(loop, segmentsFor(loop), shellLimit(b) * 0.98, 14, true)), halo);
      haloMesh.renderOrder = 4;
      bar.add(haloMesh);
      // A faint haze in the air around it, always toward the camera.
      haze.map = tex(softDot());
      hazeSprite = new Sprite(haze);
      hazeSprite.scale.set(b.w * 1.8, b.h * 3.4, 1);
      hazeSprite.position.z = -DEPTH / 2;
      hazeSprite.renderOrder = 0;
      bar.add(hazeSprite);
      // Its light, from along its length, falling off with the square of the distance.
      for (const x of [-hw * 0.55, hw * 0.55]) {
        const lamp = new PointLight(0xffffff, 0, 0, 2);
        lamp.position.set(x, 0, 0);
        lamps.push(lamp);
        bar.add(lamp);
      }
      litBar = bar;
    }
    logo.add(bar);
  }

  // Key from the upper left and in front, a soft fill from the lower right, a rim from behind:
  // the faces separate at every angle.
  const ambient = new AmbientLight(0xffffff, 0.05);
  const key = new DirectionalLight(0xfff6ec, 1.6);
  key.position.set(-14, 26, 18);
  const fill = new DirectionalLight(0xe8eeff, 0.35);
  fill.position.set(16, -10, 12);
  const rim = new DirectionalLight(0xffffff, 1.5);
  rim.position.set(12, 6, -14);
  scene.add(ambient, key, fill, rim);

  // The bloom: the rod alone, rendered small, blurred into mips, laid over the frame.
  const bloomTarget = new WebGLRenderTarget(1, 1, { type: HalfFloatType });
  const bloom = new UnrealBloomPass(new Vector2(1, 1), 0.5, 0, 0.08);
  const composite = own(
    new ShaderMaterial({
      vertexShader: compositeVertex,
      fragmentShader: compositeFragment,
      uniforms: { tBloom: { value: bloom.renderTargetsHorizontal[0].texture }, uAdd: { value: 1 }, uGain: { value: 1.5 }, uKeep: { value: 0 } },
      transparent: true,
      depthTest: false,
      depthWrite: false,
      blending: CustomBlending,
      blendSrc: OneFactor,
      blendDst: OneMinusSrcAlphaFactor,
      blendSrcAlpha: OneFactor,
      blendDstAlpha: OneMinusSrcAlphaFactor,
    }),
  );
  // A tighter glow than the pass's default: the widest mips would tint the whole canvas. (Radius 0
  // keeps these as they are; the pass mixes them toward 1.2 - factor as the radius grows.)
  bloom.compositeMaterial.uniforms.bloomFactors.value = [1, 0.6, 0.28, 0.1, 0.03];
  const overlay = new Scene();
  const quad = geo(new PlaneGeometry(2, 2));
  overlay.add(new Mesh(quad, composite));
  const flat = new OrthographicCamera(-1, 1, 1, -1, 0, 1);

  let level = 1; // the strike: how lit the rod is right now
  let base = { level: 1, glow: 0, haze: 0, spill: 0, bloom: 1 };

  function applyQuality() {
    if (haloMesh) haloMesh.visible = quality === 'low';
  }

  function paint() {
    const light = isLight();
    const L = look(light);
    const accent = new Color(css('--accent') || (light ? '#14e6ff' : '#145aff'));
    const markInk = new Color(css('--mark-ink') || (light ? '#1b1c20' : '#e8e8ea'));
    // Dark: pale grey polymer; light: near-black, its shape in its clear coat's reflections.
    inkCap.color.copy(markInk).multiplyScalar(light ? 1 : 0.42);
    inkSide.color.copy(markInk).multiplyScalar(light ? 1 : 0.34);
    for (const m of [inkCap, inkSide]) {
      m.roughness = light ? 0.42 : 0.5;
      m.clearcoat = light ? 0.85 : 0.55;
      m.emissive.copy(markInk).multiplyScalar(L.inkGlow * 0.5);
    }
    // The room's reflections (three applies the scene's strength to every lit material).
    scene.environmentIntensity = light ? 0.8 : 0.45;
    renderer.toneMappingExposure = light ? 1.05 : 1.15;
    key.intensity = light ? 2.2 : 1.6;
    ambient.intensity = light ? 0.15 : 0.05;

    for (const m of [sleeveBack, sleeveFront]) {
      m.color.set(light ? 0x363944 : 0xffffff);
      m.opacity = light ? 0.06 : 0.04;
      m.emissive.copy(accent).multiplyScalar(light ? 0.08 : 0.12);
    }
    edge.uEdge.value.set(light ? 0x000000 : 0xffffff).multiplyScalar(light ? 0 : 0.45);
    edge.uEdgeAlpha.value = light ? 0.55 : 0.4;

    rod.uniforms.uColor.value.copy(accent);
    rod.uniforms.uHot.value = L.hot;
    rod.uniforms.uCore.value = L.coreWidth;
    halo.uniforms.uColor.value.copy(accent);
    haze.color.copy(accent);
    // On dark the glow adds light; on white it has to lay colour down instead.
    for (const m of [halo, haze]) addLight(m, L.additive);
    composite.uniforms.uAdd.value = L.additive ? 1 : 0;
    composite.uniforms.uGain.value = light ? 0.8 : 1;
    for (const lamp of lamps) lamp.color.copy(accent);
    base = { level: L.level, glow: L.glow, haze: L.air * 0.25, spill: light ? 4 : 7, bloom: light ? 0.32 : 0.55 };
    for (const m of mats) m.needsUpdate = true;
    applyQuality();
    setLevel(level);
    draw();
  }

  /** How lit the rod is (the strike), 0-1. */
  function setLevel(k: number) {
    level = k;
    rod.uniforms.uLevel.value = base.level * (0.4 + 0.6 * k);
    halo.uniforms.uOpacity.value = base.glow * k;
    haze.opacity = base.haze * k * k;
    bloom.strength = base.bloom * k;
    for (const lamp of lamps) lamp.intensity = base.spill * k;
  }

  function fit() {
    const w = Math.max(1, host.clientWidth);
    const h = Math.max(1, host.clientHeight);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    const need = camera.aspect >= 1 ? FIT : FIT / camera.aspect;
    camera.position.set(0, 0, need / 2 / Math.tan((FOV * Math.PI) / 360));
    camera.updateProjectionMatrix();
    // The bloom works at half the canvas's pixels (and the pass halves again for its mips).
    const bw = Math.max(1, Math.round((w * ratio) / 2));
    const bh = Math.max(1, Math.round((h * ratio) / 2));
    bloomTarget.setSize(bw, bh);
    bloom.setSize(bw, bh);
    draw();
  }

  /** The rod alone, the unlit bars black, into the bloom's target; then blurred. */
  function renderBloom() {
    for (const m of inkMeshes) m.material = occluder;
    for (const m of glassMeshes) m.visible = false;
    if (hazeSprite) hazeSprite.visible = false;
    renderer.setRenderTarget(bloomTarget);
    renderer.clear();
    renderer.render(scene, camera);
    bloom.render(renderer, bloomTarget, bloomTarget, 0, false); // writes its result back into bloomTarget; we read its own mips
    renderer.setRenderTarget(null);
    for (const m of inkMeshes) m.material = [inkCap, inkSide];
    for (const m of glassMeshes) m.visible = true;
    if (hazeSprite) hazeSprite.visible = true;
  }

  function frameOnce() {
    const glow = quality === 'high';
    if (glow) renderBloom();
    renderer.render(scene, camera);
    if (glow) {
      renderer.autoClear = false;
      renderer.render(overlay, flat);
      renderer.autoClear = true;
    }
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
    if (!perf) return frameOnce();
    // Development only: how long one draw takes on the GPU too (waits for it), kept on window.
    const t = performance.now();
    frameOnce();
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
    // Frames running long: step down to the glow without the bloom pass, for good.
    if (quality === 'high' && dt > 0 && budget.add(dt * 1000)) {
      quality = 'low';
      applyQuality();
    }
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

  if (import.meta.env.DEV) {
    const w = window as unknown as { __logoBench?: (n: number) => number; __logoSnap?: () => string; __logoQuality?: () => Quality };
    // Development only: the mean time of `n` full frames (bloom included), waiting for the GPU.
    w.__logoBench = (n) => {
      const gl = renderer.getContext();
      const px = new Uint8Array(4);
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      const t = performance.now();
      for (let i = 0; i < n; i++) {
        apply(pose(i * 0.016 + 6, 0));
        frameOnce();
      }
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      return (performance.now() - t) / n;
    };
    // The still pose, fully lit, as a PNG, to remake the static images (static/home/).
    // On dark the glow is added as light with no alpha, which an image can't hold: the snapshot
    // writes it into the alpha too, so the still image keeps its glow.
    w.__logoSnap = () => {
      composite.uniforms.uKeep.value = 1;
      for (const m of [halo, haze]) addLight(m, false);
      apply(REST);
      setLevel(1);
      draw();
      const png = canvas.toDataURL('image/png');
      composite.uniforms.uKeep.value = 0;
      paint();
      return png;
    };
    w.__logoQuality = () => quality;
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
      bloomTarget.dispose();
      bloom.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
    },
  };
}
