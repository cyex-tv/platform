import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import ShowcaseArt from './ShowcaseArt.svelte';
import { DECK_PANES } from './showcase-geometry';

// The cyex deck picture is an exact 2 x 2 grid: four equal panes, equal gaps, one of them lit.
describe('the deck showcase picture', () => {
  it('has four panes of one size in a 2 x 2 grid with equal gaps', () => {
    expect(DECK_PANES).toHaveLength(4);
    const [a, b, c, d] = DECK_PANES;
    for (const p of DECK_PANES) expect([p.w, p.h]).toEqual([a.w, a.h]);
    expect([a.y, c.y]).toEqual([b.y, d.y]);
    expect([a.x, b.x]).toEqual([c.x, d.x]);
    const gapX = b.x - (a.x + a.w);
    const gapY = c.y - (a.y + a.h);
    expect(gapX).toBe(gapY);
    expect(gapX).toBeGreaterThan(0);
    expect(DECK_PANES.filter((p) => p.lit)).toHaveLength(1);
  });

  it('draws them: four equal rects, the lit one among them, beside the sidebar', () => {
    const { body } = render(ShowcaseArt, { props: { kind: 'deck' } });
    const rects = [...body.matchAll(/<rect class="(?:halo|edge|ink pane)[^"]*"[^>]*?x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"/g)];
    const drawn = rects.map((m) => `${m[1]},${m[2]},${m[3]},${m[4]}`);
    const want = DECK_PANES.map((p) => `${p.x},${p.y},${p.w},${p.h}`);
    // Every pane is drawn (the lit one as halo and edge too), none at another size.
    for (const w of want) expect(drawn).toContain(w);
    expect(new Set(drawn.map((d) => d.split(',').slice(2).join(','))).size).toBe(1);
    expect(body).toContain('M31 9v62');
  });

  it('keeps the other pictures even too: the api ends match', () => {
    const { body } = render(ShowcaseArt, { props: { kind: 'api' } });
    expect(body).toContain('x="9" y="22" width="32" height="36"');
    expect(body).toContain('x="79" y="22" width="32" height="36"');
  });
});
