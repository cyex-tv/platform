<script lang="ts">
  // A small line under a control whose list didn't load: what is missing and a Retry. For a load
  // that is a convenience (the "upload as" list, the "By" editors), not for a page that failed
  // (that is Problem / State).
  let { children, onretry }: { children: import('svelte').Snippet; onretry: () => void } = $props();
</script>

<p class="ui-hint note" role="status">
  {@render children()}
  <button type="button" onclick={onretry}>Retry</button>
</p>

<style>
  .note { margin: 0; }
  button { padding: 0; border: 0; background: none; font: inherit; color: var(--text); font-weight: 500; text-decoration: underline;
    text-decoration-color: var(--border-strong); text-underline-offset: 3px; cursor: pointer; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: var(--radius-sm); }
  @media (hover: hover) and (pointer: fine) { button:hover { text-decoration-color: currentColor; } }
</style>
