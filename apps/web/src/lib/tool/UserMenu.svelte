<script lang="ts">
  import { goto } from '$app/navigation';
  import { signOut } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Name from '$lib/components/Name.svelte';
  import { pop } from '$lib/motion';
  import { forgetMe, session } from './session.svelte';
  import { fail } from './notices.svelte';

  let open = $state(false);
  let root: HTMLDivElement;
  const who = $derived(session.me?.identities[0]);

  function onDocument(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (open && e.key === 'Escape') {
      open = false;
      root.querySelector<HTMLButtonElement>('.who')?.focus();
    }
  }
  async function leave() {
    open = false;
    try {
      await signOut();
      forgetMe();
      await goto('/');
    } catch (e) {
      fail(e);
    }
  }
</script>

<svelte:document onclick={onDocument} onkeydown={onKey} />

<div class="menu-root" bind:this={root} onfocusout={(e) => { if (open && !root.contains(e.relatedTarget as Node | null)) open = false; }}>
  <button class="who" type="button" aria-expanded={open} aria-controls="user-menu" onclick={() => (open = !open)}>
    <Avatar src={who?.avatar} name={who?.displayName ?? '?'} size={22} tone="tool" />
    <span class="name"><Name value={who?.displayName} /></span>
    <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
  </button>
  {#if open}
    <div class="pop" id="user-menu" in:pop={{ y: -4, from: 0.98, duration: 200 }} out:pop={{ y: -4, from: 0.98, duration: 120 }}>
      <a href="/me/account" onclick={() => (open = false)}>Account</a>
      <a href="/" onclick={() => (open = false)}>The front page</a>
      <hr />
      <button type="button" onclick={leave}>Sign out</button>
    </div>
  {/if}
</div>

<style>
  .menu-root { position: relative; }
  .who { display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 8px 0 5px; border: 1px solid var(--border-strong);
    border-radius: var(--radius); background: var(--surface); color: var(--text); cursor: pointer; font: inherit; font-weight: 600;
    transition: background-color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .who:hover { background: var(--surface-hover); } }
  .who:active { opacity: .6; transition: none; }
  .who svg { fill: none; stroke: var(--dim); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  .name { max-width: 16ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pop { position: absolute; right: 0; top: calc(100% + 6px); z-index: var(--z-pop); min-width: 200px; padding: 4px;
    background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius); box-shadow: var(--shadow-pop);
    transform-origin: top right; }
  .pop a, .pop button { display: flex; align-items: center; width: 100%; height: 30px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--text); font: inherit; text-align: left; text-decoration: none; cursor: pointer; }
  @media (hover: hover) and (pointer: fine) { .pop a:hover, .pop button:hover { background: var(--surface-hover); } }
  .pop a:focus-visible, .pop button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .pop button { color: var(--red); }
  hr { margin: 4px 0; border: 0; border-top: 1px solid var(--border); }
  @media (max-width: 480px) { .name { display: none; } }
</style>
