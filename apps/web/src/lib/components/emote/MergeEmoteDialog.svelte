<script lang="ts">
  // "Merge into another emote" (ADR 0040), for staff with emotes.merge. Two steps in one pop-up:
  //   1 the target: a field taking an emote page link (any origin) or a bare ID; Next asks
  //     emoteMergePreview, which has the API's own answers (merge_into_self, not_found, ...);
  //   2 the confirm screen: both emotes side by side, an arrow from the source to the target, what
  //     will happen in plain words, then "Merge into <target>". It can't be undone.
  // On success it goes to the target's page and says what moved in a toast.
  //   <MergeEmoteDialog bind:this={dlg} emoteId={e.id} name={e.name} />      dlg.open()
  import { goto } from '$app/navigation';
  import { ApiError, gql } from '$lib/api';
  import Dialog from '$lib/components/Dialog.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count, iso } from '$lib/format';
  import { reducedMotion } from '$lib/media.svelte';
  import { EMOTE_MERGE_PREVIEW, MERGE_EMOTE } from '$lib/queries';
  import type { EmoteMergePreviewQuery } from '$lib/gql/graphql';
  import { notify } from '$lib/tool/notices.svelte';
  import { scaleFiles, type PageEmote } from './emote-page';
  import { mergeProblem, mergeSummary, mergeTarget } from './merge';

  type Props = { emoteId: string; name: string };
  let { emoteId, name }: Props = $props();

  type Preview = EmoteMergePreviewQuery['emoteMergePreview'];

  let dialog: Dialog;
  const uid = $props.id();
  let text = $state('');
  let preview = $state<Preview | null>(null);
  let asking = $state(false);
  let merging = $state(false);
  let problem = $state('');

  export function open() {
    text = '';
    preview = null;
    problem = '';
    dialog.open();
  }

  const id = $derived(mergeTarget(text));
  const fail = (e: unknown) => mergeProblem(e instanceof ApiError ? e.code : undefined, e instanceof Error ? e.message : '');

  async function next(ev: SubmitEvent) {
    ev.preventDefault();
    if (asking) return;
    if (!id) {
      problem = text.trim() ? 'That isn’t an emote link or ID. Paste the address of the emote’s page, or its ID.' : 'Paste the link or ID of the emote to merge into.';
      return;
    }
    asking = true;
    problem = '';
    try {
      preview = (await gql(EMOTE_MERGE_PREVIEW, { id: emoteId, intoId: id })).emoteMergePreview;
    } catch (e) {
      problem = fail(e);
    } finally {
      asking = false;
    }
  }

  async function merge() {
    if (!preview || merging) return;
    merging = true;
    problem = '';
    const from = preview.source.name;
    try {
      const r = (await gql(MERGE_EMOTE, { id: emoteId, intoId: preview.target.id })).mergeEmote;
      notify(mergeSummary(iso(from), iso(r.target.name), r.moved, r.collapsed.length));
      dialog.close();
      await goto(`/emotes/${encodeURIComponent(r.target.id)}`);
    } catch (e) {
      problem = fail(e);
    } finally {
      merging = false;
    }
  }

  // The picture at 2x and 4x (the 4x at most 128 px tall), the still frame under reduced motion.
  const url = (e: PageEmote, f: { scale: number; name: string }) =>
    reducedMotion.matches && e.animated && e.files.some((x) => x.name === '1x_static.webp') ? `${e.host}/${f.scale}x_static.webp` : `${e.host}/${f.name}`;
  const pics = (e: PageEmote) => scaleFiles(e.files).filter((f) => f.scale === 2 || f.scale === 4);
  const targetName = $derived(preview ? clean(preview.target.name) : '');
  const busy = $derived(asking || merging);
</script>

