<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import { ready, src } from '$lib/emote';
  import { clean, date } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, DELETE_EMOTE, EMOTE, MY_SETS, UPDATE_EMOTE } from '$lib/queries';
  import { nameProblem, parseTags, tagsProblem } from '$lib/rules';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { STATE_HINT, STATE_LABEL } from '$lib/tool/state';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const scales = [1, 2, 4] as const;
  let emote = $state<EmoteFieldsFragment | null>(null);
  let slow = $state(false);
  let sets = $state<{ id: string; name: string; kind: string }[]>([]);
  let target = $state('');

  let name = $state('');
  let tagText = $state('');
  let zeroWidth = $state(false);
  let hidden = $state(false);
  let nsfw = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let adding = $state(false);

  const nameError = $derived(nameProblem(name));
  const tags = $derived(parseTags(tagText));
  const tagError = $derived(tagsProblem(tags));
  const mine = $derived(emote?.owner?.id === session.me?.id);
  const changed = $derived(
    emote !== null &&
      (name.trim() !== emote.name ||
        tags.join(',') !== emote.tags.join(',') ||
        zeroWidth !== emote.flags.zeroWidth ||
        hidden !== emote.flags.private ||
        nsfw !== emote.flags.nsfw),
  );

  function fill(e: EmoteFieldsFragment) {
    emote = e;
    name = e.name;
    tagText = e.tags.join(', ');
    zeroWidth = e.flags.zeroWidth;
    hidden = e.flags.private;
    nsfw = e.flags.nsfw;
  }

  const L = loader(async ({ fetch }) => {
    const id = page.params.id ?? '';
    slow = false;
    const r = await gql(EMOTE, { id }, fetch);
    if (!r.emote) throw new Error('This emote doesn’t exist, or isn’t yours to see.');
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
        input: { name: name.trim(), tags, zeroWidth, private: hidden, nsfw },
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
      notify(`${emote.name} deleted.`);
      await goto('/me');
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
      notify(`Added to ${set?.name ?? 'the set'}.`);
      sets = sets.filter((s) => s.id !== target);
      target = sets[0]?.id ?? '';
    } catch (err) {
      fail(err);
    } finally {
      adding = false;
    }
  }
</script>

<svelte:head><title>{emote?.name ?? 'Emote'} - cyex.tv</title></svelte:head>

<LoadGate source={L} label="Loading the emote…">
{#snippet children()}
{#if emote}
  <p class="crumbs">
    {#if mine}<a href="/me">Your emotes</a>{:else if can('emotes.review')}<a href="/admin">Admin</a>{:else}<a href="/search">Emotes</a>{/if}
    <span aria-hidden="true">/</span> {clean(emote.name)}
  </p>
  <div class="layout">
    <section class="ui-card" aria-label="Preview">
      <div class="stage">
        {#if ready(emote)}
          {#each scales as s}<img src={src(emote, s)} alt={s === 4 ? emote.name : ''} />{/each}
        {:else}
          <span class="ui-spinner" aria-hidden="true"></span>
          <p class="ui-hint">{slow ? 'The files are taking longer than usual. Reload in a minute.' : 'The files are still being made. This takes a few seconds.'}</p>
        {/if}
      </div>
      <div>
        <div class="ui-row">
          <div><strong>State</strong><span class="ui-hint">{STATE_HINT[emote.state]}</span></div>
          <span class="ui-chip {emote.state.toLowerCase()}">{STATE_LABEL[emote.state]}</span>
        </div>
        <div class="ui-row"><div><strong>Uploaded</strong></div><span class="ui-mono">{date(emote.createdAt)}</span></div>
        <div class="ui-row"><div><strong>ID</strong></div><span class="ui-mono id">{emote.id}</span></div>
        {#if emote.state === 'APPROVED'}
          <div class="ui-row"><div><strong>Public page</strong></div><a href="/emotes/{emote.id}">Open</a></div>
        {/if}
      </div>
    </section>

    <div class="ui-stack">
      {#if mine}
        <form class="ui-card ui-form" onsubmit={save} novalidate>
          <h2>Details</h2>
          <label class="ui-field">
            <span class="ui-label">Name</span>
            <input class="ui-input" bind:value={name} maxlength="100" spellcheck="false" aria-invalid={nameError ? 'true' : undefined} />
            {#if nameError}<span class="ui-error">{nameError}</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">Tags</span>
            <input class="ui-input" bind:value={tagText} placeholder="cat, dance" aria-invalid={tagError ? 'true' : undefined} />
            {#if tagError}<span class="ui-error">{tagError}</span>{/if}
          </label>
          <div>
            <label class="ui-row">
              <div><strong>Zero-width</strong><span class="ui-hint">Drawn over the emote before it, like a hat.</span></div>
              <input class="ui-switch" type="checkbox" bind:checked={zeroWidth} />
            </label>
            <label class="ui-row">
              <div><strong>Unlisted</strong><span class="ui-hint">Left out of search. Sets that have it keep it.</span></div>
              <input class="ui-switch" type="checkbox" bind:checked={hidden} />
            </label>
            <label class="ui-row">
              <div><strong>Not safe for work</strong><span class="ui-hint">Hidden from search unless people ask for it.</span></div>
              <input class="ui-switch" type="checkbox" bind:checked={nsfw} />
            </label>
          </div>
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
                {#each sets as s}<option value={s.id}>{s.name} ({s.kind === 'PERSONAL' ? 'personal' : 'channel'})</option>{/each}
              </select>
              <button class="ui-btn secondary" type="button" disabled={adding} onclick={addTo}>Add</button>
            </div>
          {:else}
            <p class="ui-hint">It’s in all your sets already, or you have none yet. <a href="/me/sets">Your sets</a></p>
          {/if}
        </section>
      {/if}

      {#if mine}
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
  .crumbs { margin-bottom: 16px; min-width: 0; overflow-wrap: anywhere; color: var(--dim); font-size: var(--text-sub); }
  .crumbs a { color: var(--muted); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .crumbs a:hover { color: var(--text); } }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .stage { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: center; gap: 24px; min-height: 200px;
    padding: 24px 0; border-bottom: 1px solid var(--border); text-align: center; }
  .stage img { max-width: 100%; height: auto; }
  .stage .ui-hint { flex-basis: 100%; }
  .ui-row .ui-hint { display: block; }
  .id { font-size: .78rem; color: var(--muted); overflow-wrap: anywhere; text-align: right; }
  .add { display: flex; gap: 8px; }
  .add select { flex: 1; }
  .gap { margin-top: 12px; }
  @media (max-width: 900px) { .layout { grid-template-columns: minmax(0, 1fr); } }
</style>
