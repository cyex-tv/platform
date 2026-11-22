<script lang="ts">
  // An emote's flags. Two shapes (ADR 0020, amendment 2026/11/22):
  //   creators (the upload dialog and the edit page): two checkboxes, Overlaying (zero-width) and
  //     Private (only their own channel). Everything else is public. Unlisted is a moderator's
  //     call: an emote a moderator took off the listings says so, and Private can't change then.
  //   staff (`staff`, the Edit emote flags panel): Visibility (Public / Unlisted / Private) as a
  //     segmented control, then Zero-width as a switch.
  // NSFW is no flag: NSFW emotes aren't allowed on cyex.tv.
  //   <EmoteFlags bind:zeroWidth bind:visibility id="f1" />
  import Segmented from '$lib/components/Segmented.svelte';
  import type { EmoteVisibility } from '$lib/api';
  import { FLAG_HINT, STAFF_VISIBILITY_HINT, VISIBILITY_OPTIONS } from './visibility';

  type Props = { zeroWidth: boolean; visibility: EmoteVisibility; id: string; disabled?: boolean; staff?: boolean };
  let { zeroWidth = $bindable(), visibility = $bindable(), id, disabled = false, staff = false }: Props = $props();
  const locked = $derived(!staff && visibility === 'UNLISTED');
</script>

{#if staff}
  <div class="flags">
    <div class="vis">
      <span class="ui-label" id="{id}-vis">Visibility</span>
      <Segmented label="Visibility" size="s" options={VISIBILITY_OPTIONS}
        value={visibility} onchange={(v) => !disabled && (visibility = v)} />
      <span class="ui-hint">{STAFF_VISIBILITY_HINT[visibility]}</span>
    </div>
    <label class="ui-row">
      <div><strong>Zero-width</strong><span class="ui-hint">{FLAG_HINT.overlaying}</span></div>
      <input class="ui-switch" type="checkbox" bind:checked={zeroWidth} {disabled} />
    </label>
  </div>
{:else}
  <div class="checks" role="group" aria-label="Flags">
    <label class="check" for="{id}-overlay">
      <input id="{id}-overlay" type="checkbox" data-testid="flag-overlaying" bind:checked={zeroWidth} {disabled} />
      <span><strong>Overlaying</strong><span class="hint">{FLAG_HINT.overlaying}</span></span>
    </label>
    <label class="check" for="{id}-private">
      <input
        id="{id}-private"
        type="checkbox"
        data-testid="flag-private"
        checked={visibility === 'PRIVATE'}
        onchange={(e) => (visibility = e.currentTarget.checked ? 'PRIVATE' : 'PUBLIC')}
        disabled={disabled || locked}
      />
      <span><strong>Private</strong><span class="hint">{FLAG_HINT.private}</span></span>
    </label>
    {#if locked}
      <p class="hint locked" data-testid="flag-unlisted">A moderator took this emote off the listings: people with its link can still open it and add it.</p>
    {/if}
  </div>
{/if}

<style>
  .flags { display: grid; gap: 4px; }
  .vis { display: grid; justify-items: start; gap: 8px; padding-bottom: 8px; }
  .ui-row .ui-hint { display: block; }
  /* The switch stays beside its text on phones too (the shared row wraps there). */
  .flags .ui-row { flex-wrap: nowrap; gap: 16px; }

  .checks { display: grid; gap: 2px; }
  .check { display: flex; align-items: flex-start; gap: 10px; min-height: 44px; padding: 6px 0; cursor: pointer; }
  .check input { flex: none; width: 18px; height: 18px; margin: 2px 0 0; accent-color: var(--accent); cursor: inherit; }
  .check input:disabled { cursor: default; }
  .check > span { display: grid; gap: 1px; min-width: 0; }
  .check strong { font-size: var(--t-ui, 14px); font-weight: 600; color: var(--text); }
  .hint { font-size: var(--t-small); line-height: 1.45; color: var(--muted); }
  .locked { margin: 4px 0 0 28px; }
  .check:has(input:disabled) strong { color: var(--muted); }
</style>
