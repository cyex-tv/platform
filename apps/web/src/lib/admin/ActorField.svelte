<script lang="ts">
  // The audit log's actor filter: a button with the actor's name (or "Any actor") that opens a
  // popover where you type a name or login and pick the account from the matches (the same
  // search Users uses). An account ID pasted in still works. The filter itself holds the ID.
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { clean } from '$lib/format';
  import AccountSearch from './AccountSearch.svelte';
  import { looksLikeId } from './audit';
  import { lookUp, who } from './names.svelte';

  type Props = { value: string; onchange: (id: string) => void };
  let { value, onchange }: Props = $props();

  let open = $state(false);
  let text = $state('');
  let query = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  const uid = $props.id();

  $effect(() => {
    if (value) lookUp([value]);
  });
  $effect(() => {
    if (open) text = query = '';
  });
  const name = $derived(value ? who(value) : null);
  const shown = $derived(!value ? 'Any actor' : name ? clean(name.name) : value);

  function type() {
    clearTimeout(timer);
    timer = setTimeout(() => (query = text.trim().replace(/^@/, '')), 250);
  }
  function pick(id: string) {
    open = false;
    onchange(id);
  }
</script>

<div class="af">
  <span class="ui-label">Actor</span>
  <Popover label="Find the actor" width="340px" bind:open>
    {#snippet trigger(props)}
      <button class="ui-btn secondary who" type="button" {...props} aria-label="Actor: {shown}">
        <Icon name="user" size={14} /><span class="shown" class:none={!value}><bdi>{shown}</bdi></span>
      </button>
    {/snippet}
    <div class="pop">
      <label class="sr-only" for="{uid}-q">Name or login</label>
      <input id="{uid}-q" class="ui-input" bind:value={text} oninput={type} placeholder="Name or login" autocomplete="off" spellcheck="false" maxlength="100" />
      {#if looksLikeId(text)}
        <button class="ui-btn secondary small" type="button" onclick={() => pick(text.trim().toUpperCase())}>Use this account ID</button>
      {:else if query}
        {#key query}<AccountSearch {query} onpick={(a) => pick(a.id)} />{/key}
      {/if}
      {#if value}<button class="ui-btn ghost small" type="button" onclick={() => pick('')}>Any actor</button>{/if}
    </div>
  </Popover>
</div>

<style>
  .af { display: grid; gap: 4px; }
  .who { justify-content: flex-start; gap: 6px; width: 100%; min-width: 0; }
  .who :global(svg) { flex: none; color: var(--muted); }
  .shown { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .none { color: var(--muted); font-weight: 400; }
  .pop { display: grid; gap: 8px; padding: 12px; }
</style>
