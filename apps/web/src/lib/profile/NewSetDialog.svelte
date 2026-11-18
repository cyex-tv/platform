<script lang="ts">
  import Segmented from '$lib/components/Segmented.svelte';
  // Make a set for an account: yours, or one you edit with SETS. Opens its editor when made.
  //   <NewSetDialog bind:this={dlg} ownerId={id} mine hasPersonal={false} oncreated={…} />  dlg.open()
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import type { SetKind } from '$lib/gql/graphql';
  import { CREATE_SET } from '$lib/queries';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { forgetProfile } from './data.svelte';

  type Props = { ownerId: string; mine: boolean; hasPersonal: boolean };
  let { ownerId, mine, hasPersonal }: Props = $props();

  let dialog: HTMLDialogElement;
  let name = $state('');
  let kind = $state<SetKind>('CHANNEL');
  let creating = $state(false);
  let tried = $state(false);
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : null);

  export function open() {
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
      const r = await gql(CREATE_SET, { input: { name: trimmed, kind, ownerId: mine ? null : ownerId } });
      dialog.close();
      forgetProfile('library:');
      notify(`${trimmed} created.`);
      await goto(`/me/sets/${r.createSet.id}`);
    } catch (err) {
      fail(err);
    } finally {
      creating = false;
    }
  }
</script>

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
        <Segmented label="Kind" options={[{ value: 'CHANNEL', label: 'Channel', icon: 'tv' }, { value: 'PERSONAL', label: 'Personal', icon: 'user', disabled: hasPersonal }]} value={kind} onchange={(v) => (kind = v)} />
        <span class="ui-hint">
          {#if hasPersonal}{mine ? 'You have' : 'This account has'} a personal set already; there’s one per account.{:else if kind === 'PERSONAL'}Approved emotes only, shown wherever {mine ? 'you chat' : 'they chat'}.{:else}For a channel’s chat. Pick it as the active set in Channels.{/if}
        </span>
      </div>
    </div>
    <footer>
      <button class="btn btn-s" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="btn btn-s btn-primary" type="submit" disabled={creating}>{creating ? 'Creating…' : 'Create'}</button>
    </footer>
  </form>
</dialog>

<style>
</style>
