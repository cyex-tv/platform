<script lang="ts">
  // The profile's emote panel (the Inspector profile): an emote picked in the grid shows here, in
  // place of About, without leaving the page (`?e=<id>` in the address, so back closes it). The
  // emote on a neutral stage, its name, creator and upload date, flags and tags, the actions (add
  // to set and the More menu), the four sizes, a few facts and "Open full page". ← → step to the
  // emote before or after it in the grid, Escape closes (both handled by the page).
  import Avatar from '$lib/components/Avatar.svelte';
  import EmoteActions from '$lib/components/EmoteActions.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import State from '$lib/components/State.svelte';
  import { canReport, flagsOf, scaleFiles, type PageEmote } from '$lib/components/emote/emote-page';
  import { Viewer } from '$lib/components/emote/viewer.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { reduced, watchReduced } from '$lib/cosmetics/reduced.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { ready } from '$lib/emote';
  import { clean, count, date } from '$lib/format';
  import { session } from '$lib/tool/session.svelte';
  import { onMount } from 'svelte';

  // `head={false}`: inside InspectorLayout, whose own panel head carries the label and the buttons.
  type Props = { emote: PageEmote | null; failed?: boolean; onclose: () => void; head?: boolean };
  let { emote: e, failed = false, onclose, head = true }: Props = $props();
  onMount(watchReduced);

  // One sign-in check per emote shown (the actions share it).
  const viewer = $derived(e ? new Viewer(e.id) : null);
  $effect(() => {
    void viewer?.load();
  });
  $effect(() => {
    if (e?.owner?.id) wantLooks([e.owner.id]);
  });
  const files = $derived(e ? scaleFiles(e.files) : []);
  const big = $derived(files.at(-1));
  const frozen = $derived(!!e && e.animated && reduced.on && e.files.some((f) => f.name === '1x_static.webp'));
  const url = (f: { scale: number; name: string }) => (frozen ? `${e!.host}/${f.scale}x_static.webp` : `${e!.host}/${f.name}`);
  const name = $derived(e ? clean(e.name) : '');
  const owner = $derived(e?.owner ? clean(e.owner.displayName) : '');
  const path = $derived(e ? `/emotes/${encodeURIComponent(e.id)}` : '');
  const flags = $derived(e ? flagsOf(e) : []);
</script>

{#if head}
  <div class="ip-head">
    <h2 class="label">Selected emote</h2>
    {#if e}<IconButton icon="open" label="Open full page" size="s" variant="ghost" href={path} />{/if}
    <IconButton icon="x" label="Close" size="s" variant="ghost" onclick={onclose} />
  </div>
{/if}

{#if failed}
  <div class="ip-body" class:bare={!head}><State bare title="Couldn’t load this emote.">It may have been taken down. <a class="link" href={path}>Open its page</a></State></div>
{:else if !e}
  <div class="ip-body" class:bare={!head} aria-busy="true"><div class="ip-stage ui-skeleton"></div><span class="ph ui-skeleton"></span></div>
{:else}
  <div class="ip-body" class:bare={!head}>
    <div class="ip-stage">
      {#if ready(e) && big}
        <img src={url(big)} width={big.width} height={big.height} alt={name} />
      {:else}
        <p class="dim">The files are still being made.</p>
      {/if}
    </div>
    <div class="who">
      <p class="ip-name"><bdi>{name}</bdi></p>
      {#if e.owner}
        <a class="by" href="/users/{e.owner.id}">
          <Avatar src={e.owner.avatar} name={owner} size={20} /><PaintedName name={owner} paint={looks[e.owner.id]?.paint} />
          <span class="dim">· <time datetime={e.createdAt}>{date(e.createdAt)}</time></span>
        </a>
      {/if}
    </div>
    {#if flags.length || e.tags.length}
      <ul class="chips">
        {#each flags as f (f.key)}<li class="chip"><Icon name={f.icon} size={12} />{f.label}</li>{/each}
        {#each e.tags as t (t)}<li><a class="chip tag" href="/search?tag={encodeURIComponent(t)}">#<bdi>{clean(t)}</bdi></a></li>{/each}
      </ul>
    {/if}
    {#if viewer}
      {#key e.id}
        <div class="acts">
          <EmoteActions emoteId={e.id} {name} {viewer} picture={e} addable={e.state === 'APPROVED'} reportable={canReport(e, session.me?.id)} owner={e.owner} {path} />
        </div>
      {/key}
    {/if}
    {#if ready(e) && files.length}
      <ul class="ladder" aria-label="Sizes">
        {#each files as f (f.name)}
          <li><img src={url(f)} alt="" height={Math.min(f.height, 16 * f.scale)} style:aspect-ratio="{f.width} / {f.height}" /><span>{f.scale}x</span></li>
        {/each}
      </ul>
    {/if}
    <dl class="kv">
      {#if e.channelCount != null}<dt>Channels</dt><dd>{count(e.channelCount)}</dd>{/if}
      <dt>Uploaded</dt><dd><time datetime={e.createdAt}>{date(e.createdAt)}</time></dd>
    </dl>
    <a class="btn btn-s open" href={path}>Open full page<Icon name="arrow-right" size={14} /></a>
  </div>
{/if}

<style>
  p { margin: 0; }
  .ip-head { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 4px; height: 48px; padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .label { margin: 0 auto 0 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .ip-body { display: grid; gap: 16px; padding: 16px; }
  .ip-body.bare { padding: 8px 0 0; }
  .ip-stage { display: grid; place-items: center; height: 180px; padding: 16px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--bg); }
  .ip-stage img { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; }
  .ph { display: block; height: 20px; width: 60%; border-radius: var(--radius); }
  .who { display: grid; gap: 6px; min-width: 0; }
  .ip-name { font-size: 18px; font-weight: 700; letter-spacing: -.01em; overflow-wrap: anywhere; }
  .by { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .by :global(.pn) { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dim { color: var(--muted); font-weight: 400; white-space: nowrap; }
  .chips { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px; }
  .chip { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 8px; border-radius: var(--radius);
    background: var(--neutral-chip, var(--surface-2)); color: var(--muted); font-size: var(--t-fine); font-weight: 600; }
  .tag { color: var(--text); }
  .acts :global(.actions-row) { justify-content: flex-start; }
  .ladder { list-style: none; margin: 0; padding: 12px; display: flex; align-items: flex-end; gap: 14px; border: 1px solid var(--border);
    border-radius: var(--radius-card); overflow-x: auto; }
  .ladder li { display: grid; justify-items: center; gap: 6px; flex: none; }
  .ladder img { display: block; width: auto; }
  .ladder span { font-size: var(--t-fine); font-weight: 600; color: var(--muted); font-variant-numeric: tabular-nums; }
  .kv { display: grid; grid-template-columns: minmax(0, 1fr) auto; margin: 0; border-top: 1px solid var(--border); }
  .kv dt, .kv dd { margin: 0; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .kv dt { color: var(--muted); }
  .kv dd { text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; }
  .open { justify-self: start; gap: 6px; }
  .link { color: var(--accent-text); }
</style>
