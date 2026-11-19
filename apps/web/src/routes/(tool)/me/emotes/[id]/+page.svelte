<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql, type EmoteVisibility } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import { ready } from '$lib/emote';
  import { clean, date, iso } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, DELETE_EMOTE, EMOTE, MY_SETS, UPDATE_EMOTE } from '$lib/queries';
  import { sectionHref } from '$lib/profile/rows';
  import { TAG_MAX, cleanTag, nameProblem, tagsProblem } from '$lib/rules';
  import TagInput from '$lib/components/TagInput.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { STATE_HINT, STATE_LABEL } from '$lib/tool/state';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import EmoteFacts from '$lib/components/emote/EmoteFacts.svelte';
  import SizeRow from '$lib/components/emote/SizeRow.svelte';
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

  const nameError = $derived(nameProblem(name));
  const tagError = $derived(tagsProblem(tags));
  const mine = $derived(emote?.owner?.id === session.me?.id);
  // Staff edit and delete anyone's emote with emotes.edit_any / emotes.delete_any (the API checks and audits).
  const canEdit = $derived(mine || can('emotes.edit_any'));
  const canDelete = $derived(mine || can('emotes.delete_any'));
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

  $effect(() => {
    const id = page.params.id ?? '';
    gql(MY_SETS, {})
      .then((r) => {
        sets = r.mySets.filter((s) => s.kind !== 'GLOBAL' && !s.emotes.some((m) => m.emote.id === id));
        target = sets[0]?.id ?? '';
      })
      .catch(() => {});
  });

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!emote || nameError || tagError) return;
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
      await goto(mine || !emote.owner ? '/me' : `/admin/users/${encodeURIComponent(emote.owner.id)}`);
    } catch (err) {
      fail(err);
      deleting = false;
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
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No emote here') : `${emote?.name ?? 'Emote'} - cyex.tv`}</title></svelte:head>

<LoadGate
  source={L}
  label="Loading the emote…"
  level={1}
  home={{ href: sectionHref(session.me?.id ?? '', 'uploads'), label: 'Your emotes' }}
  missing={{ title: 'No emote here.', body: 'It may have been deleted, or it isn’t yours to see.' }}
>
{#snippet children()}
{#if emote}
  <PageHead
    name
    title={clean(emote.name)}
    back={mine ? { href: '/me', label: 'Your emotes' } : can('emotes.review') ? { href: '/admin', label: 'Admin' } : { href: '/search', label: 'Emotes' }}
  >
    {#snippet line()}
      <span class="ui-chip {emote!.state.toLowerCase()}">{STATE_LABEL[emote!.state]}</span>
      Uploaded <span>{date(emote!.createdAt)}</span>
    {/snippet}
    {#snippet actions()}
      {#if emote!.state === 'APPROVED'}
        <a class="ui-btn secondary" href="/emotes/{emote!.id}"><Icon name="arrow-right" size={15} />Public page</a>
      {/if}
    {/snippet}
  </PageHead>
  <div class="layout">
    <div class="ui-stack">
      <section class="ui-card sizes" aria-label="Picture">
        {#if ready(emote)}
          <SizeRow {emote} />
        {:else}
          <p class="ui-hint">{slow ? 'The files are taking longer than usual. Reload in a minute.' : 'The files are still being made. This takes a few seconds.'}</p>
        {/if}
      </section>
      <section class="ui-card" aria-label="Status">
        <div class="ui-row"><div><strong>State</strong><span class="ui-hint">{STATE_HINT[emote.state]}</span></div></div>
        <div class="ui-row"><div><strong>ID</strong></div><span class="ui-mono id">{emote.id}</span></div>
      </section>
      <EmoteFacts {emote} channels={emote.channelCount > 0 ? emote.channelCount : null} />
    </div>

    <div class="ui-stack">
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

      {#if emote.state !== 'REJECTED' && emote.state !== 'TAKEN_DOWN'}
        <section class="ui-card">
          <h2>Add to a set</h2>
          {#if sets.length}
            <div class="add">
              <select class="ui-input" bind:value={target} aria-label="Set">
                {#each sets as s}<option value={s.id}>{iso(s.name)} ({s.kind === 'PERSONAL' ? 'personal' : 'channel'})</option>{/each}
              </select>
              <button class="ui-btn secondary" type="button" disabled={adding} onclick={addTo}>Add</button>
            </div>
          {:else}
            <p class="ui-hint">It’s in all your sets already, or you have none yet. <a href="/me/sets">Your sets</a></p>
          {/if}
        </section>
      {/if}

      {#if canDelete}
        <section class="ui-card">
          <h2>Delete</h2>
          <p class="ui-hint">It leaves every set it’s in. This can’t be undone.</p>
          <div class="ui-actions gap">
            <Confirm label="Delete emote" confirm="Click again to delete" busy={deleting} onconfirm={remove} />
          </div>
        </section>
      {/if}
    </div>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .ui-row .ui-hint { display: block; }
  .id { font-size: .78rem; color: var(--muted); overflow-wrap: anywhere; text-align: right; }
  .add { display: flex; gap: 8px; }
  .add select { flex: 1; }
  .gap { margin-top: 12px; }
  .sizes { display: grid; place-items: center; padding: 32px 16px 24px; min-width: 0; }
  @media (max-width: 1100px) { .layout { grid-template-columns: minmax(0, 1fr); } }
</style>
