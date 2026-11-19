<script lang="ts">
  // An emote's visibility and flags, the same controls on the upload form and the edit page:
  // Visibility (Public / Unlisted / Private, ADR 0020) as a segmented control, then Zero-width
  // and NSFW as switches.
  // Turning NSFW on asks first (what it means, and that rule-breaking images are taken down);
  // until "Mark NSFW" is chosen the flag stays off. Turning it off needs no question.
  //   <EmoteFlags bind:zeroWidth bind:visibility bind:nsfw id="f1" />
  import { tick } from 'svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { EmoteVisibility } from '$lib/api';
  import { VISIBILITY_HINT, VISIBILITY_OPTIONS } from './visibility';

  type Props = { zeroWidth: boolean; visibility: EmoteVisibility; nsfw: boolean; id: string; disabled?: boolean };
  let { zeroWidth = $bindable(), visibility = $bindable(), nsfw = $bindable(), id, disabled = false }: Props = $props();

  let asking = $state(false);
  let confirmBtn = $state<HTMLButtonElement>();
  let nsfwSwitch = $state<HTMLInputElement>();

  function toggleNsfw(e: Event & { currentTarget: HTMLInputElement }) {
    if (e.currentTarget.checked && !nsfw) {
      e.currentTarget.checked = false;
      asking = true;
      tick().then(() => confirmBtn?.focus());
      return;
    }
    nsfw = e.currentTarget.checked;
    asking = false;
  }

  function answer(yes: boolean) {
    nsfw = yes;
    asking = false;
    nsfwSwitch?.focus();
  }
</script>

<div class="flags">
  <div class="vis">
    <span class="ui-label" id="{id}-vis">Visibility</span>
    <Segmented label="Visibility" size="s" options={VISIBILITY_OPTIONS}
      value={visibility} onchange={(v) => !disabled && (visibility = v)} />
    <span class="ui-hint">{VISIBILITY_HINT[visibility]}</span>
  </div>
  <label class="ui-row">
    <div><strong>Zero-width</strong><span class="ui-hint">Drawn over the emote before it, like a hat or rain.</span></div>
    <input class="ui-switch" type="checkbox" bind:checked={zeroWidth} {disabled} />
  </label>
  <label class="ui-row">
    <div><strong>NSFW</strong><span class="ui-hint">Not for every stream. Search leaves it out unless people ask for it.</span></div>
    <input class="ui-switch" type="checkbox" bind:this={nsfwSwitch} checked={nsfw} onchange={toggleNsfw} {disabled} aria-describedby={asking ? `${id}-ask` : undefined} />
  </label>
  {#if asking}
    <div class="ask" id="{id}-ask" role="group" aria-label="Mark as NSFW?">
      <p><strong>Mark it NSFW?</strong> It’s hidden from browsing unless people turn NSFW on. Anything against Twitch’s or Kick’s rules is taken down either way.</p>
      <div class="ui-actions">
        <button type="button" class="ui-btn secondary" bind:this={confirmBtn} onclick={() => answer(true)}>Mark NSFW</button>
        <button type="button" class="ui-btn ghost" onclick={() => answer(false)}>Cancel</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .flags { display: grid; gap: 4px; }
  .vis { display: grid; justify-items: start; gap: 8px; padding-bottom: 8px; }
  .ui-row .ui-hint { display: block; }
  /* The switch stays beside its text on phones too (the shared row wraps there). */
  .flags .ui-row { flex-wrap: nowrap; gap: 16px; }
  .ask { display: grid; gap: 10px; padding: 12px 14px; border: 1px solid var(--border-strong); border-radius: var(--radius-card); background: var(--surface-2, var(--surface)); }
  .ask p { margin: 0; font-size: var(--t-small); line-height: 1.55; color: var(--muted); }
  .ask strong { color: var(--text); }
</style>
