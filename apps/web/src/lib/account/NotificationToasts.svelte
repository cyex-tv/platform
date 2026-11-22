<script lang="ts">
  // New notifications pop up from the header, under the bell (top right; full width on phones),
  // like the deck's Twitch notifications: one sentence and its icon, a click opens it (and marks
  // it read), × closes it. Each stays 6s, held while the pointer or focus is on it; at most three.
  // They come from the bell's count: when it goes up, the list is fetched and every unread one
  // not seen yet and newer than the first count is shown. That first count after a page load
  // only sets the baseline, so unread ones from before never pop up.
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import { isoParts } from '$lib/format';
  import { dur } from '$lib/motion';
  import { describe } from './notifications';
  import { inbox, loadInbox, markRead, type Note } from './inbox.svelte';
  import { loadPrefs, quietKinds } from './prefs.svelte';
  import { fresh } from './toasts';
  import { holds, pausableTimer, type HoldReason, type PausableTimer } from '$lib/tool/pausable-timer';

  type Shown = { note: Note; timer?: PausableTimer; hold?: ReturnType<typeof holds> };
  const LIFE = 6000;
  let since = Date.now();
  const seen = new Set<string>();
  let baseline: number | null = null;
  let shown = $state<Shown[]>([]);

  $effect(() => {
    const c = inbox.count;
    if (!inbox.known) return;
    untrack(() => void check(c));
  });

  // Whatever the list says (the bell opened it, the page loaded it), what's in it is seen.
  $effect(() => {
    for (const n of inbox.items ?? []) if (n.read) seen.add(n.id);
  });

  async function check(count: number) {
    if (baseline === null) {
      baseline = count;
      since = Date.now() - 5_000; // a little clock skew between here and the API
      return;
    }
    const up = count > baseline;
    baseline = count;
    if (!up) return;
    await Promise.all([loadInbox(), loadPrefs()]);
    const add = fresh(inbox.items ?? [], seen, since);
    for (const n of add) seen.add(n.id);
    // Kinds whose pop-up is off (Settings → Notifications) still count as seen, just quietly.
    const quiet = quietKinds();
    for (const n of add.filter((x) => !quiet.has(x.kind)).slice(0, 3).reverse()) push(n);
  }

  function push(note: Note) {
    const t: Shown = { note };
    for (const old of shown.slice(2)) old.timer?.cancel();
    shown = [t, ...shown].slice(0, 3);
    arm(t);
  }
  function arm(t: Shown) {
    t.timer = pausableTimer(() => close(t.note.id), LIFE);
    t.hold = holds(t.timer);
  }
  function hold(t: Shown, by: HoldReason) {
    t.hold?.(by, true);
  }
  function release(t: Shown, by: HoldReason) {
    t.hold?.(by, false);
  }
  function close(id: string) {
    shown.find((t) => t.note.id === id)?.timer?.cancel();
    shown = shown.filter((t) => t.note.id !== id);
  }
  async function open(n: Note, href: string | null) {
    close(n.id);
    void markRead([n.id]);
    if (href) await goto(href);
  }
</script>

<div class="stack" aria-live="polite" aria-label="New notifications">
  {#each shown as t (t.note.id)}
    {@const line = describe(t.note.kind, t.note.payload)}
    <div
      class="toast"
      role="status"
      in:fly={{ y: -8, duration: dur(200), opacity: 0 }}
      out:fade={{ duration: dur(140) }}
      animate:flip={{ duration: dur(200) }}
      onpointerenter={() => hold(t, 'pointer')}
      onpointerleave={() => release(t, 'pointer')}
      onfocusin={() => hold(t, 'focus')}
      onfocusout={() => release(t, 'focus')}
    >
      <button class="body" type="button" onclick={() => open(t.note, line.href)}>
        <span class="icon"><Icon name={line.icon} /></span>
        <span class="text">
          <span class="kicker">New notification</span>
          <span class="sentence">{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</span>
        </span>
      </button>
      <button class="close" type="button" aria-label="Dismiss" onclick={() => close(t.note.id)}><Icon name="x" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .stack { position: fixed; top: calc(var(--nav-h) + 10px + env(safe-area-inset-top, 0px)); z-index: var(--z-toast);
    right: var(--page-pad); display: grid; gap: 8px;
    width: min(360px, calc(100vw - 2 * var(--page-pad))); pointer-events: none; }
  .toast { position: relative; display: flex; align-items: flex-start; pointer-events: auto; border: 1px solid var(--border-strong);
    border-radius: var(--radius-m); background: var(--surface); color: var(--text); box-shadow: var(--shadow-pop); }
  .body { flex: 1; min-width: 0; display: flex; align-items: flex-start; gap: 12px; padding: 12px 8px 12px 14px; border: 0; background: none;
    color: inherit; font: inherit; font-size: var(--t-nav); text-align: left; cursor: pointer; border-radius: var(--radius-m) 0 0 var(--radius-m); }
  .icon { flex: none; margin-top: 1px; color: var(--accent-text); }
  .text { display: grid; gap: 2px; min-width: 0; }
  .kicker { font-size: var(--t-fine); color: var(--muted); }
  .sentence { font-weight: 600; line-height: 1.4; overflow-wrap: anywhere; }
  .close { position: relative; flex: none; display: grid; place-items: center; width: 32px; height: 32px; margin: 6px 6px 0 0; padding: 0;
    border: 0; border-radius: var(--radius); background: none; color: var(--dim); cursor: pointer; }
  .close::after { content: ''; position: absolute; inset: -4px; }
  .body:focus-visible, .close:focus-visible { outline-offset: -2px; }
  .body:active, .close:active { opacity: .6; }
  @media (hover: hover) and (pointer: fine) { .close:hover { color: var(--text); background: var(--row-hover); } }
  @media (max-width: 720px) { .stack { left: var(--page-pad); right: var(--page-pad); width: auto; } }
  @media (forced-colors: active) { .toast { border-color: CanvasText; } }
</style>
