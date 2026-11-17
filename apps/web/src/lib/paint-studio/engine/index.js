// @ts-nocheck
// The animated paint engine (layers, effects, scene elements, keyframes, the GIF, APNG and WebP
// encoders), from the owner's cyex paints tool. Each file installs its part into one shared
// object, in this order; the page and the render worker both import this module, so frames
// match wherever they are made. Nothing here touches the DOM at load time.
import core from './core.js';
import fx from './fx.js';
import encode from './encode.js';
import library from './library.js';
import more from './more.js';
import geometric from './packs/geometric.js';
import nature from './packs/nature.js';
import retro from './packs/retro.js';
import stylise from './packs/stylise.js';
import surfaces from './packs/surfaces.js';
import elements from './elements.js';
import engine from './engine.js';
import presets from './presets.js';
import surprise from './surprise.js';
import tags from './tags.js';

const PL = {};
for (const install of [core, fx, encode, library, more, geometric, nature, retro, stylise, surfaces, elements, engine, presets, surprise, tags]) install(PL);

export default PL;
