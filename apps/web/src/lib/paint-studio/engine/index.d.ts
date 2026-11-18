// Types for the parts of the engine the studio uses. The engine itself is plain JS.

/** Straight-alpha colour stop of a layer's gradient: `at` 0..1, `color` #rrggbb, `a` 0..1. */
export type EngineStop = { at: number; color: string; a: number };
/** A weighted palette: each colour's `w` is its share of the ramp. */
export type EnginePalette = { colors: { c: string; a: number; w: number }[]; blend: number; loop: boolean };

export type ParamDef =
  | { k: string; t: 'range'; l: string; min: number; max: number; step: number; d: number; hint?: string; g?: string; show?: (p: Record<string, unknown>) => boolean }
  | { k: string; t: 'color'; l: string; d: string; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'select'; l: string; o: [string, string][]; d: string; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'bool'; l: string; d: boolean; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'text'; l: string; d: string; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'image'; l: string; d: string; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'stops'; l: string; d: EngineStop[]; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string }
  | { k: string; t: 'palette'; l: string; d: EnginePalette; g?: string; show?: (p: Record<string, unknown>) => boolean; hint?: string };

export type ItemDef = { id: string; name: string; cat: string; desc?: string; tags?: string[]; params: ParamDef[]; pack?: string; draw?: unknown; sample?: unknown; run?: unknown };

export type Transform = { scale: number; rotate: number; ox: number; oy: number; sx: number; sy: number; spin: number; tile: boolean };
export type Tweak = { hue: number; sat: number; bright: number; contrast: number; tint: string; tintAmt: number };
export type Keys = Record<string, { t: number; v: number; e: string }[]>;

export type Layer = {
  id: string; type: string; name: string; visible: boolean; blend: string; opacity: number;
  params: Record<string, unknown>; xf: Transform; tweak: Tweak; clip?: boolean; group?: string; keys?: Keys;
};
export type Effect = { id: string; type: string; name: string; visible: boolean; params: Record<string, unknown>; keys?: Keys };
/** Engine-side shadows use packed integer colours; the studio converts them to hex. */
export type EngineShadow = { x_offset: number; y_offset: number; radius: number; color: number };

export type AnimDoc = {
  name: string; width: number; height: number; fps: number; duration: number;
  layers: Layer[]; effects: Effect[]; shadows: EngineShadow[]; tweak: Tweak;
  groups: { id: string; name: string; visible: boolean; collapsed: boolean }[];
};

export type Compiled = { W: number; H: number; frames: number };

export type EncodeOpts = { delayCs?: number; delayMs?: number; colors?: number; dither?: boolean; alphaCut?: number; matte?: [number, number, number] | null; loop?: number; quality?: number };

export type Engine = {
  PALETTES: Record<string, EngineStop[]>;
  LAYERS: Record<string, ItemDef>;
  EFFECTS: Record<string, ItemDef>;
  LAYER_CATS: string[];
  EFFECT_CATS: string[];
  BLENDS: [string, string][];
  SEARCH_SYNONYMS: string[][];
  ANIM_PRESETS: (() => AnimDoc)[];
  SCENE_COUNT: number;
  imageCache: Record<string, { w: number; h: number; data: Uint8ClampedArray }>;
  clone<T>(x: T): T;
  paletteToStops(p: EnginePalette): EngineStop[];
  stopsToPalette(s: EngineStop[], loop?: boolean): EnginePalette;
  Engine: {
    blankDoc(): AnimDoc;
    newLayer(type: string, over?: Partial<Layer>): Layer;
    newEffect(type: string, over?: Partial<Effect>): Effect;
    sanitizeDoc(raw: unknown): { doc: AnimDoc; dropped: number };
    frameCount(doc: AnimDoc): number;
    compile(doc: AnimDoc): Compiled;
    renderFrame(cp: Compiled, index: number, W?: number, H?: number): ImageData;
    fixParams(def: ItemDef, params: Record<string, unknown>): Record<string, unknown>;
    DEFAULT_XF: Transform;
    DEFAULT_TWEAK: Tweak;
  };
  Encode: {
    encodeGif(frames: ImageData[], opts?: EncodeOpts): Blob;
    encodeApng(frames: ImageData[], opts?: EncodeOpts): Promise<Blob>;
    encodeWebp(frames: ImageData[], opts?: EncodeOpts): Promise<Blob>;
    estimateSize(frames: ImageData[], format: 'gif' | 'apng' | 'webp', opts?: EncodeOpts & { total?: number }): Promise<number>;
  };
  Surprise: { make(seed?: number | string): AnimDoc };
};

declare const PL: Engine;
export default PL;
