import { describe, expect, it } from 'vitest';
import { latestWins } from './latest';

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
