<script lang="ts">
  // The one account menu: the same trigger (avatar, name, chevron) and the same popover on every
  // page, public or signed in. A menu: the trigger says expanded or not, the panel is a
  // list of menu items (AccountLinks, shared with the phone menu sheet). Renders nothing signed out;
  // the AppHeader shows Sign in in that slot instead.
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { clean } from '$lib/format';
  import { session } from '$lib/tool/session.svelte';
  import AccountLinks from './AccountLinks.svelte';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const who = $derived(session.me?.identities[0]);
  const name = $derived(clean(who?.displayName));
</script>

{#if session.me}
  <Popover label="Your account" kind="menu" align="end" width="260px" bind:open>
    {#snippet trigger(props)}
      <button class="who" type="button" {...props}>
        <Avatar src={who?.avatar} {name} size={24} />
        <span class="sr-only">Your account: <bdi>{name}</bdi></span>
        <span class="name" aria-hidden="true"><bdi>{name}</bdi></span>
        <Icon name="chevron-down" size={14} />
      </button>
    {/snippet}
    <AccountLinks menu onpick={() => (open = false)} />
  </Popover>
{:else if !session.known}
  <!-- Who you are isn't known yet (the first answer is on its way): the trigger's shape with a
       blank avatar, so the box isn't empty and nothing moves when the answer lands. -->
  <span class="who hold" data-account-hold aria-hidden="true"><span class="ghost"></span><Icon name="chevron-down" size={14} /></span>
{/if}

<style>
  .who { position: relative; display: flex; align-items: center; gap: 8px; height: 40px; max-width: 200px; padding: 0 8px 0 5px;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: transparent; color: var(--text);
    font: inherit; font-size: var(--t-nav); font-weight: 500; cursor: pointer;
    transition: background-color var(--fast), border-color var(--fast), transform var(--dur-press) ease-out; }
  .who:active { transform: scale(var(--press)); }
  .who[aria-expanded='true'] { background: var(--surface-hover); border-color: var(--border-strong); }
  .hold { cursor: default; pointer-events: none; }
  .ghost { flex: none; width: 24px; height: 24px; border-radius: 50%; background: var(--neutral-chip, var(--surface-2)); }
  .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who :global(svg) { color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .who:hover { background: var(--surface-hover); border-color: var(--border-strong); } }
  @media (pointer: coarse) { .who::after { content: ''; position: absolute; inset: -2px 0; } }
  @media (forced-colors: active) { .who[aria-expanded='true'] { outline: 2px solid Highlight; outline-offset: 1px; } }
  @media (max-width: 1000px) { .name { display: none; } .who { padding-right: 6px; } }
</style>
