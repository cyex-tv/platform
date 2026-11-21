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
    <div class="ui-field">
      <span class="ui-label">Kind</span>
      <Segmented label="Kind" options={KINDS} value={kind} onchange={(v) => (kind = v)} size="s" />
      <span class="ui-hint">{HINT[kind]}</span>
    </div>
    <label class="ui-field">
      <span class="ui-label">Line</span>
      <input class="ui-input" bind:value={text} aria-invalid={errors.text ? 'true' : undefined} />
      {#if errors.text}<span class="ui-error">{errors.text}</span>{:else}<span class="ui-hint"><span class="ui-num">{[...text.trim()].length}</span> of {TEXT_MAX}. Plain text, one line.</span>{/if}
    </label>
  </section>
  <section class="adm-sec">
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
  </section>
  <section class="adm-sec">
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
  </section>
  <section class="adm-sec">
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
  form .adm-sec { gap: 14px; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
</style>
