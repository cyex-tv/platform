<script lang="ts">
  // An emote's editor, in the Inspector shape: the picture at its sizes and the details form on
  // the left; on the right its status, facts and id, adding it to a set, giving it to another
  // account (its owner only) and deleting it. The owner, the account's editors with Manage, and
  // staff with the matching permissions.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql, type EmoteVisibility } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import { ready, size } from '$lib/emote';
  import { clean, count, date, iso } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, DELETE_EMOTE, EMOTE, MY_SETS, TRANSFER_EMOTE, UPDATE_EMOTE } from '$lib/queries';
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
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import Section from '$lib/components/Section.svelte';
  import Row from '$lib/components/Row.svelte';
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import ContactSheet from '$lib/components/ContactSheet.svelte';
  import { reduced, watchReduced } from '$lib/cosmetics/reduced.svelte';
  import { onMount } from 'svelte';
  import { scaleFiles } from '$lib/components/emote/emote-page';
  import PageHead from '$lib/tool/PageHead.svelte';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import { visibilityOf } from '$lib/tool/visibility';

  let emote = $state<EmoteFieldsFragment | null>(null);
  let slow = $state(false);
  let sets = $state<{ id: string; name: string; kind: string }[]>([]);
  let target = $state('');

  let name = $state('');
  let tags = $state<string[]>([]);
  let zeroWidth = $state(false);
  let visibility = $state<EmoteVisibility>('PUBLIC');
  let saving = $state(false);
  let deleting = $state(false);
  let adding = $state(false);
  let grants = $state<readonly Grant[] | null>(null);
  let giveTo = $state('');
  let giving = $state(false);
  const nameError = $derived(nameProblem(name));
  const tagError = $derived(tagsProblem(tags));
  const mine = $derived(emote?.owner?.id === session.me?.id);
  // Editors of the owner's account with Manage on every set edit and delete as the owner does
  // (ADR 0014). Only the owner gives it away: nobody can ask for someone else's emote (amendment
  // 2026/11/22). Staff edit and delete anyone's emote with emotes.edit_any / emotes.delete_any.
  // The API checks and audits.
  const manages = $derived(!mine && emoteCan(grants, emote?.owner?.id, session.me?.id, 'EMOTE_MANAGE'));
  const canEdit = $derived(mine || manages || can('emotes.edit_any'));
  const canDelete = $derived(mine || manages || can('emotes.delete_any'));
  const canGive = $derived(mine);
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
        visibility !== visibilityOf(emote)),
  );

  function fill(e: EmoteFieldsFragment) {
    emote = e;
    name = e.name;
    tags = [...e.tags];
    zeroWidth = e.flags.zeroWidth;
    visibility = visibilityOf(e);
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
        input: { name: name.trim(), tags, zeroWidth, visibility },
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
  const one = $derived(emote ? size(emote) : { width: 0, height: 0 });
  const files = $derived(emote ? scaleFiles(emote.files) : []);
  const biggest = $derived(files.at(-1));
  // Animated emotes play; under reduced motion they show their still frame.
  onMount(watchReduced);
  const frozen = $derived(!!emote && emote.animated && emote.files.some((f) => f.name === '1x_static.webp') && reduced.on);
  const fileUrl = (f: { scale: number; name: string }) => (emote ? (frozen ? `${emote.host}/${f.scale}x_static.webp` : `${emote.host}/${f.name}`) : '');
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No emote here') : `${emote?.name ?? 'Emote'} - cyex.tv`}</title></svelte:head>

{#snippet side()}
  {#if emote}
    <PanelBlock title="Facts">
      <FactRows
        label="Facts"
        items={[
          { label: 'Status', value: STATE_LABEL[emote.state], tone: emote.state === 'APPROVED' ? 'ok' : emote.state === 'PENDING' ? 'warn' : 'bad' },
          { label: 'Uploaded', value: date(emote.createdAt), mono: true },
          { label: 'Id', value: emote.id, mono: true },
          { label: 'Channels', value: count(emote.channelCount), mono: true },
          ...(emote.files.length ? [{ label: 'Size at 1x', value: `${one.width} × ${one.height} px`, mono: true }] : []),
          ...(emote.owner && !mine ? [{ label: 'Owner', value: clean(emote.owner.displayName), href: ownerHome }] : []),
        ]}
      />
      <p class="pb-hint">{STATE_HINT[emote.state]}</p>
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
          <p class="pb-hint">It’s in all your sets already, or you have none yet.</p>
        {/if}
      </PanelBlock>
    {/if}

    <PanelBlock title="Go to">
      <LinkRows
        label="Related"
        items={[
          ...(emote.state === 'APPROVED' ? [{ href: `/emotes/${emote.id}`, label: 'Open public page', icon: 'arrow-right' as const }] : []),
          { href: '/me/sets', label: 'Your sets', icon: 'grid' as const },
        ]}
      />
    </PanelBlock>

    {#if grantsFailed}
      <RetryNote onretry={loadGrants}>Couldn’t load which accounts you edit, so some actions (like giving this emote away) may be missing.</RetryNote>
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
  <MeFrame title="Emote" label="About this emote" panel={side}>
    <div class="col">
      <PageHead
        name
        title={clean(emote.name)}
        back={mine ? { href: sectionHref(session.me?.id ?? '', 'uploads'), label: 'Your emotes' } : manages ? { href: ownerHome, label: `${clean(emote.owner?.displayName ?? '')}’s emotes` } : can('emotes.review') ? { href: '/admin', label: 'Admin' } : { href: '/search', label: 'Emotes' }}
      >
        {#snippet line()}
          <span class="ui-chip {emote!.state.toLowerCase()}">{STATE_LABEL[emote!.state]}</span> Uploaded <span class="ui-mono">{date(emote!.createdAt)}</span>{#if !mine && emote!.owner && manages}{' · '}<bdi>{clean(emote!.owner.displayName)}</bdi>’s emote, you edit their account{/if}
        {/snippet}
      </PageHead>

      <section class="picture" aria-label="Picture">
        {#if ready(emote) && biggest}
          <div class="stage" data-testid="emote-stage">
            <img src={fileUrl(biggest)} width={biggest.width} height={biggest.height} alt={emote.name} />
          </div>
          <ContactSheet label="Sizes" min={120}>
            {#each files as f (f.name)}
              <li class="cell">
                <span class="pic"><img src={fileUrl(f)} width={f.width} height={f.height} alt="" /></span>
                <span class="cap"><span class="x">{f.scale}x</span><span class="dm ui-mono">{f.width} × {f.height}</span></span>
              </li>
            {/each}
          </ContactSheet>
        {:else}
          <div class="stage wait"><p class="ui-hint">{slow ? 'The files are taking longer than usual. Reload in a minute.' : 'The files are still being made. This takes a few seconds.'}</p></div>
        {/if}
      </section>

      {#if canEdit}
        <form onsubmit={save} novalidate>
          <Section title="Name">
            <Row label="Name" for="edit-name" hint={nameError ? undefined : 'Shown on the emote’s page and typed in chat.'}>
              <input id="edit-name" class="ui-input name-in" bind:value={name} maxlength="100" spellcheck="false" aria-invalid={nameError ? 'true' : undefined} />
              <button class="ui-btn secondary small" type="submit" disabled={saving || name.trim() === emote.name || !!nameError || !!tagError}>Rename</button>
            </Row>
            {#if nameError}<p class="ui-error err">{nameError}</p>{/if}
          </Section>
          <Section title="Tags">
            <Row label="Tags" hint="Enter or a comma adds a tag. Up to {TAG_MAX}.">
              <div class="tags">
                <TagInput label="Tags" {tags} onchange={(t) => (tags = t)} max={TAG_MAX} clean={cleanTag} placeholder="cat, dance" disabled={saving} id="edit-tags" />
              </div>
            </Row>
            {#if tagError}<p class="ui-error err">{tagError}</p>{/if}
          </Section>
          <Section title="Flags">
            <div class="flags"><EmoteFlags id="edit" bind:zeroWidth bind:visibility disabled={saving} /></div>
            <div class="ui-actions save">
              <button class="ui-btn primary" type="submit" disabled={saving || !changed || !!nameError || !!tagError}>
                {saving ? 'Saving…' : 'Save'}
              </button>
            </div>
          </Section>
        </form>
      {/if}

      {#if canGive}
        <Section title="Hand-over" lead="Give it to another account. It moves with its name, tags and sets.">
          <div id="transfer">
            {#if receivers.length}
              <Row label="Give to" for="give-to" hint="Only accounts where you can upload can take it.">
                <select id="give-to" class="ui-input give-in" bind:value={giveTo}>
                  {#each receivers as r (r.id)}<option value={r.id}>{clean(r.displayName)}</option>{/each}
                </select>
                <Confirm small label="Give" confirm="Click again to give it" busy={giving} onconfirm={give} />
              </Row>
            {:else}
              <Row label="Give to" hint="You can give it only to accounts where you can upload; there are none yet." />
            {/if}
          </div>
        </Section>
      {/if}

      {#if canDelete}
        <Section title="Danger" tone="danger">
          <Row label="Delete this emote" hint="It leaves every set it’s in. This can’t be undone.">
            <Confirm small label="Delete emote" confirm="Click again to delete" busy={deleting} onconfirm={remove} />
          </Row>
        </Section>
      {/if}
    </div>
  </MeFrame>
{/if}
{/snippet}
</LoadGate>

<style>
  .col { max-width: 820px; min-width: 0; }
  .picture { display: grid; gap: 16px; margin-bottom: 24px; min-width: 0; }
  .stage { display: grid; place-items: center; min-height: 220px; padding: 32px 16px; border: 1px solid var(--border); background: var(--surface); min-width: 0; }
  .stage img { display: block; max-width: 100%; height: auto; max-height: 320px; object-fit: contain; }
  .wait p { margin: 0; text-align: center; }
  .cell { display: grid; grid-template-rows: 1fr auto; justify-items: center; align-items: end; gap: 10px; padding: 16px 8px 10px; }
  .pic { display: grid; place-items: end center; max-width: 100%; }
  .pic img { display: block; max-width: 100%; height: auto; }
  .cap { display: grid; justify-items: center; gap: 1px; font-size: var(--t-fine); color: var(--muted); }
  .x { font-size: var(--t-small); font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
  .dm { white-space: nowrap; }
  .picture :global(.cs) { grid-template-columns: repeat(auto-fit, minmax(min(120px, 100%), 1fr)); }
  .name-in { width: min(220px, 100%); }
  .tags { flex: 1; min-width: min(280px, 100%); text-align: left; }
  .give-in { width: min(240px, 100%); }
  .err { margin: 6px 0 0; }
  .flags { padding: 4px 0 0; }
  .save { margin-top: 12px; }
  .add { display: flex; gap: 8px; }
  .add select { flex: 1; min-width: 0; }
  @media (max-width: 559.98px) { .name-in, .give-in { width: 100%; } }
</style>
