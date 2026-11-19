<script lang="ts">
  // Staff notes on an account: newest first, with author and time. Only staff with users.notes
  // see them; the account never does. You can delete your own notes.
  import { gql } from '$lib/api';
  import { ADD_STAFF_NOTE, DELETE_STAFF_NOTE, STAFF_NOTES } from '$lib/queries';
  import NoteList from './NoteList.svelte';

  let { accountId }: { accountId: string } = $props();
  let shown = $state(0);
</script>

<section class="ui-panel" aria-labelledby="sn-h">
  <header><h2 id="sn-h">Staff notes</h2>{#if shown}<span class="ui-hint ui-num">{shown}</span>{/if}</header>
  <NoteList
    key={accountId}
    load={async () => (await gql(STAFF_NOTES, { accountId })).staffNotes}
    add={async (body) => (await gql(ADD_STAFF_NOTE, { accountId, body })).addStaffNote}
    remove={(id) => gql(DELETE_STAFF_NOTE, { id })}
    placeholder="A note only staff see: context, warnings, what was agreed."
    oncount={(n) => (shown = n)}
  />
</section>
