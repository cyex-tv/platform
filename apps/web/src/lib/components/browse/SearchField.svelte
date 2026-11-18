<script lang="ts">
  // The /search page's text field (emotes and people): a search icon, a clear button once there
  // is text, 16px type so phones don't zoom. `data-page-search` makes "/" focus it instead of
  // opening the palette. The page owns the text and the debounce; this only draws and reports.
  import IconButton from '$lib/components/IconButton.svelte';
  import Icon from '$lib/components/Icon.svelte';

  type Props = {
    value: string;
    input?: HTMLInputElement;
    label: string;
    placeholder: string;
    oninput: () => void;
    onsubmit: (e: SubmitEvent) => void;
    onclear: () => void;
  };
  let { value = $bindable(), input = $bindable(), label, placeholder, oninput, onsubmit, onclear }: Props = $props();
</script>

<form class="search" role="search" {onsubmit}>
  <Icon name="search" size={16} />
  <label class="sr-only" for="q">{label}</label>
  <input
    id="q"
    bind:this={input}
    bind:value
    {oninput}
    type="search"
    name="q"
    {placeholder}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    enterkeyhint="search"
    data-page-search
  />
  {#if value}
    <span class="clear"><IconButton icon="x" label="Clear search" size="s" tip="none" onclick={onclear} type="button" /></span>
  {/if}
</form>

<style>
  .search { position: relative; display: flex; align-items: center; min-width: 0; color: var(--muted); }
  .search > :global(svg) { position: absolute; left: 10px; pointer-events: none; }
  .search input { width: 100%; min-width: 0; height: 40px; padding: 0 40px 0 32px; font: inherit; font-size: 16px; color: var(--text);
    background: var(--bg); border: 1px solid var(--field-border); border-radius: var(--radius-btn);
    transition: border-color var(--fast); -webkit-appearance: none; appearance: none; }
  .search input::-webkit-search-cancel-button { display: none; }
  .search input::placeholder { color: var(--dim); }
  .search input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .clear { position: absolute; right: 4px; display: inline-flex; }
  @media (hover: hover) and (pointer: fine) { .search input:hover { border-color: var(--muted); } }
  /* Narrow: the field sits in the sticky Filters bar, beside the button, at its height. */
  :global(.bar-extra) .search input { height: 40px; background: var(--surface); }
</style>