<Dialog bind:this={dialog} title={preview ? 'Merge emotes' : `Merge ${clean(name)}`} titleId="{uid}-title" wide {busy} class="merge-emote" onsubmit={preview ? undefined : next}>
  {#if !preview}
    <div class="step">
      <label class="field" for="{uid}-into">
        <span class="k">Link or ID of the emote to merge into</span>
        <input id="{uid}-into" class="ui-input" bind:value={text} placeholder="https://cyex.tv/emotes/…" autocomplete="off" spellcheck="false" aria-invalid={problem ? 'true' : undefined} aria-describedby="{uid}-hint" />
      </label>
      <p id="{uid}-hint" class="hint">Paste the emote’s page address or its ID. You see what will happen before anything changes.</p>
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
  {:else}
    {@const p = preview}
    <div class="step confirm">
      {#if p.followed}
        <p class="note followed" role="status"><Icon name="arrow-right" size={14} /><span>That emote was merged into <bdi>{targetName}</bdi> already; this goes to <bdi>{targetName}</bdi>.</span></p>
      {/if}
      <div class="pair">
        {#each [{ key: 'from', e: p.source, label: 'Merge this' }, { key: 'to', e: p.target, label: 'Into this' }] as side, i (side.key)}
          {#if i === 1}<span class="arrow" aria-hidden="true"><Icon name="arrow-right" size={20} /></span>{/if}
          <section class="card {side.key}" aria-label="{side.label}: {clean(side.e.name)}">
            <h3 class="lbl">{side.label}</h3>
            <div class="pics">
              {#each pics(side.e) as f (f.name)}
                <img src={url(side.e, f)} width={f.scale === 4 ? Math.min(f.width, 128) : f.width} height={f.scale === 4 ? Math.min(f.height, 128) : f.height} alt={f.scale === 4 ? clean(side.e.name) : ''} />
              {/each}
            </div>
            <p class="nm"><bdi>{clean(side.e.name)}</bdi></p>
            <p class="meta">{#if side.e.owner}by <bdi>{clean(side.e.owner.displayName)}</bdi> · {/if}{count(side.e.channelCount)} {side.e.channelCount === 1 ? 'channel' : 'channels'}</p>
          </section>
        {/each}
      </div>

      <h3 class="lbl">What happens</h3>
      <ul class="what">
        <li>{#if p.moves === 0}No set entry needs to move: every set that has <bdi>{clean(p.source.name)}</bdi> has <bdi>{targetName}</bdi> already.{:else}{count(p.moves)} {p.moves === 1 ? 'set entry moves' : 'set entries move'} to <bdi>{targetName}</bdi> and keep {p.moves === 1 ? 'its' : 'their'} name in the set. Channel sets, personal sets and any other set.{/if}</li>
        {#if p.collapses.length}
          <li>
            {p.collapses.length === 1 ? '1 set already has' : `${count(p.collapses.length)} sets already have`} <bdi>{targetName}</bdi> and keep{p.collapses.length === 1 ? 's' : ''} that entry:
            <ul class="sets">
              {#each p.collapses.slice(0, 8) as c (c.setId)}
                <li><bdi>{clean(c.setName)}</bdi>{#if c.keptName !== c.droppedName}: keeps <bdi>{clean(c.keptName)}</bdi>, drops <bdi>{clean(c.droppedName)}</bdi>{/if}</li>
              {/each}
              {#if p.collapses.length > 8}<li>and {count(p.collapses.length - 8)} more</li>{/if}
            </ul>
          </li>
        {/if}
        <li>The name, tags and files of <bdi>{targetName}</bdi> stay as they are.</li>
        <li><bdi>{clean(p.source.name)}</bdi> is retired: it leaves search, and its page and links go to <bdi>{targetName}</bdi>.</li>
        <li>Channels can remove the emote as usual, and the counts are redone.</li>
        <li>Both owners are told.</li>
        <li class="strong">This can’t be undone.</li>
      </ul>
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
  {/if}
  {#snippet footer()}
    {#if preview}
      <button class="ui-btn ghost" type="button" disabled={busy} onclick={() => { preview = null; problem = ''; }}>Back</button>
      <button class="ui-btn secondary" type="button" disabled={busy} onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn danger armed" type="button" disabled={busy} onclick={merge}>{merging ? 'Merging…' : `Merge into ${targetName}`}</button>
    {:else}
      <button class="ui-btn secondary" type="button" disabled={busy} onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={asking || !text.trim()}>{asking ? 'Checking…' : 'Next'}</button>
    {/if}
  {/snippet}
</Dialog>

<style>
  :global(.merge-emote) { width: min(640px, calc(100vw - 24px)); }
  p { margin: 0; }
  .step { display: grid; gap: 12px; min-width: 0; }
  .field { display: grid; gap: 4px; min-width: 0; }
  .k { font-size: var(--t-small); font-weight: 600; color: var(--muted); }
  .hint { font-size: var(--t-small); color: var(--muted); }
  .lbl { margin: 0; font-size: var(--t-label, 12px); font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .note { display: flex; gap: 8px; align-items: flex-start; padding: 8px 10px; border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface-2); font-size: var(--t-ui); }
  .note :global(.icon) { flex: none; margin-top: 3px; color: var(--muted); }
  .pair { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); gap: 12px; align-items: stretch; }
  .arrow { display: grid; place-items: center; color: var(--muted); }
  .card { display: grid; align-content: start; gap: 6px; min-width: 0; padding: 12px; border: 1px solid var(--border); border-radius: var(--radius-btn); background: var(--surface); }
  .card.to { border-color: var(--border-strong); }
  .pics { display: flex; align-items: flex-end; gap: 16px; min-height: 64px; padding: 4px 0; }
  .pics img { display: block; max-width: 100%; object-fit: contain; }
  .nm { font-weight: 700; overflow-wrap: anywhere; }
  .meta { font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .what { margin: 0; padding-left: 18px; display: grid; gap: 6px; font-size: var(--t-ui); }
  .what .strong { font-weight: 700; }
  .sets { margin: 4px 0 0; padding-left: 16px; font-size: var(--t-small); color: var(--muted); display: grid; gap: 2px; }
  @media (max-width: 639.98px) {
    .pair { grid-template-columns: minmax(0, 1fr); }
    .arrow { transform: rotate(90deg); height: 24px; }
  }
</style>
