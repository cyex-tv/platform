<script lang="ts">
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import { count, iso } from '$lib/format';
  import Name from '$lib/components/Name.svelte';
  import type { MySetsQuery, SetKind } from '$lib/gql/graphql';
  import { CREATE_SET, MY_SETS } from '$lib/queries';
  import Thumb from '$lib/tool/Thumb.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import State from '$lib/components/State.svelte';

  const L = loader(async ({ fetch }) => (await gql(MY_SETS, {}, fetch)).mySets);
  const sets = $derived(L.data);
  let dialog: HTMLDialogElement;
  let name = $state('');
  let kind = $state<SetKind>('CHANNEL');
  let creating = $state(false);
  let tried = $state(false);
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : null);

  const owned = $derived(sets?.filter((s) => s.owner?.id === session.me?.id) ?? []);
  const edited = $derived(sets?.filter((s) => s.owner?.id !== session.me?.id) ?? []);
  const hasPersonal = $derived(owned.some((s) => s.kind === 'PERSONAL'));

  function open() {
    name = '';
    tried = false;
    kind = 'CHANNEL';
    dialog.showModal();
  }

  async function create(e: SubmitEvent) {
    e.preventDefault();
    if (creating) return;
    tried = true;
    const trimmed = name.trim();
    if (!trimmed) return;
    creating = true;
    try {
      const r = await gql(CREATE_SET, { input: { name: trimmed, kind } });
      dialog.close();
      notify(`${trimmed} created.`);
      await goto(`/me/sets/${r.createSet.id}`);
    } catch (err) {
      fail(err);
    } finally {
      creating = false;
    }
  }
</script>

<svelte:head><title>Your sets - cyex.tv</title></svelte:head>

<PageHead title="Sets" line="A channel shows one set to everyone in its chat. Your personal set goes with you to every channel that allows it.">
  {#snippet actions()}
    <button class="ui-btn primary" type="button" onclick={open}><Icon name="plus" size={15} />New set</button>
  {/snippet}
</PageHead>

{#snippet table(items: MySetsQuery['mySets'], title: string)}
  <section class="ui-panel">
    <header><h2>{title}</h2><span class="ui-hint ui-num">{items.length}</span></header>
    <ul class="ui-list">
      {#each items as s (s.id)}
        <li>
          <a class="set" href="/me/sets/{s.id}">
            <span class="peek" aria-hidden="true">
              {#each s.emotes.slice(0, 4) as m}<Thumb emote={m.emote} name="" box={28} />{/each}
            </span>
            <span class="what">
              <strong><Name value={s.name} /></strong>
              <span class="ui-hint">{s.kind === 'PERSONAL' ? 'Personal' : s.kind === 'GLOBAL' ? 'Global' : 'Channel'}{s.owner && s.owner.id !== session.me?.id ? ` · by ${iso(s.owner.displayName)}` : ''}</span>
            </span>
            <span class="ui-num ui-hint">{count(s.emotes.length)} / {count(s.capacity)}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/snippet}

<LoadGate source={L} label="Loading your sets…">
{#snippet children(sets)}
{#if sets.length === 0}
  <State title="No sets yet">
    Make one for your channel, then pick it in Channels.
    {#snippet actions()}<button class="ui-btn primary" type="button" onclick={open}><Icon name="plus" size={15} />New set</button>{/snippet}
  </State>
{:else}
  <div class="ui-stack">
    {#if owned.length}{@render table(owned, 'Yours')}{/if}
    {#if edited.length}{@render table(edited, 'You edit')}{/if}
  </div>
{/if}
{/snippet}
</LoadGate>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="new-set-title">
  <form onsubmit={create} novalidate>
    <div class="body">
      <h2 id="new-set-title">New set</h2>
      <label class="ui-field">
        <span class="ui-label">Name</span>
        <input class="ui-input" bind:value={name} maxlength="100" placeholder="Main" aria-invalid={nameError ? 'true' : undefined} aria-describedby={nameError ? 'new-set-err' : undefined} />
        {#if nameError}<span class="ui-error" id="new-set-err">{nameError}</span>{/if}
      </label>
      <div class="ui-field">
        <span class="ui-label" id="kind-label">Kind</span>
        <div class="ui-seg" role="group" aria-labelledby="kind-label">
          <button type="button" aria-pressed={kind === 'CHANNEL'} onclick={() => (kind = 'CHANNEL')}>Channel</button>
          <button type="button" aria-pressed={kind === 'PERSONAL'} disabled={hasPersonal} onclick={() => (kind = 'PERSONAL')}>Personal</button>
        </div>
        <span class="ui-hint">
          {#if hasPersonal}You have a personal set already; there’s one per account.{:else if kind === 'PERSONAL'}Approved emotes only, shown wherever you chat.{:else}For a channel’s chat. Pick it as the active set in Channels.{/if}
        </span>
      </div>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={creating}>{creating ? 'Creating…' : 'Create'}</button>
    </footer>
  </form>
</dialog>

<style>
  .set { display: flex; align-items: center; gap: 14px; flex: 1; min-width: 0; color: var(--text); text-decoration: none; padding: 4px 0; }
  .set:active { opacity: .6; }
  .peek { display: flex; gap: 2px; width: 118px; flex: none; }
  .what { display: grid; min-width: 0; flex: 1; }
  .what strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ui-seg button:disabled { opacity: .45; cursor: not-allowed; }
  @media (max-width: 520px) { .peek { display: none; } }
</style>
