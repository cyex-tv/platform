<script lang="ts">
  // An emote's editor, in the Inspector shape: the picture at its sizes and the details form on
  // the left; on the right its status, facts and id, adding it to a set, giving it to another
  // account (with open hand-over requests) and deleting it. The owner, the account's editors with
  // Manage, and staff with the matching permissions.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql, type EmoteVisibility } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import { ready, size } from '$lib/emote';
  import { clean, count, date, iso } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import {
    ACCEPT_EMOTE_TRANSFER,
    ADD_SET_EMOTE,
    CANCEL_EMOTE_TRANSFER,
    DECLINE_EMOTE_TRANSFER,
    DELETE_EMOTE,
    EMOTE,
    EMOTE_TRANSFER_REQUESTS,
    MY_SETS,
    TRANSFER_EMOTE,
    UPDATE_EMOTE,
  } from '$lib/queries';
  import type { TransferRequestFieldsFragment } from '$lib/gql/graphql';
  import { editorOf, forgetProfile, type Grant } from '$lib/profile/data.svelte';
  import { emoteCan, transferTargets } from '$lib/tool/editors';
  import { sectionHref } from '$lib/profile/rows';
  import { TAG_MAX, cleanTag, nameProblem, tagsProblem } from '$lib/rules';
  import RetryNote from '$lib/components/RetryNote.svelte';
  import { quiet } from '$lib/quiet';
  import TagInput from '$lib/components/TagInput.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { STATE_HINT, STATE_LABEL } from '$lib/tool/state';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';
  import { formats, kind } from '$lib/components/emote/emote-page';
  import SizeRow from '$lib/components/emote/SizeRow.svelte';
  import { transferLine } from '$lib/components/emote/emote-page';
  import Icon from '$lib/components/Icon.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import { visibilityOf } from '$lib/tool/visibility';

  const scales = [1, 2, 4] as const;
  let emote = $state<EmoteFieldsFragment | null>(null);
  let slow = $state(false);
  let sets = $state<{ id: string; name: string; kind: string }[]>([]);
  let target = $state('');

  let name = $state('');
  let tags = $state<string[]>([]);
  let zeroWidth = $state(false);
  let visibility = $state<EmoteVisibility>('PUBLIC');
  let nsfw = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let adding = $state(false);
  let grants = $state<readonly Grant[] | null>(null);
  let giveTo = $state('');
  let giving = $state(false);
  // Transfer requests on this emote (ADR 0014): an editor's waits for the owner to accept.
  let requests = $state<TransferRequestFieldsFragment[]>([]);
  let answering = $state<string | null>(null);
  async function loadRequests(id: string) {
    try {
      requests = (await gql(EMOTE_TRANSFER_REQUESTS, { emoteId: id })).emoteTransferRequests;
    } catch {
      requests = [];
    }
  }
  $effect(() => {
    const id = page.params.id ?? '';
    if (session.me && id) loadRequests(id);
  });

  const nameError = $derived(nameProblem(name));
  const tagError = $derived(tagsProblem(tags));
  const mine = $derived(emote?.owner?.id === session.me?.id);
  // Editors of the owner's account with Manage on every set edit and delete as the owner does, and
  // with Transfer give it away (ADR 0014). Staff edit and delete anyone's emote with
  // emotes.edit_any / emotes.delete_any. The API checks and audits.
  const manages = $derived(!mine && emoteCan(grants, emote?.owner?.id, session.me?.id, 'EMOTE_MANAGE'));
  const canEdit = $derived(mine || manages || can('emotes.edit_any'));
  const canDelete = $derived(mine || manages || can('emotes.delete_any'));
  const canGive = $derived(emoteCan(grants, emote?.owner?.id, session.me?.id, 'EMOTE_TRANSFER'));
  const me = $derived(session.me ? { id: session.me.id, displayName: session.me.identities[0]?.displayName ?? 'You' } : null);
  const receivers = $derived(transferTargets(me, grants, emote?.owner?.id));
  $effect(() => {
    if (!receivers.some((r) => r.id === giveTo)) giveTo = receivers[0]?.id ?? '';
  });
  let grantsFailed = $state(false);
  function loadGrants() {
    grantsFailed = false;
    void quiet(editorOf().then((g) => (grants = g)), 'editor grants', () => (grantsFailed = true));
  }
  $effect(() => {
    if (session.me) loadGrants();
  });
  const ownerHome = $derived(emote?.owner ? `/users/${encodeURIComponent(emote.owner.id)}` : '/me');
  const changed = $derived(
    emote !== null &&
      (name.trim() !== emote.name ||
        [...tags].sort().join(',') !== [...emote.tags].sort().join(',') ||
        zeroWidth !== emote.flags.zeroWidth ||
        visibility !== visibilityOf(emote) ||
        nsfw !== emote.flags.nsfw),
  );

  function fill(e: EmoteFieldsFragment) {
    emote = e;
    name = e.name;
    tags = [...e.tags];
    zeroWidth = e.flags.zeroWidth;
    visibility = visibilityOf(e);
    nsfw = e.flags.nsfw;
  }

  const L = loader(async ({ fetch }) => {
    const id = page.params.id ?? '';
    slow = false;
    const r = await gql(EMOTE, { id }, fetch);
    if (!r.emote) throw new ApiError('This emote doesn’t exist, or isn’t yours to see.', 'not_found');
    if (id === page.params.id) fill(r.emote);
    return r.emote;
  });

  // While the worker is still making the files, look again every two seconds (for a minute).
  $effect(() => {
    if (!emote || ready(emote)) return;
    const id = emote.id;
    let tries = 0;
    const timer = setInterval(async () => {
      tries += 1;
      try {
        const r = await gql(EMOTE, { id });
        if (r.emote && ready(r.emote) && emote?.id === id) {
          emote.host = r.emote.host;
          emote.files = r.emote.files;
        }
      } catch {}
      if (tries >= 30) {
        clearInterval(timer);
        if (emote?.id === id && !ready(emote)) slow = true;
      }
    }, 2000);
    return () => clearInterval(timer);
  });

  let setsFailed = $state(false);
  function loadSets() {
    const id = page.params.id ?? '';
    setsFailed = false;
    void quiet(
      gql(MY_SETS, {}).then((r) => {
        sets = r.mySets.filter((s) => s.kind !== 'GLOBAL' && !s.emotes.some((m) => m.emote.id === id));
        target = sets[0]?.id ?? '';
      }),
      'your sets',
      () => (setsFailed = true),
    );
  }
  $effect(() => {
    void page.params.id;
    loadSets();
  });

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!emote || nameError || tagError || saving) return;
    saving = true;
    try {
      const r = await gql(UPDATE_EMOTE, {
        id: emote.id,
        input: { name: name.trim(), tags, zeroWidth, visibility, nsfw },
      });
      fill(r.updateEmote);
      notify('Saved.');
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!emote) return;
    deleting = true;
    try {
      await gql(DELETE_EMOTE, { id: emote.id });
      notify(`${iso(emote.name)} deleted.`);
      await goto(mine || !emote.owner ? '/me' : manages ? ownerHome : `/admin/users/${encodeURIComponent(emote.owner.id)}`);
    } catch (err) {
      fail(err);
      deleting = false;
    }
  }

  async function give() {
    if (!emote || !giveTo || giving) return;
    const to = receivers.find((r) => r.id === giveTo);
    giving = true;
    try {
      const r = await gql(TRANSFER_EMOTE, { id: emote.id, toAccountId: giveTo });
      if (!mine) {
        // An editor's transfer waits for the owner.
        notify(`Asked ${iso(clean(emote.owner?.displayName ?? 'the owner'))} to accept. ${iso(emote.name)} moves once they do.`);
        await loadRequests(emote.id);
        return;
      }
      forgetProfile('library');
      notify(`${iso(emote.name)} now belongs to ${to ? (to.self ? 'you' : iso(clean(to.displayName))) : 'that account'}.`);
      if (emoteCan(grants, r.transferEmote.owner?.id, session.me?.id, 'EMOTE_MANAGE')) fill(r.transferEmote);
      else await goto(to?.self ? '/me' : `/users/${encodeURIComponent(giveTo)}`);
    } catch (err) {
      fail(err);
    } finally {
      giving = false;
    }
  }

  async function answer(r: TransferRequestFieldsFragment, how: 'accept' | 'decline' | 'cancel') {
    answering = r.id;
    try {
      if (how === 'accept') {
        const done = await gql(ACCEPT_EMOTE_TRANSFER, { id: r.id });
        forgetProfile('library');
        notify(`${iso(r.emoteName)} now belongs to ${iso(clean(r.to.displayName))}.`);
        if (emoteCan(grants, done.acceptEmoteTransfer.owner?.id, session.me?.id, 'EMOTE_MANAGE')) fill(done.acceptEmoteTransfer);
        else return void (await goto('/me'));
      } else if (how === 'decline') {
        await gql(DECLINE_EMOTE_TRANSFER, { id: r.id });
        notify('Declined. It stays yours.');
      } else {
        await gql(CANCEL_EMOTE_TRANSFER, { id: r.id });
        notify('Request withdrawn.');
      }
      await loadRequests(r.emoteId);
    } catch (err) {
      fail(err);
      await loadRequests(r.emoteId);
    } finally {
      answering = null;
    }
  }

  async function addTo() {
    if (!emote || !target) return;
    adding = true;
    try {
      await gql(ADD_SET_EMOTE, { setId: target, emoteId: emote.id });
      const set = sets.find((s) => s.id === target);
      notify(`Added to ${set ? iso(set.name) : 'the set'}.`);
      sets = sets.filter((s) => s.id !== target);
      target = sets[0]?.id ?? '';
    } catch (err) {
      fail(err);
    } finally {
      adding = false;
    }
  }
  const kinds = $derived(emote ? formats(emote.files) : []);
  const one = $derived(emote ? size(emote) : { width: 0, height: 0 });
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No emote here') : `${emote?.name ?? 'Emote'} - cyex.tv`}</title></svelte:head>

