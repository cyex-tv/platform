<script lang="ts">
  // Admin: the site announcement (site.announce). One line under the header for everyone, as
  // News, Maintenance or Event, with an optional link, a small emote or badge, and a start and an
  // end. The newest one in its window is the one shown; the list says which. Edit changes one that
  // hasn't ended; End takes it down now. Every change is audited.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, dateTime, fromLocalInput, toLocalInput } from '$lib/format';
  import type { AnnouncementFieldsFragment, AnnouncementKind } from '$lib/gql/graphql';
  import { ACTIVE_ANNOUNCEMENT, ANNOUNCEMENTS, CREATE_ANNOUNCEMENT, END_ANNOUNCEMENT, UPDATE_ANNOUNCEMENT } from '$lib/queries';
  import { KIND, emoteIdFrom, phase, safeLink } from '$lib/shell/announcement';
  import { banner } from '$lib/shell/announcement.svelte';
  import Gate from '$lib/shop/Gate.svelte';
  import { staffMessage } from '$lib/shop/shop';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { notify } from '$lib/tool/notices.svelte';

  type Row = AnnouncementFieldsFragment;
  const TEXT_MAX = 200;

  const L = loader(async ({ fetch }) => {
    const [list, active, catalog] = await Promise.all([gql(ANNOUNCEMENTS, {}, fetch), gql(ACTIVE_ANNOUNCEMENT, {}, fetch), loadCatalog().catch(() => null)]);
    return { rows: list.announcements, showing: active.activeAnnouncement?.id ?? null, catalog };
  });

  const KINDS: { value: AnnouncementKind; label: string; icon: (typeof KIND)[AnnouncementKind]['icon'] }[] = (['INFO', 'MAINTENANCE', 'CAMPAIGN'] as const).map((k) => ({
    value: k,
    label: KIND[k].label,
    icon: KIND[k].icon,
  }));
  const HINT: Record<AnnouncementKind, string> = {
    INFO: 'News or a tip: something new on cyex.tv.',
    MAINTENANCE: 'Downtime, or something that doesn’t work right now.',
    CAMPAIGN: 'An event or a sale. Add an emote or a badge to show beside the line.',
  };

  // The form, for a new one (editing = null) or a change.
  let dialog: HTMLDialogElement;
  let editing = $state<Row | null>(null);
  let catalog = $state<Catalog | null>(null);
  let kind = $state<AnnouncementKind>('INFO');
  let text = $state('');
  let linkUrl = $state('');
  let linkLabel = $state('');
  let emote = $state('');
  let badgeId = $state('');
  let starts = $state('');
  let ends = $state('');
  let tried = $state(false);
  let saving = $state(false);
  let problem = $state<string | null>(null);
  let busy = $state(false);

  const errors = $derived({
    text: tried && (!text.trim() || text.trim().length > TEXT_MAX) ? `Write one line of 1 to ${TEXT_MAX} characters.` : null,
    link: tried && linkUrl.trim() && !safeLink(linkUrl) ? 'Use https:// or a path on cyex.tv, like /shop.' : null,
    label: tried && linkLabel.trim() && !linkUrl.trim() ? 'Add a link for these words, or leave them empty.' : null,
    emote: tried && emote.trim() && !emoteIdFrom(emote) ? 'Paste an emote’s link or ID.' : null,
    ends:
      tried && ends && Date.parse(ends) <= Date.now()
        ? 'Pick a time in the future, or leave it empty.'
        : tried && ends && starts && Date.parse(ends) <= Date.parse(starts)
          ? 'The end comes after the start.'
          : null,
  });
  const invalid = $derived(Object.values(errors).some(Boolean));

  function open(d: { catalog: Catalog | null }, row: Row | null) {
    catalog = d.catalog;
    editing = row;
    kind = row?.kind ?? 'INFO';
    text = row?.text ?? '';
    linkUrl = row?.linkUrl ?? '';
    linkLabel = row?.linkLabel ?? '';
    emote = row?.emote?.id ?? '';
    badgeId = row?.badgeId ?? '';
    starts = row?.startsAt ? toLocalInput(row.startsAt) : '';
    ends = row?.endsAt ? toLocalInput(row.endsAt) : '';
    tried = false;
    problem = null;
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (invalid) {
      dialog.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
      return;
    }
    saving = true;
    problem = null;
    const input = {
      kind,
      text: text.trim(),
      linkUrl: linkUrl.trim() || null,
      linkLabel: linkLabel.trim() || null,
      emoteId: emoteIdFrom(emote),
      badgeId: badgeId || null,
      startsAt: fromLocalInput(starts),
      endsAt: fromLocalInput(ends),
    };
    try {
      if (editing) await gql(UPDATE_ANNOUNCEMENT, { id: editing.id, input });
      else await gql(CREATE_ANNOUNCEMENT, { input });
      dialog.close();
      notify(editing ? 'Announcement changed.' : 'Announcement saved.');
      await L.reload();
      banner.load(true);
    } catch (err) {
      problem = staffMessage(err);
    } finally {
      saving = false;
    }
  }

  async function end(row: Row) {
    busy = true;
    try {
      await gql(END_ANNOUNCEMENT, { id: row.id });
      notify('Announcement ended.');
      await L.reload();
      banner.load(true);
    } catch (err) {
      notify(staffMessage(err), true);
    } finally {
      busy = false;
    }
  }

  // In its window but not shown: a newer one is up (only one shows at a time).
  const PHASE = { live: 'Behind a newer one', scheduled: 'Scheduled', ended: 'Ended' } as const;
  function when(a: Row): string {
    const parts = [a.startsAt ? `from ${dateTime(a.startsAt)}` : `from ${dateTime(a.createdAt)}`];
    if (a.endedAt) parts.push(`ended ${dateTime(a.endedAt)}`);
    else parts.push(a.endsAt ? `until ${dateTime(a.endsAt)}` : 'until you end it');
    return parts.join(' · ');
  }
  const previewHref = $derived(safeLink(linkUrl));
  const previewBadge = $derived(badgeId && catalog ? (catalog.badges.get(badgeId) ?? null) : null);
