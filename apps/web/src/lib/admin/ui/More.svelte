<script lang="ts">
  // "Show all 12" under a list that grows, and "Show fewer" to fold it again. Nothing when
  // everything already fits in `limit` rows.
  import { count } from '$lib/format';

  type Props = { total: number; limit: number; expanded: boolean };
  let { total, limit, expanded = $bindable() }: Props = $props();
</script>

{#if total > limit}
  <div class="more">
    <span class="ui-hint ui-num" aria-live="polite">{count(expanded ? total : limit)} of {count(total)}</span>
    <button class="ui-btn ghost small" type="button" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>{expanded ? 'Show fewer' : `Show all ${count(total)}`}</button>
  </div>
{/if}

<style>
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 44px; padding: 0 8px 0 16px; border-top: 1px solid var(--border); }
</style>