{#snippet side()}
  {#if emote}
    <PanelBlock title="Status">
      <p class="state"><span class="ui-chip {emote.state.toLowerCase()}">{STATE_LABEL[emote.state]}</span></p>
      <p class="pb-hint">{STATE_HINT[emote.state]}</p>
    </PanelBlock>
    <PanelBlock title="Facts">
      <dl class="kv">
        {#if emote.channelCount > 0}<dt>Channels using it</dt><dd>{count(emote.channelCount)}</dd>{/if}
        <dt>Type</dt><dd>{kind(emote)}</dd>
        {#if kinds.length}<dt>{kinds.length > 1 ? 'Formats' : 'Format'}</dt><dd>{kinds.join(', ')}</dd>{/if}
        {#if emote.files.length}<dt>Size at 1x</dt><dd>{one.width} × {one.height} px</dd>{/if}
        <dt>Uploaded</dt><dd><time datetime={emote.createdAt}>{date(emote.createdAt)}</time></dd>
        {#if emote.owner && !mine}<dt>Owner</dt><dd><a class="who" href={ownerHome}><bdi>{clean(emote.owner.displayName)}</bdi></a></dd>{/if}
        <dt>ID</dt><dd class="ui-mono id">{emote.id}</dd>
      </dl>
    </PanelBlock>

    {#if emote.state !== 'REJECTED' && emote.state !== 'TAKEN_DOWN'}
      <PanelBlock title="Add to a set">
        {#if sets.length}
          <div class="add">
            <select class="ui-input" bind:value={target} aria-label="Set">
              {#each sets as s}<option value={s.id}>{iso(s.name)} ({s.kind === 'PERSONAL' ? 'personal' : 'channel'})</option>{/each}
            </select>
            <button class="ui-btn secondary" type="button" disabled={adding} onclick={addTo}>Add</button>
          </div>
        {:else if setsFailed}
          <RetryNote onretry={loadSets}>Couldn’t load your sets.</RetryNote>
        {:else}
          <p class="pb-hint">It’s in all your sets already, or you have none yet. <a href="/me/sets">Your sets</a></p>
        {/if}
      </PanelBlock>
    {/if}

    {#if grantsFailed}
      <RetryNote onretry={loadGrants}>Couldn’t load which accounts you edit, so some actions (like giving this emote away) may be missing.</RetryNote>
    {/if}

    {#if (canGive && (receivers.length || !mine)) || requests.length}
      <PanelBlock title="Give to another account">
        <div id="transfer" class="give">
          {#if requests.length}
            <ul class="requests">
              {#each requests as r (r.id)}
                {@const owner = r.from.id === session.me?.id}
                {@const line = transferLine(r, session.me?.id)}
                <li>
                  <p>
                    {#if line.asker === 'you'}You asked{:else}<bdi>{clean(line.asker.name)}</bdi> asks{/if}
                    to give it to {#if typeof line.target === 'string'}{line.target}{:else}<bdi>{clean(line.target.name)}</bdi>{/if}.
                    <span class="ui-hint">{owner ? 'It moves only if you accept' : `Waiting for ${clean(r.from.displayName)} to accept`}; open until <span class="ui-num">{date(r.expiresAt)}</span>.</span>
                  </p>
                  <div class="ui-actions">
                    {#if owner}
                      <Confirm small label="Accept" confirm="Click again to give it away" busy={answering === r.id} onconfirm={() => answer(r, 'accept')} />
                      <button class="ui-btn secondary small" type="button" disabled={answering === r.id} onclick={() => answer(r, 'decline')}>Decline</button>
                    {:else if r.requestedBy.id === session.me?.id}
                      <button class="ui-btn secondary small" type="button" disabled={answering === r.id} onclick={() => answer(r, 'cancel')}>Cancel request</button>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
          {#if !canGive || (!mine && requests.length)}
            <!-- One open request per emote: the editor waits, or cancels it first. -->
          {:else if receivers.length}
            <p class="pb-hint">It moves with its name, tags and sets. Only you, or accounts where you can upload, can take it.{mine ? '' : ` ${clean(emote.owner?.displayName ?? 'The owner')} has to accept first.`}</p>
            <div class="add">
              <select class="ui-input" bind:value={giveTo} aria-label="Give to">
                {#each receivers as r (r.id)}<option value={r.id}>{r.self ? `${clean(r.displayName)} (you)` : clean(r.displayName)}</option>{/each}
              </select>
              <Confirm label="Give" confirm="Click again to give it" busy={giving} onconfirm={give} />
            </div>
          {:else}
            <p class="pb-hint">You can give it only to yourself or to accounts where you can upload; there are none yet.</p>
          {/if}
        </div>
      </PanelBlock>
    {/if}

    {#if canDelete}
      <PanelBlock title="Delete">
        <p class="pb-hint">It leaves every set it’s in. This can’t be undone.</p>
        <div class="pb-actions"><Confirm label="Delete emote" confirm="Click again to delete" busy={deleting} onconfirm={remove} /></div>
      </PanelBlock>
    {/if}
  {/if}
{/snippet}

<LoadGate
  source={L}
  label="Loading the emote…"
  level={1}
  home={{ href: sectionHref(session.me?.id ?? '', 'uploads'), label: 'Your emotes' }}
  missing={{ title: 'No emote here.', body: 'It may have been deleted, or it isn’t yours to see.' }}
>
{#snippet children()}
{#if emote}
  <InspectorLayout title="Emote" label="About this emote" panel={side} narrow="after" testid="emote-panel">
    {#snippet head()}
      <PageHead
        name
        title={clean(emote!.name)}
        back={mine ? { href: sectionHref(session.me?.id ?? '', 'uploads'), label: 'Your emotes' } : manages ? { href: ownerHome, label: `${clean(emote!.owner?.displayName ?? '')}’s emotes` } : can('emotes.review') ? { href: '/admin', label: 'Admin' } : { href: '/search', label: 'Emotes' }}
      >
        {#snippet line()}
          Uploaded <span>{date(emote!.createdAt)}</span>{#if !mine && emote!.owner && (manages || canGive)}{' · '}<bdi>{clean(emote!.owner.displayName)}</bdi>’s emote, you edit their account{/if}
        {/snippet}
        {#snippet actions()}
          {#if emote!.state === 'APPROVED'}
            <a class="ui-btn secondary" href="/emotes/{emote!.id}"><Icon name="arrow-right" size={15} />Public page</a>
          {/if}
        {/snippet}
      </PageHead>
    {/snippet}
    <div class="ui-stack main">
      <section class="ui-card sizes" aria-label="Picture">
        {#if ready(emote)}
          <SizeRow {emote} />
        {:else}
          <p class="ui-hint">{slow ? 'The files are taking longer than usual. Reload in a minute.' : 'The files are still being made. This takes a few seconds.'}</p>
        {/if}
      </section>
      {#if canEdit}
        <form class="ui-card ui-form" onsubmit={save} novalidate>
          <h2>Details and visibility</h2>
          <label class="ui-field">
            <span class="ui-label">Name</span>
            <input class="ui-input" bind:value={name} maxlength="100" spellcheck="false" aria-invalid={nameError ? 'true' : undefined} />
            {#if nameError}<span class="ui-error">{nameError}</span>{/if}
          </label>
          <div class="ui-field">
            <TagInput label="Tags" showLabel {tags} onchange={(t) => (tags = t)} max={TAG_MAX} clean={cleanTag}
              placeholder="cat, dance" hint="Enter or a comma adds a tag. Up to {TAG_MAX}." disabled={saving} id="edit-tags" />
            {#if tagError}<span class="ui-error">{tagError}</span>{/if}
          </div>
          <EmoteFlags id="edit" bind:zeroWidth bind:visibility bind:nsfw disabled={saving} />
          <div class="ui-actions">
            <button class="ui-btn primary" type="submit" disabled={saving || !changed || !!nameError || !!tagError}>
              {saving ? 'Saving…' : 'Save'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </InspectorLayout>
{/if}
{/snippet}
</LoadGate>

<style>
  .main { max-width: 820px; }
  .sizes { display: grid; place-items: center; padding: 32px 16px 24px; min-width: 0; }
  .state { display: flex; }
  .who { color: var(--text); }
  .id { font-size: .78rem; font-weight: 500; color: var(--muted); }
  .add { display: flex; gap: 8px; }
  .add select { flex: 1; min-width: 0; }
  .give { display: grid; gap: 10px; }
  .requests { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  .requests li { display: grid; gap: 8px; padding-bottom: 10px; border-bottom: 1px solid var(--border); }
  .requests li:last-child { border-bottom: 0; padding-bottom: 0; }
  .requests p { margin: 0; font-size: var(--t-small); overflow-wrap: anywhere; }
  .requests .ui-hint { display: block; }
</style>
