<script lang="ts">
  // The keyboard shortcut sheet: `?` anywhere outside a field, or the footer's Keyboard shortcuts.
  // A modal dialog (focus stays inside, Escape and × close it, focus goes back where it was) listing
  // this page's keys first, then the site-wide ones; pages register theirs (shortcuts.svelte.ts).
  import Icon from '$lib/components/Icon.svelte';
  import { asksForSheet, keyParts, shortcuts } from './shortcuts.svelte';

  let dialog: HTMLDialogElement;
  let list: HTMLDivElement;
  let back: HTMLElement | null = null;

  $effect(() => {
    if (shortcuts.open && !dialog.open) {
      back = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      dialog.showModal();
      // Focus on the list (it scrolls with the arrow keys), not the × in the corner.
      list.focus();
    } else if (!shortcuts.open && dialog.open) dialog.close();
  });

  function onclose() {
    shortcuts.open = false;
    // Back to what had focus (the footer link, or the page); not if focus already moved on.
    if (back?.isConnected && (document.activeElement === document.body || dialog.contains(document.activeElement))) back.focus();
    back = null;
  }

  function onwindowkey(e: KeyboardEvent) {
    if (!asksForSheet(e)) return;
    // Another dialog is open: its keys first.
    if (!shortcuts.open && document.querySelector('dialog[open]')) return;
    e.preventDefault();
    shortcuts.open = !shortcuts.open;
  }

  // A click on the backdrop (the dialog box itself, outside its content) closes it.
  function onclick(e: MouseEvent) {
    if (e.target === dialog) shortcuts.open = false;
  }
</script>

<svelte:window onkeydown={onwindowkey} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="ui-dialog keys" bind:this={dialog} aria-labelledby="keys-title" {onclose} {onclick}>
  <div class="inner">
    <header class="head">
      <h2 id="keys-title">Keyboard shortcuts</h2>
      <button class="ui-icon close" type="button" aria-label="Close" onclick={() => (shortcuts.open = false)}><Icon name="x" size={18} /></button>
    </header>
    <div class="list" bind:this={list} tabindex="-1" role="region" aria-label="Shortcuts">
      {#if shortcuts.open}
        {#each shortcuts.groups as g (g.id)}
          <section aria-labelledby="keys-{g.id}">
            <h3 class="ui-label" id="keys-{g.id}">{g.title}</h3>
            <dl>
              {#each g.items as s (s.label)}
                <div class="row">
                  <dt>{s.label}</dt>
                  <dd>
                    {#each s.keys as combo, i (combo)}
                      {#if i > 0}<span class="or">or</span>{/if}
                      <span class="combo">{#each keyParts(combo) as k, j (j)}<kbd>{k}</kbd>{/each}</span>
                    {/each}
                  </dd>
                </div>
              {/each}
            </dl>
          </section>
        {/each}
      {/if}
    </div>
  </div>
</dialog>

<style>
  .keys { width: min(560px, calc(100vw - 24px)); max-height: min(720px, calc(100dvh - 48px)); }
  .inner { display: flex; flex-direction: column; max-height: inherit; }
  .head { display: flex; align-items: center; gap: 8px; padding: 10px 8px 6px 16px; }
  h2 { flex: 1; margin: 0; font-size: var(--t-ui); font-weight: 600; }
  .list:focus { outline: none; }
  .list { display: grid; gap: 18px; padding: 6px 16px 18px; overflow-y: auto; overscroll-behavior: contain; }
  section { display: grid; gap: 4px; }
  h3 { margin: 0 0 2px; }
  dl { margin: 0; display: grid; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 36px; padding: 4px 0;
    border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  dt { min-width: 0; font-size: var(--t-nav); color: var(--text); line-height: 1.4; }
  dd { flex: none; display: flex; align-items: center; gap: 6px; margin: 0; }
  .combo { display: inline-flex; gap: 3px; }
  kbd { display: inline-grid; place-items: center; min-width: 24px; height: 24px; padding: 0 6px; border: 1px solid var(--border-strong);
    border-bottom-width: 2px; border-radius: var(--radius); background: var(--surface); color: var(--text);
    font-size: 12px; font-weight: 600; line-height: 1; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .or { font-size: 12px; color: var(--dim); }
  @media (max-width: 480px) {
    .row { flex-direction: column; align-items: flex-start; gap: 6px; padding: 8px 0; }
  }
  @media (forced-colors: active) { kbd { border-color: CanvasText; } }
</style>
