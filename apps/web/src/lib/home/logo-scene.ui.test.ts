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
    // A capable device, so the full-quality path (bloom) runs whatever machine runs the tests
    // (CI runners have few cores, which would start the light step).
    vi.spyOn(navigator, 'hardwareConcurrency', 'get').mockReturnValue(8);
    Object.defineProperty(navigator, 'deviceMemory', { value: 8, configurable: true });
  });
  afterEach(() => {
    vi.restoreAllMocks();
    delete (navigator as Navigator & { deviceMemory?: number }).deviceMemory;
  });

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
  describe('leaning toward the pointer', () => {
    const media = (m: { hover: boolean; fine: boolean; reduced: boolean }) => {
      const real = window.matchMedia;
      window.matchMedia = ((q: string) => ({
        ...real(q),
        matches: q.includes('hover: hover') ? m.hover : q.includes('pointer: fine') ? m.fine : q.includes('reduced-motion') ? m.reduced : false,
      })) as typeof window.matchMedia;
      return () => (window.matchMedia = real);
    };
    const pointerAdds = (spy: ReturnType<typeof vi.spyOn>) => spy.mock.calls.filter(([type]) => type === 'pointermove').length;

    it('listens for the pointer while it runs, and lets go on dispose', () => {
      const restore = media({ hover: true, fine: true, reduced: false });
      const on = vi.spyOn(window, 'addEventListener');
      const off = vi.spyOn(window, 'removeEventListener');
      const { scene } = mount(false);
      expect(pointerAdds(on)).toBe(1);
      scene.dispose();
      expect(pointerAdds(off)).toBe(1);
      expect(off).toHaveBeenCalledWith('blur', expect.any(Function));
      restore();
    });

    it('stops listening while the tab is hidden, and starts again when it shows', () => {
      const restore = media({ hover: true, fine: true, reduced: false });
      const on = vi.spyOn(window, 'addEventListener');
      const off = vi.spyOn(window, 'removeEventListener');
      const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true);
      const { scene } = mount(false);
      expect(pointerAdds(on)).toBe(0); // hidden from the start: no frame, no listener
      hidden.mockReturnValue(false);
      document.dispatchEvent(new Event('visibilitychange'));
      expect(pointerAdds(on)).toBe(1);
      hidden.mockReturnValue(true);
      document.dispatchEvent(new Event('visibilitychange'));
      expect(pointerAdds(off)).toBe(1);
      scene.dispose();
      restore();
    });

    it('never listens on touch, with reduced motion, or when still', () => {
      for (const m of [
        { hover: false, fine: false, reduced: false },
        { hover: true, fine: true, reduced: true },
      ]) {
        const restore = media(m);
        const on = vi.spyOn(window, 'addEventListener');
        const { scene } = mount(false);
        expect(pointerAdds(on)).toBe(0);
        scene.dispose();
        on.mockRestore();
        restore();
      }
      const restore = media({ hover: true, fine: true, reduced: false });
      const on = vi.spyOn(window, 'addEventListener');
      mount(true).scene.dispose();
      expect(pointerAdds(on)).toBe(0);
      restore();
    });
  });
});
