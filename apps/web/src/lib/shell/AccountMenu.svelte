<script lang="ts">
  // The one account menu: the same trigger (avatar, name, chevron) and the same popover on every
  // page, public or signed in. A menu: the trigger says expanded or not, the panel is a
  // list of menu items (AccountLinks, shared with the phone menu sheet). Renders nothing signed out;
  // the AppHeader shows Sign in in that slot instead.
  import Avatar from '$lib/components/Avatar.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { clean } from '$lib/format';
  import { session } from '$lib/tool/session.svelte';
  import AccountLinks from './AccountLinks.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { whatsNew } from '$lib/changelog/seen.svelte';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  // After the session ends mid-use the menu stops showing the account (the page keeps it).
  const me = $derived(session.ended ? null : session.me);
  const who = $derived(me?.identities[0]);
  const name = $derived(clean(who?.displayName));
  // Your name in your paint, as everywhere else a name shows (the Inspector header).
  $effect(() => {
    if (me) wantLooks([me.id]);
  });
  const paint = $derived(me ? (looks[me.id]?.paint ?? null) : null);
</script>

{#if me}
  <Popover label="Your account" kind="menu" align="end" width="260px" bind:open>
    {#snippet trigger(props)}
      <button class="who" type="button" {...props}>
        <span class="pic"><Avatar src={who?.avatar} {name} size={24} />{#if whatsNew.unseen}<span class="dot" aria-hidden="true"></span>{/if}</span>
        <span class="sr-only">Your account: <bdi>{name}</bdi>{#if whatsNew.unseen}, something new in What’s new{/if}</span>
        <span class="name" aria-hidden="true"><PaintedName {name} {paint} /></span>
      </button>
    {/snippet}
    <AccountLinks menu onpick={() => (open = false)} />
  </Popover>
{:else if !session.known}
  <!-- Who you are isn't known yet (the first answer is on its way): the trigger's shape with a
       blank avatar, so the box isn't empty and nothing moves when the answer lands. -->
  <span class="who hold" data-account-hold aria-hidden="true"><span class="ghost"></span></span>
{/if}

<style>
  /* Flat in the bar (the Inspector header): the avatar and your painted name, a fill on hover. */
  .who { position: relative; display: flex; align-items: center; gap: 8px; height: 40px; max-width: 200px; padding: 0 10px 0 6px;
    border: 1px solid transparent; border-radius: var(--radius-btn); background: transparent; color: var(--text);
    font: inherit; font-size: var(--t-nav); font-weight: 500; cursor: pointer;
    transition: background-color var(--fast), border-color var(--fast), transform var(--dur-press) ease-out; }
  .who:active { transform: scale(var(--press)); }
  .who[aria-expanded='true'] { background: var(--surface-hover); border-color: var(--border-strong); }
  /* Something new in the changelog: a small accent dot on the avatar's corner, ringed in the bar's ground. */
  .pic { position: relative; display: inline-flex; flex: none; }
  .dot { position: absolute; top: -2px; right: -2px; width: 9px; height: 9px; border-radius: 50%; background: var(--accent);
    box-shadow: 0 0 0 2px var(--bg); }
  @media (forced-colors: active) { .dot { background: Highlight; } }
  .hold { cursor: default; pointer-events: none; }
  .ghost { flex: none; width: 24px; height: 24px; border-radius: 50%; background: var(--neutral-chip, var(--surface-2)); }
  .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who :global(svg) { color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .who:hover { background: var(--surface-hover); border-color: var(--border-strong); } }
  @media (pointer: coarse) { .who::after { content: ''; position: absolute; inset: -2px 0; } }
  @media (forced-colors: active) { .who[aria-expanded='true'] { outline: 2px solid Highlight; outline-offset: 1px; } }
  @media (max-width: 1000px) { .name { display: none; } .who { padding-right: 6px; } }
  .who.hold { width: 120px; }
  @media (max-width: 1000px) { .who.hold { width: 40px; } }
</style>
