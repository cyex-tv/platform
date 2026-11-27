<script lang="ts">
  // An emote's two creator flags (ADR 0039): Overlaying (zero-width) and Private (only their own
  // channel). Nobody chooses listed or unlisted: every emote is unlisted until a moderator approves
  // its public listing, so `visibility` here is only PRIVATE or not (PUBLIC means "not private").
  // NSFW is no flag: NSFW emotes aren't allowed on cyex.tv.
  //   <EmoteFlags bind:zeroWidth bind:visibility id="f1" />
  import type { EmoteVisibility } from '$lib/api';
  import { FLAG_HINT } from './visibility';

  type Props = { zeroWidth: boolean; visibility: EmoteVisibility; id: string; disabled?: boolean };
  let { zeroWidth = $bindable(), visibility = $bindable(), id, disabled = false }: Props = $props();
</script>

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
      {disabled}
    />
    <span><strong>Private</strong><span class="hint">{FLAG_HINT.private}</span></span>
  </label>
</div>

<style>
  .checks { display: grid; gap: 2px; }
  .check { display: flex; align-items: flex-start; gap: 10px; min-height: 44px; padding: 6px 0; cursor: pointer; }
  .check input { flex: none; width: 18px; height: 18px; margin: 2px 0 0; accent-color: var(--accent); cursor: inherit; }
  .check input:disabled { cursor: default; }
  .check > span { display: grid; gap: 1px; min-width: 0; }
  .check strong { font-size: var(--t-ui, 14px); font-weight: 600; color: var(--text); }
  .hint { font-size: var(--t-small); line-height: 1.45; color: var(--muted); }
  .check:has(input:disabled) strong { color: var(--muted); }
</style>
