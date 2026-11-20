import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { initDocs } from './behaviour';

const PAGE = `
<div class="docs">
  <aside class="docs-nav">
    <div class="nav-search"><input aria-label="Filter" /><span class="nav-kbd"></span><button class="input-clear" type="button">Clear</button></div>
    <nav class="nav-scroll">
      <p data-filter-status></p>
      <div class="nav-group"><a class="nav-item" href="#emotes">Emotes</a><a class="nav-item" href="#sets" data-search="collections">Sets</a></div>
      <div class="nav-group"><a class="nav-item" href="#admin-users">Admin users</a></div>
      <p class="nav-empty" hidden></p>
    </nav>
  </aside>
  <main>
    <section id="emotes">Emotes</section>
    <section id="sets">Sets</section>
    <section id="admin-users">Admin users</section>
    <div class="code-block" role="tablist">
      <button class="lang-tab" data-lang="curl" role="tab" aria-selected="true">curl</button>
      <button class="lang-tab" data-lang="js" role="tab" aria-selected="false">JS</button>
      <button class="lang-tab" data-lang="py" role="tab" aria-selected="false">Python</button>
      <pre data-lang="curl">curl x</pre><pre data-lang="js">fetch x</pre><pre data-lang="py">requests x</pre>
    </div>
  </main>
</div>`;

let stop: () => void;
const input = () => document.querySelector<HTMLInputElement>('.nav-search input')!;
const item = (id: string) => document.querySelector<HTMLElement>(`a.nav-item[href="#${id}"]`)!;
const visiblePre = () => [...document.querySelectorAll<HTMLElement>('pre')].filter((p) => !p.hidden).map((p) => p.dataset.lang);

beforeEach(() => {
  document.body.innerHTML = PAGE;
  localStorage.clear();
  sessionStorage.clear();
  stop = initDocs(document, { headerOffset: 0 });
});
afterEach(() => stop());

describe('docs filter', () => {
  it('narrows the index to entries whose own text matches, and says how many', async () => {
    const user = userEvent.setup();
    await user.type(input(), 'set');
    expect(item('sets').hidden).toBe(false);
    expect(item('emotes').hidden).toBe(true);
    expect(item('admin-users').hidden).toBe(true);
    expect(document.querySelectorAll('.nav-group')[1].hasAttribute('hidden')).toBe(true); // an empty group goes
    expect(document.querySelector('[data-filter-status]')!.textContent).toBe('1 entry matches');
    expect(document.querySelector('.docs-nav')!.classList.contains('filtering')).toBe(true);
  });

  it('matches the hidden search words too', async () => {
    const user = userEvent.setup();
    await user.type(input(), 'collections');
    expect(item('sets').hidden).toBe(false);
  });

  it('says so when nothing matches, and clearing brings everything back', async () => {
    const user = userEvent.setup();
    await user.type(input(), 'zzz');
    expect(document.querySelector<HTMLElement>('.nav-empty')!.hidden).toBe(false);
    expect(document.querySelector('.nav-empty')!.textContent).toContain('zzz');
    expect(document.querySelector('[data-filter-status]')!.textContent).toBe('Nothing matches');
    await user.click(document.querySelector('.input-clear')!);
    expect(input().value).toBe('');
    expect([...document.querySelectorAll<HTMLElement>('.nav-item')].every((a) => !a.hidden)).toBe(true);
    expect(document.querySelector<HTMLElement>('.nav-empty')!.hidden).toBe(true);
  });

  it('Ctrl+K focuses the filter, and Escape in it clears and leaves', async () => {
    const user = userEvent.setup();
    await user.keyboard('{Control>}k{/Control}');
    expect(document.activeElement).toBe(input());
    await user.keyboard('emo');
    expect(item('sets').hidden).toBe(true);
    await user.keyboard('{Escape}');
    expect(input().value).toBe('');
    expect(document.activeElement).not.toBe(input());
    expect(item('sets').hidden).toBe(false);
  });
});

describe('code sample tabs', () => {
  it('start on the first language and switch every sample at once', async () => {
    const user = userEvent.setup();
    expect(visiblePre()).toEqual(['curl']);
    await user.click(document.querySelector('[data-lang="py"].lang-tab')!);
    expect(visiblePre()).toEqual(['py']);
    expect(localStorage.getItem('cyexDocsLang')).toBe('py');
  });

  it('arrow keys, Home and End move between tabs and wrap', async () => {
    const user = userEvent.setup();
    document.querySelector<HTMLElement>('.lang-tab')!.focus();
    await user.keyboard('{ArrowLeft}');
    expect(visiblePre()).toEqual(['py']);
    expect(document.activeElement).toBe(document.querySelector('[data-lang="py"].lang-tab'));
    await user.keyboard('{ArrowRight}');
    expect(visiblePre()).toEqual(['curl']);
    await user.keyboard('{End}');
    expect(visiblePre()).toEqual(['py']);
    await user.keyboard('{Home}');
    expect(visiblePre()).toEqual(['curl']);
  });

  it('remembers the choice on the next visit, and ignores a saved language the page lacks', () => {
    stop();
    document.body.innerHTML = PAGE;
    localStorage.setItem('cyexDocsLang', 'js');
    stop = initDocs(document, { headerOffset: 0 });
    expect(visiblePre()).toEqual(['js']);
    stop();
    document.body.innerHTML = PAGE;
    localStorage.setItem('cyexDocsLang', 'cobol');
    stop = initDocs(document, { headerOffset: 0 });
    expect(visiblePre()).toEqual(['curl']);
  });
});

describe('cleanup', () => {
  it('removes its listeners', async () => {
    const user = userEvent.setup();
    stop();
    await user.keyboard('{Control>}k{/Control}');
    expect(document.activeElement).not.toBe(input());
    stop = initDocs(document, { headerOffset: 0 });
  });
});
