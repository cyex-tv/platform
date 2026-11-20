// The mock's upload route reads the multipart form as UTF-8, so names in any script and emoji
// arrive whole (they were mojibake when it was read as latin1).
import { spawn } from 'node:child_process';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const PORT = 19300 + Math.floor(Math.random() * 500);
let child;

beforeAll(async () => {
  child = spawn(process.execPath, ['dev/mock-api.mjs'], { env: { ...process.env, PORT: String(PORT) }, stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      await fetch(`http://localhost:${PORT}/mock/nothing`);
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  throw new Error('mock did not start');
}, 15000);

afterAll(() => child?.kill());

async function upload(name) {
  const form = new FormData();
  form.set('name', name);
  form.set('file', new Blob([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0xff, 0xfe])], { type: 'image/png' }), 'a.png');
  return fetch(`http://localhost:${PORT}/v1/emotes`, { method: 'POST', body: form, headers: { cookie: 'mock_session=1' } });
}

describe('mock POST /v1/emotes', () => {
  it('keeps right-to-left and emoji names whole', async () => {
    for (const name of ['مرحبا', 'שלום', 'catJAM😀', '猫ジャム']) {
      const res = await upload(name);
      expect(res.status).toBe(202);
      expect((await res.json()).name).toBe(name);
    }
  });

  it('never answers a name of hang (for the stall and Cancel checks)', async () => {
    const out = await Promise.race([upload('hang').then(() => 'answered'), new Promise((r) => setTimeout(() => r('silent'), 800))]);
    expect(out).toBe('silent');
  });
});
