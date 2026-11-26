// Fetches Panchang (Indian Type Foundry, ITF Free Font License) from Fontshare's official download
// into static/fonts/. It is NOT kept in the repository: section 02 of the licence forbids
// distributing the font through public repositories, so the production image build runs this
// (`npm run fonts`) and every deploy serves its own copy from cyex.tv, as section 01 allows.
// Without it the landing page falls back to Arial (dev and tests work the same).
// The woff2 is verified against a pinned sha256 and used unmodified (the licence forbids changes).
// Plain Node, no dependencies: the zip is read with node:zlib.
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync, existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateRawSync } from 'node:zlib';

const URL_ZIP = 'https://api.fontshare.com/v2/fonts/download/panchang';
const FILES = [
  { entry: 'Fonts/WEB/fonts/Panchang-Bold.woff2', out: 'Panchang-Bold.woff2', sha256: 'a98a575d98d41afea5a989499e39ecbae79c3210507dfdba81aaa1afb362fdc6' },
  { entry: 'License/FFL.txt', out: 'Panchang-LICENSE.txt', sha256: null },
];
const dest = join(dirname(fileURLToPath(import.meta.url)), '..', 'static', 'fonts');
const sha = (b) => createHash('sha256').update(b).digest('hex');
const fail = (m) => { console.error(`fetch-fonts: ${m}`); process.exit(1); };

function entries(zip) {
  let e = zip.length - 22;
  while (e >= 0 && zip.readUInt32LE(e) !== 0x06054b50) e--;
  if (e < 0) fail('the download is not a zip file');
  let p = zip.readUInt32LE(e + 16);
  const out = new Map();
  for (let n = zip.readUInt16LE(e + 10); n > 0; n--) {
    const nameLen = zip.readUInt16LE(p + 28), extraLen = zip.readUInt16LE(p + 30), commentLen = zip.readUInt16LE(p + 32);
    out.set(zip.toString('utf8', p + 46, p + 46 + nameLen), { method: zip.readUInt16LE(p + 10), size: zip.readUInt32LE(p + 20), offset: zip.readUInt32LE(p + 42) });
    p += 46 + nameLen + extraLen + commentLen;
  }
  return out;
}
function read(zip, ent) {
  const start = ent.offset + 30 + zip.readUInt16LE(ent.offset + 26) + zip.readUInt16LE(ent.offset + 28);
  const raw = zip.subarray(start, start + ent.size);
  return ent.method === 0 ? raw : inflateRawSync(raw);
}

const font = join(dest, FILES[0].out);
if (existsSync(font) && sha(readFileSync(font)) === FILES[0].sha256) { console.log('fetch-fonts: Panchang is already in place'); process.exit(0); }

const tmp = mkdtempSync(join(tmpdir(), 'panchang-'));
try {
  const res = await fetch(URL_ZIP).catch((e) => fail(`could not download ${URL_ZIP}: ${e.message}`));
  if (!res.ok) fail(`${URL_ZIP} answered ${res.status}`);
  const zip = Buffer.from(await res.arrayBuffer());
  writeFileSync(join(tmp, 'panchang.zip'), zip);
  const all = entries(zip);
  mkdirSync(dest, { recursive: true });
  for (const f of FILES) {
    const key = [...all.keys()].find((k) => k.endsWith(f.entry));
    if (!key) fail(`${f.entry} is not in the download`);
    const bytes = read(zip, all.get(key));
    if (f.sha256 && sha(bytes) !== f.sha256) fail(`${f.out} does not match the pinned sha256 (got ${sha(bytes)}, expected ${f.sha256}). Fontshare changed the file: check it, then update the pin in scripts/fetch-fonts.mjs.`);
    writeFileSync(join(dest, f.out), bytes);
  }
  console.log('fetch-fonts: Panchang-Bold.woff2 and its licence are in static/fonts/');
} finally { rmSync(tmp, { recursive: true, force: true }); }
