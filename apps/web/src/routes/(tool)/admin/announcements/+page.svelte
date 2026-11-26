<script lang="ts">
  // Admin: the site announcement (site.announce). One line under the header for everyone, as
  // News, Maintenance or Event, with an optional link, a small emote or badge, and a start and an
  // end. The newest one in its window is the one shown; the list says which. The list is on the
  // left, the inspector on the right holds the form with a live preview of the bar (?a=<id>, or
  // ?a=new): Save changes edits one that hasn't ended, End takes it down now, one that has ended
  // is shown as it was. Every change is audited. Keyboard: arrows over the rows, Enter opens one,
  // Escape closes it.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import AnnouncementEditor from '$lib/admin/AnnouncementEditor.svelte';
  import AnnouncementPreview from '$lib/admin/AnnouncementPreview.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, date, dateTime } from '$lib/format';
  import type { AnnouncementFieldsFragment } from '$lib/gql/graphql';
  import { ACTIVE_ANNOUNCEMENT, ANNOUNCEMENTS, END_ANNOUNCEMENT } from '$lib/queries';
  import { KIND, phase, safeLink } from '$lib/shell/announcement';
  import { banner } from '$lib/shell/announcement.svelte';
  import Gate from '$lib/shop/Gate.svelte';
  import { staffMessage } from '$lib/shop/shop';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
    import { loader } from '$lib/tool/loader.svelte';
  import { notify } from '$lib/tool/notices.svelte';

  type Row = AnnouncementFieldsFragment;

  const L = loader(async ({ fetch }) => {
    const [list, active, catalog] = await Promise.all([gql(ANNOUNCEMENTS, {}, fetch), gql(ACTIVE_ANNOUNCEMENT, {}, fetch), loadCatalog().catch(() => null)]);
    return { rows: list.announcements, showing: active.activeAnnouncement?.id ?? null, catalog };
  });

  // The picked one lives in the address: an ID, or "new".
  const key = $derived(page.url.searchParams.get('a'));
  const fresh = $derived(key === 'new');
  const picked = $derived(L.data?.rows.find((r) => r.id === key) ?? null);
  let list = $state<HTMLElement>();
  let busy = $state(false);

  function pick(id: string | null) {
    void pickInUrl('a', id);
  }
  // Closing the inspector puts focus back on the row (or on New announcement).
  function close() {
    const id = key;
    pick(null);
    void tick().then(() => (list?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id ?? '')}"]`) ?? document.getElementById('ann-new'))?.focus());
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !key || e.defaultPrevented || document.querySelector('dialog[open]')) return;
    if ((e.target as HTMLElement | null)?.closest('input, textarea, select')) return;
    e.preventDefault();
    close();
  }

  async function saved(changed: boolean) {
    notify(changed ? 'Announcement changed.' : 'Announcement saved.');
    pick(null);
    await L.reload();
    banner.load(true);
  }

  async function end(row: Row) {
    busy = true;
    try {
      await gql(END_ANNOUNCEMENT, { id: row.id });
      notify('Announcement ended.');
      pick(null);
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
    const parts = [`from ${date(a.startsAt ?? a.createdAt)}`];
    if (a.endedAt) parts.push(`ended ${date(a.endedAt)}`);
    else parts.push(a.endsAt ? `until ${date(a.endsAt)}` : 'until you end it');
    return parts.join(' · ');
  }
  const editable = (a: Row) => phase(a, Date.now()) !== 'ended';
</script>

<svelte:head><title>{fresh ? 'New announcement' : picked ? 'Announcement' : 'Announcements'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Gate permission="site.announce">
  <Split label="Announcement" size="l" open={fresh || !!picked} onclose={() => pick(null)}>
    <Bar title="Announcements" count={L.data ? `${count(L.data.rows.length)} ${L.data.rows.length === 1 ? 'announcement' : 'announcements'}` : null}>
      {#snippet actions()}
        <button id="ann-new" class="ui-btn primary small" type="button" disabled={!L.data} onclick={() => pick('new')}><Icon name="plus" size={15} />New announcement</button>
      {/snippet}
    </Bar>

    <LoadGate source={L} label="Loading announcements…">
      {#snippet children(d)}
        {#if !d.rows.length}
          <State title="No announcements yet.">
            Tell everyone about maintenance, news or an event. It shows under the header until it ends.
            {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => pick('new')}>New announcement</button>{/snippet}
          </State>
        {:else}
          <ul class="adm-rows" bind:this={list} use:gridKeys aria-label="Announcements">
            {#each d.rows as a, i (a.id)}
              {@const ph = phase(a, Date.now())}
              {@const showing = a.id === d.showing}
              <li>
                <button class="item" type="button" data-item={a.id} aria-current={a.id === key ? 'true' : undefined}
                  tabindex={(picked ? a.id === key : i === 0) ? 0 : -1} onclick={() => pick(a.id)}>
                  <span class="kind" title={KIND[a.kind].label}><Icon name={KIND[a.kind].icon} size={16} /></span>
                  <span class="what">
                    <span class="line"><bdi>{a.text}</bdi></span>
                    <span class="meta adm-num">{KIND[a.kind].label} · {when(a)}</span>
                  </span>
                  <span class="st" class:on={showing} class:old={ph === 'ended'}>{#if showing}<Icon name="eye" size={14} />Showing now{:else}{PHASE[ph]}{/if}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/snippet}
    </LoadGate>

    {#snippet empty()}
      <div class="adm-ip-head"><span class="adm-label">Announcement</span></div>
      <p class="none">Pick an announcement to change or end it, or start a new one.</p>
    {/snippet}

    {#snippet panel()}
      {#if fresh || picked}
        <div class="adm-ip-head">
          <span class="adm-label">{fresh ? 'New announcement' : editable(picked!) ? 'Announcement' : 'Ended announcement'}</span>
          <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={close} />
        </div>
        <div class="adm-ip-body">
          {#if fresh}
            {#key 'new'}<AnnouncementEditor row={null} catalog={L.data?.catalog ?? null} onsaved={saved} />{/key}
          {:else if picked && editable(picked)}
            {#key picked.id}
              <AnnouncementEditor row={picked} catalog={L.data?.catalog ?? null} onsaved={saved} />
            {/key}
            <section class="adm-sec">
              <div class="adm-acts row">
                <Confirm small label="End now" confirm="Click again to end it now" {busy} onconfirm={() => end(picked!)} />
              </div>
            </section>
          {:else if picked}
            {@const b = picked.badgeId ? L.data?.catalog?.badges.get(picked.badgeId) : null}
            <section class="adm-sec">
              <h3 class="adm-label">Preview</h3>
              <AnnouncementPreview kind={picked.kind} text={picked.text} href={safeLink(picked.linkUrl)} linkLabel={picked.linkLabel ?? ''} emote={picked.emote} emoteSet={!!picked.emote} badge={picked.emote ? null : (b ?? null)} />
            </section>
            <section class="adm-sec">
              <h3 class="adm-label">Facts</h3>
              <dl class="adm-kv">
                <dt>Kind</dt><dd>{KIND[picked.kind].label}</dd>
                <dt>From</dt><dd>{dateTime(picked.startsAt ?? picked.createdAt)}</dd>
                <dt>Ended</dt><dd>{dateTime(picked.endedAt ?? picked.endsAt ?? '')}</dd>
                {#if picked.linkUrl}<dt>Link</dt><dd class="mono">{picked.linkUrl}</dd>{/if}
                {#if picked.emote}<dt>Emote</dt><dd><bdi>{clean(picked.emote.name)}</bdi></dd>{/if}
                {#if b}<dt>Badge</dt><dd><bdi>{clean(b.name)}</bdi></dd>{/if}
              </dl>
            </section>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Split>
</Gate>

<style>
  .item { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto; align-items: center; gap: 12px; width: 100%; height: 60px; padding: 0 12px;
    border: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-small); text-align: left; cursor: pointer; }
  .item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .item:active { opacity: .6; }
  @media (hover: hover) and (pointer: fine) { .item:hover { background: var(--row-hover); } }
  .item[aria-current='true'] { box-shadow: inset 0 0 0 1px var(--accent-border); }
  .item[aria-current='true'] .line { font-weight: 700; }
  .kind { display: inline-flex; color: var(--dim); }
  .item[aria-current='true'] .kind { color: var(--accent-text); }
  .what { display: grid; min-width: 0; line-height: 1.35; }
  .line { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .meta { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); color: var(--dim); }
  .st { display: inline-flex; align-items: center; gap: 6px; font-size: var(--t-small); color: var(--muted); white-space: nowrap; }
  .st.old { color: var(--dim); }
  .st.on { color: var(--text); font-weight: 700; }
  .st.on :global(svg) { color: var(--accent-text); }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  @media (max-width: 639.98px) {
    .item { grid-template-columns: 20px minmax(0, 1fr); height: auto; min-height: 64px; padding-block: 8px; }
    .st { grid-column: 2; }
  }
</style>