</script>

<svelte:head><title>Announcements - Admin - cyex.tv</title></svelte:head>

<Gate permission="site.announce">
  <PageHead title="Announcements" line="One line under the header on every page. People can close it on their device; a new one shows again.">
    {#snippet actions()}
      <button class="ui-btn primary" type="button" disabled={!L.data} onclick={() => L.data && open(L.data, null)}><Icon name="plus" size={15} />New announcement</button>
    {/snippet}
  </PageHead>

  <LoadGate source={L} label="Loading announcements…">
    {#snippet children(d)}
      {#if !d.rows.length}
        <State title="No announcements yet.">
          Tell everyone about maintenance, news or an event. It shows under the header until it ends.
          {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => open(d, null)}>New announcement</button>{/snippet}
        </State>
      {:else}
        <ul class="list">
          {#each d.rows as a (a.id)}
            {@const ph = phase(a, Date.now())}
            {@const showing = a.id === d.showing}
            {@const b = a.badgeId ? d.catalog?.badges.get(a.badgeId) : null}
            <li class="item">
              <div class="row">
                <span class="kind" title={KIND[a.kind].label}><Icon name={KIND[a.kind].icon} size={16} /></span>
                <span class="what">
                  <strong><bdi>{a.text}</bdi></strong>
                  <span class="ui-hint">
                    {KIND[a.kind].label} · {when(a)}{#if a.linkUrl} · <span class="ui-mono">{a.linkUrl}</span>{/if}{#if a.emote} · {clean(a.emote.name)}{/if}{#if b} · {clean(b.name)}{/if}
                  </span>
                </span>
                <span class="st" class:on={showing}>{#if showing}<Icon name="eye" size={14} />Showing now{:else}{PHASE[ph]}{/if}</span>
                {#if ph !== 'ended'}
                  <span class="acts">
                    <button class="ui-btn secondary small" type="button" onclick={() => open(d, a)}>Edit</button>
                    <Confirm small label="End" confirm="Click again to end it now" {busy} onconfirm={() => end(a)} />
                  </span>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
  </LoadGate>

  <dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="ann-title">
    <form onsubmit={save} novalidate>
      <div class="body">
        <h2 id="ann-title">{editing ? 'Change the announcement' : 'New announcement'}</h2>
        <div class="ui-field">
          <span class="ui-label" id="ann-kind">Kind</span>
          <Segmented label="Kind" options={KINDS} value={kind} onchange={(v) => (kind = v)} size="s" />
          <span class="ui-hint">{HINT[kind]}</span>
        </div>
        <label class="ui-field">
          <span class="ui-label">Line</span>
          <input class="ui-input" bind:value={text} maxlength={TEXT_MAX} aria-invalid={errors.text ? 'true' : undefined} />
          {#if errors.text}<span class="ui-error">{errors.text}</span>{:else}<span class="ui-hint"><span class="ui-num">{text.trim().length}</span> of {TEXT_MAX}. Plain text, one line.</span>{/if}
        </label>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Link</span>
            <input class="ui-input" bind:value={linkUrl} placeholder="/shop" autocomplete="off" aria-invalid={errors.link ? 'true' : undefined} />
            {#if errors.link}<span class="ui-error">{errors.link}</span>{:else}<span class="ui-hint">Optional. A path here, or https://.</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">Link words</span>
            <input class="ui-input" bind:value={linkLabel} maxlength="40" placeholder="Learn more" aria-invalid={errors.label ? 'true' : undefined} />
            {#if errors.label}<span class="ui-error">{errors.label}</span>{:else}<span class="ui-hint">Empty: “Learn more”.</span>{/if}
          </label>
        </div>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Emote</span>
            <input class="ui-input" bind:value={emote} placeholder="Emote link or ID" autocomplete="off" aria-invalid={errors.emote ? 'true' : undefined} />
            {#if errors.emote}<span class="ui-error">{errors.emote}</span>{:else}<span class="ui-hint">Optional, shown small. Approved emotes only.</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">Badge</span>
            <select class="ui-input" bind:value={badgeId} disabled={!!emote.trim()}>
              <option value="">No badge</option>
              {#each [...(catalog?.badges.values() ?? [])] as b (b.id)}<option value={b.id}>{clean(b.name)}</option>{/each}
            </select>
            <span class="ui-hint">{emote.trim() ? 'The emote shows instead.' : 'Optional, shown small.'}</span>
          </label>
        </div>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Starts</span>
            <input class="ui-input" type="datetime-local" bind:value={starts} />
            <span class="ui-hint">Empty: as soon as you save.</span>
          </label>
          <label class="ui-field">
            <span class="ui-label">Ends</span>
            <input class="ui-input" type="datetime-local" bind:value={ends} aria-invalid={errors.ends ? 'true' : undefined} />
            {#if errors.ends}<span class="ui-error">{errors.ends}</span>{:else}<span class="ui-hint">Empty: until you end it.</span>{/if}
          </label>
        </div>
        <div class="ui-field">
          <span class="ui-label">Preview</span>
          <div class="preview" aria-hidden="true">
            <span class="pk"><Icon name={KIND[kind].icon} size={15} />{KIND[kind].label}</span>
            {#if !emote.trim() && previewBadge}<BadgeIcon badge={previewBadge} size={18} />{/if}
            <span class="pt"><bdi>{text.trim() || 'Your line shows here.'}</bdi>{#if previewHref}<span class="pl">{linkLabel.trim() || 'Learn more'}<Icon name="chevron-right" size={14} /></span>{/if}</span>
            <Icon name="x" size={14} />
          </div>
        </div>
        {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      </div>
      <footer>
        <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
        <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : editing ? 'Save changes' : 'Put it up'}</button>
      </footer>
    </form>
  </dialog>
</Gate>

<style>
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; padding: 10px 14px; }
  .kind { flex: none; display: inline-flex; color: var(--dim); }
  .what { display: grid; flex: 1 1 260px; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow-wrap: anywhere; }
  .st { flex: none; display: inline-flex; align-items: center; gap: 6px; min-width: 96px; font-size: var(--t-small); color: var(--muted); }
  .st.on { color: var(--text); font-weight: 700; }
  .st.on :global(svg) { color: var(--accent-text); }
  .acts { flex: none; display: flex; gap: 8px; }
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; }
  .two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; align-items: start; }
  .preview { display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 6px 12px; border: 1px solid var(--border);
    border-radius: var(--radius); background: var(--surface); color: var(--dim); }
  .pk { flex: none; display: inline-flex; align-items: center; gap: 6px; font-size: var(--t-fine); font-weight: 600; color: var(--muted); }
  .pt { flex: 1; min-width: 0; font-size: var(--t-nav); font-weight: 500; color: var(--text); overflow-wrap: anywhere; }
  .pl { display: inline-flex; align-items: center; gap: 2px; margin-left: 8px; color: var(--accent-text); font-weight: 600; white-space: nowrap; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  @media (max-width: 520px) { .two { grid-template-columns: minmax(0, 1fr); } }
</style>
