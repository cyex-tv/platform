import { describe, expect, it } from 'vitest';
import { latestWins, latestWinsBy } from './latest';

const gate = () => {
  let open!: () => void;
  const p = new Promise<void>((r) => (open = r));
  return { p, open };
};

describe('latestWins', () => {
  it('keeps the last pick made while a save is on its way', async () => {
    let saved = 'a';
    const sent: string[] = [];
    const g = gate();
    const save = latestWins<string>(
      async (v) => {
        sent.push(v);
        if (v === 'b') await g.p;
        saved = v;
      },
      (v) => v === saved,
    );
    const first = save('b');
    await save('c'); // while b is saving
    await save('a'); // changes its mind, last one wins
    g.open();
    await first;
    expect(sent).toEqual(['b', 'a']);
    expect(saved).toBe('a');
  });

  it('does not send a pick that is already saved, and stops on a failure', async () => {
    let saved = 'a';
    const sent: string[] = [];
    const g = gate();
    const save = latestWins<string>(
      async (v) => {
        sent.push(v);
        await g.p;
        if (v === 'b') throw new Error('no');
        saved = v;
      },
      (v) => v === saved,
    );
    const first = save('b');
    await save('c');
    g.open();
    await expect(first).rejects.toThrow('no');
    expect(sent).toEqual(['b']);
    await save('a'); // already saved: nothing to send
    expect(sent).toEqual(['b']);
  });
});

describe('latestWinsBy', () => {
  it('saves a pick on the thing it was made for, and keeps other things’ picks apart', async () => {
    const saved: Record<string, string> = { s1: 'a', s2: 'a' };
    const sent: string[] = [];
    const g = gate();
    const save = latestWinsBy<string>(
      async (key, v) => {
        sent.push(`${key}:${v}`);
        if (key === 's1') await g.p;
        saved[key] = v;
      },
      (key, v) => v === saved[key],
    );
    const first = save('s1', 'b');
    await save('s2', 'c'); // another set while s1 saves: its own save, not a replacement
    g.open();
    await first;
    expect(sent).toEqual(['s1:b', 's2:c']);
    expect(saved).toEqual({ s1: 'b', s2: 'c' });
  });
});
