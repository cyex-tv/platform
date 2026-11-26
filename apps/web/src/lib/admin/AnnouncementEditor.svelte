<script lang="ts">
  // The announcement form in the inspector: kind, line, link, emote or badge, start and end, with
  // a live preview of the bar on top. For a new one (row null) or a change; saving hands back to
  // the page. Every field keeps its check (one line of 1 to 200 characters, a safe link, an emote
  // link or ID, an end in the future and after the start).
  import { tick, untrack } from 'svelte';
  import { gql } from '$lib/api';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, fromLocalInput, toLocalInput } from '$lib/format';
  import type { AnnouncementFieldsFragment, AnnouncementKind } from '$lib/gql/graphql';
  import { CREATE_ANNOUNCEMENT, UPDATE_ANNOUNCEMENT } from '$lib/queries';
  import { KIND, emoteIdFrom, safeLink } from '$lib/shell/announcement';
  import { staffMessage } from '$lib/shop/shop';
  import AnnouncementPreview from './AnnouncementPreview.svelte';

  type Row = AnnouncementFieldsFragment;
  type Props = { row: Row | null; catalog: Catalog | null; onsaved: (changed: boolean) => void };
  let { row, catalog, onsaved }: Props = $props();

  const TEXT_MAX = 200;
  const KINDS = (['INFO', 'MAINTENANCE', 'CAMPAIGN'] as const).map((k) => ({ value: k, label: KIND[k].label, icon: KIND[k].icon }));
  const HINT: Record<AnnouncementKind, string> = {
    INFO: 'News or a tip: something new on cyex.tv.',
    MAINTENANCE: 'Downtime, or something that doesn’t work right now.',
    CAMPAIGN: 'An event or a sale. Add an emote or a badge to show beside the line.',
  };

  // The form starts from the row once; the page keys this component on the row's id.
  const start = untrack(() => row);
  let kind = $state<AnnouncementKind>(start?.kind ?? 'INFO');
  let text = $state(start?.text ?? '');
  let linkUrl = $state(start?.linkUrl ?? '');
  let linkLabel = $state(start?.linkLabel ?? '');
  let emote = $state(start?.emote?.id ?? '');
  let badgeId = $state(start?.badgeId ?? '');
  let starts = $state(start?.startsAt ? toLocalInput(start.startsAt) : '');
  let ends = $state(start?.endsAt ? toLocalInput(start.endsAt) : '');
  let tried = $state(false);
  let saving = $state(false);
  let problem = $state<string | null>(null);
  let form: HTMLFormElement;

  const errors = $derived({
    text: tried && (!text.trim() || [...text.trim()].length > TEXT_MAX) ? `Write one line of 1 to ${TEXT_MAX} characters.` : null,
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

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (invalid) {
      await tick();
      form.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
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
      if (row) await gql(UPDATE_ANNOUNCEMENT, { id: row.id, input });
      else await gql(CREATE_ANNOUNCEMENT, { input });
      onsaved(!!row);
    } catch (err) {
      problem = staffMessage(err);
    } finally {
      saving = false;
    }
  }

  const badge = $derived(badgeId && catalog ? (catalog.badges.get(badgeId) ?? null) : null);
  const emoteId = $derived(emoteIdFrom(emote));
  const emotePic = $derived(row?.emote && emoteId === row.emote.id ? row.emote : null);
</script>

<form bind:this={form} onsubmit={save} novalidate>
  <section class="adm-sec">
    <h3 class="adm-label">Preview</h3>
    <AnnouncementPreview {kind} {text} href={safeLink(linkUrl)} {linkLabel} emote={emotePic} emoteSet={!!emote.trim()} badge={emote.trim() ? null : badge} />
  </section>
  <section class="adm-sec">
    <h3 class="adm-label">Line</h3>
    <div class="adm-lv">
      <span class="l">Kind</span>
      <div class="v"><Segmented label="Kind" options={KINDS} value={kind} onchange={(v) => (kind = v)} size="s" /><span class="ui-hint">{HINT[kind]}</span></div>
    </div>
    <label class="adm-lv">
      <span class="l">Text<small><span class="adm-num">{[...text.trim()].length}</span> of {TEXT_MAX}</small></span>
      <div class="v">
        <input class="ui-input" bind:value={text} aria-invalid={errors.text ? 'true' : undefined} />
        {#if errors.text}<span class="ui-error">{errors.text}</span>{/if}
      </div>
    </label>
    <label class="adm-lv">
      <span class="l">Link<small>Optional</small></span>
      <div class="v">
        <input class="ui-input" bind:value={linkUrl} placeholder="/shop" autocomplete="off" aria-invalid={errors.link ? 'true' : undefined} />
        {#if errors.link}<span class="ui-error">{errors.link}</span>{/if}
      </div>
    </label>
    <label class="adm-lv">
      <span class="l">Link words<small>Empty: Learn more</small></span>
      <div class="v">
        <input class="ui-input" bind:value={linkLabel} maxlength="40" placeholder="Learn more" aria-invalid={errors.label ? 'true' : undefined} />
        {#if errors.label}<span class="ui-error">{errors.label}</span>{/if}
      </div>
    </label>
  </section>
  <section class="adm-sec">
    <h3 class="adm-label">Beside the line</h3>
    <label class="adm-lv">
      <span class="l">Emote<small>Link or ID, approved only</small></span>
      <div class="v">
        <input class="ui-input" bind:value={emote} placeholder="Emote link or ID" autocomplete="off" aria-invalid={errors.emote ? 'true' : undefined} />
        {#if errors.emote}<span class="ui-error">{errors.emote}</span>{/if}
      </div>
    </label>
    <label class="adm-lv">
      <span class="l">Badge<small>{emote.trim() ? 'The emote shows instead' : 'Optional'}</small></span>
      <div class="v">
        <select class="ui-input" bind:value={badgeId} disabled={!!emote.trim()}>
          <option value="">No badge</option>
          {#each [...(catalog?.badges.values() ?? [])] as b (b.id)}<option value={b.id}>{clean(b.name)}</option>{/each}
        </select>
      </div>
    </label>
  </section>
  <section class="adm-sec">
    <h3 class="adm-label">Window</h3>
    <label class="adm-lv">
      <span class="l">Starts<small>Empty: when saved</small></span>
      <div class="v"><input class="ui-input" type="datetime-local" bind:value={starts} /></div>
    </label>
    <label class="adm-lv">
      <span class="l">Ends<small>Empty: until you end it</small></span>
      <div class="v">
        <input class="ui-input" type="datetime-local" bind:value={ends} aria-invalid={errors.ends ? 'true' : undefined} />
        {#if errors.ends}<span class="ui-error">{errors.ends}</span>{/if}
      </div>
    </label>
  </section>
  <section class="adm-sec">
    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    <div class="adm-acts row">
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : row ? 'Save changes' : 'Put it up'}</button>
    </div>
  </section>
</form>

<style>
  form { display: grid; }
  form .adm-sec { gap: 0; }
  form .adm-sec > .adm-label { margin-bottom: 6px; }
  .adm-lv { grid-template-columns: 112px minmax(0, 1fr); align-items: start; padding: 8px 0; }
  .adm-lv > .l { padding-top: 7px; }
  .adm-lv > .v { display: grid; gap: 4px; }
  .adm-lv > .v :global(.ui-input) { width: 100%; }
  .adm-lv > .v :global(.ui-input[type='datetime-local']) { font-family: var(--font-mono); font-size: var(--t-small); }
  @media (max-width: 639.98px) { .adm-lv { grid-template-columns: minmax(0, 1fr); } .adm-lv > .l { padding-top: 0; } }
  .problem { margin: 0 0 8px; font-size: var(--t-small); color: var(--red); }
</style>
