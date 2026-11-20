// The 3D logo's life cycle without a GPU: three's renderer is faked, the rest of three is real.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AgXToneMapping, BufferGeometry, Material, Mesh, Texture, WebGLRenderTarget, type Object3D } from 'three';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';

const rendered: Object3D[] = [];
const renderer = {
  setPixelRatio: vi.fn(),
  setClearColor: vi.fn(),
  setSize: vi.fn(),
  render: vi.fn((scene: Object3D) => rendered.push(scene)),
  dispose: vi.fn(),
  forceContextLoss: vi.fn(),
  getContext: vi.fn(),
  getClearColor: vi.fn(),
  getClearAlpha: vi.fn(() => 0),
  setRenderTarget: vi.fn(),
  clear: vi.fn(),
  autoClear: true,
  toneMapping: 0,
  toneMappingExposure: 1,
  outputColorSpace: '',
  shadowMap: { enabled: false },
};

vi.mock('three', async (orig) => {
  const three = await orig<typeof import('three')>();
  return {
    ...three,
    WebGLRenderer: vi.fn(() => renderer),
    PMREMGenerator: vi.fn(() => ({ fromScene: () => ({ texture: new three.Texture() }), dispose: vi.fn() })),
  };
});

const { mountLogo } = await import('./logo-scene');

/** The scene with the logo in it (the bloom's overlay is a scene too). */
const main = () => rendered.find((o) => o.type === 'Scene' && o.children.some((c) => c.type === 'Group'))!;

const observers: { disconnect: ReturnType<typeof vi.fn> }[] = [];
class FakeObserver {
  disconnect = vi.fn();
  constructor() {
    observers.push(this);
  }
  observe() {}
  unobserve() {}
}

describe('3D logo scene', () => {
  let raf: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    rendered.length = 0;
    observers.length = 0;
    vi.clearAllMocks();
    globalThis.IntersectionObserver = FakeObserver as unknown as typeof IntersectionObserver;
    globalThis.ResizeObserver = FakeObserver as unknown as typeof ResizeObserver;
    raf = vi.fn(() => 1);
    globalThis.requestAnimationFrame = raf as unknown as typeof requestAnimationFrame;
    globalThis.cancelAnimationFrame = vi.fn();
  });
  afterEach(() => vi.restoreAllMocks());

  const mount = (still: boolean) => {
    const host = document.createElement('div');
    const canvas = document.createElement('canvas');
    host.append(canvas);
    document.body.append(host);
    const onReady = vi.fn();
    const scene = mountLogo(canvas, host, { onReady, onFail: vi.fn(), still });
    return { scene, onReady, canvas };
  };

  it('has no shadow maps, no floor and no shadow-casting lights', () => {
    mount(false);
    expect(renderer.shadowMap.enabled).toBe(false);
    const scene = main();
    scene.traverse((o) => {
      expect(o.castShadow).toBe(false);
      expect(o.receiveShadow).toBe(false);
      expect(o.type).not.toBe('SpotLight');
      if (o instanceof Mesh) expect(o.geometry.type).not.toBe('PlaneGeometry');
    });
  });

  it('draws the still pose once and never schedules a frame when still (reduced motion)', () => {
    const { onReady } = mount(true);
    expect(onReady).toHaveBeenCalledTimes(1);
    expect(raf).not.toHaveBeenCalled();
    expect(renderer.render).toHaveBeenCalled();
  });

  it('tone maps with AgX and draws the bloom over the frame where it can', () => {
    mount(false);
    expect(renderer.toneMapping).toBe(AgXToneMapping);
    expect(renderer.setRenderTarget).toHaveBeenCalled(); // the rod alone, into the bloom's target
    expect(rendered.filter((o) => o.type === 'Scene').length).toBeGreaterThan(1); // and the overlay
  });

  it('moves when allowed to', () => {
    mount(false);
    expect(raf).toHaveBeenCalled();
  });

  it('gives back every geometry, material and texture, the observers and the context', () => {
    const geo = vi.spyOn(BufferGeometry.prototype, 'dispose');
    const mat = vi.spyOn(Material.prototype, 'dispose');
    const tex = vi.spyOn(Texture.prototype, 'dispose');
    const off = vi.spyOn(window, 'removeEventListener');
    const target = vi.spyOn(WebGLRenderTarget.prototype, 'dispose');
    const bloom = vi.spyOn(UnrealBloomPass.prototype, 'dispose');
    const { scene } = mount(false);
    const meshes: Mesh[] = [];
    main().traverse((o) => o instanceof Mesh && meshes.push(o));
    expect(meshes.length).toBeGreaterThanOrEqual(6); // two bars, the rod, its sleeve (2) and the halo
    const geos = new Set(meshes.map((m) => m.geometry));
    const mats = new Set(meshes.flatMap((m) => (Array.isArray(m.material) ? m.material : [m.material])));

    scene.dispose();
    const disposedGeos = new Set(geo.mock.contexts);
    const disposedMats = new Set(mat.mock.contexts);
    for (const g of geos) expect(disposedGeos.has(g)).toBe(true);
    for (const m of mats) expect(disposedMats.has(m)).toBe(true);
    expect(tex).toHaveBeenCalled(); // the environment map
    expect(target).toHaveBeenCalled(); // the bloom's own target
    expect(bloom).toHaveBeenCalled(); // and the pass's mips
    expect(renderer.dispose).toHaveBeenCalled();
    expect(renderer.forceContextLoss).toHaveBeenCalled();
    for (const o of observers) expect(o.disconnect).toHaveBeenCalled();
    expect(off).toHaveBeenCalledWith('scroll', expect.any(Function));
  });
});
