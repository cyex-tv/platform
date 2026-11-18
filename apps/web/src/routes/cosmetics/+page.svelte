<script lang="ts">
  // /cosmetics: every badge and name paint on cyex.tv, drawn live. Public, signed out too. Paints
  // show their own name in the paint; signed in, "Your name" draws yours instead, so you see how
  // it would look. Never who owns what (ADR 0015: ownership is private). One search narrows both.
  import { invalidateAll } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { clean, count } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  let find = $state('');
  let preview = $state<'paint' | 'name'>('paint');
  let retrying = $state(false);

  const me = $derived(session.me);
  const myName = $derived(clean(me?.identities[0]?.displayName) || 'you');
  const q = $derived(find.trim().toLowerCase());
  const has = (s: string | null | undefined) => !!s && s.toLowerCase().includes(q);
  const badges = $derived(q ? data.badges.filter((b) => has(b.name) || has(b.tooltip)) : data.badges);
  const paints = $derived(q ? data.paints.filter((p) => has(p.name)) : data.paints);
  const kind = { LINEAR_GRADIENT: 'Linear', RADIAL_GRADIENT: 'Radial', IMAGE: 'Picture' } as Record<string, string>;
  const empty = $derived(!data.badges.length && !data.paints.length);

  async function retry() {
    retrying = true;
    await invalidateAll();
    retrying = false;
  }
</script>

<svelte:head>
  <title>Cosmetics - cyex.tv</title>
  <meta name="description" content="Every chat badge and name paint on cyex.tv, as they look in Twitch and Kick chat." />
  <meta property="og:title" content="Cosmetics" />
</svelte:head>

<ColumnLayout>
  <header class="page-head head">
    <div>
      <h1 class="page-title">Cosmetics</h1>
      <p class="lead">Badges sit before a name in chat, paints colour the name itself. Everyone with the cyex.tv extension sees them on Twitch and Kick. For now the cyex.tv team hands them out.</p>
    </div>
    {#if me}<a class="ui-btn secondary" href={sectionHref(me.id, 'cosmetics')}>Pick yours</a>{/if}
  </header>

  {#if data.error}
    <State kind="error" title="The cosmetics didn’t load.">
      {data.error}
      {#snippet actions()}<button type="button" class="ui-btn" onclick={retry} disabled={retrying}>{retrying ? 'Trying…' : 'Try again'}</button>{/snippet}
    </State>
  {:else if empty}
    <State title="No cosmetics yet.">The first badges and paints are on their way.</State>
  {:else}
    <div class="toolbar">
      <label class="find">
        <span class="sr-only">Search badges and paints</span>
        <Icon name="search" size={15} />
        <input class="input" type="search" placeholder="Search" bind:value={find} autocomplete="off" data-page-search />
      </label>
      {#if me && data.paints.length}
        <Segmented label="Paint preview" size="s" options={[{ value: 'paint', label: 'Paint name', icon: 'brush' }, { value: 'name', label: 'Your name', icon: 'user' }]}
          value={preview} onchange={(v) => (preview = v)} />
      {/if}
    </div>

    <section class="group" aria-labelledby="badges-h">
      <h2 class="block-title" id="badges-h">Badges <span class="n">{count(badges.length)}</span></h2>
      {#if !data.badges.length}
        <State bare title="No badges yet." />
      {:else if !badges.length}
        <p class="none">No badge matches “{find.trim()}”.</p>
      {:else}
        <ul class="grid" aria-labelledby="badges-h">
          {#each badges as b (b.id)}
            <li class="item badge">
              <span class="art"><BadgeIcon badge={b} size={36} label /></span>
              <span class="txt">
                <strong><bdi>{clean(b.name)}</bdi></strong>
                {#if b.tooltip && b.tooltip !== b.name}<span class="sub"><bdi>{clean(b.tooltip)}</bdi></span>{/if}
              </span>
              <span class="chat" aria-hidden="true"><BadgeIcon badge={b} /><span class="who">{me ? myName : 'lumen_fox'}</span><span class="msg">: hi chat</span></span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="group" aria-labelledby="paints-h">
      <h2 class="block-title" id="paints-h">Paints <span class="n">{count(paints.length)}</span></h2>
      {#if !data.paints.length}
        <State bare title="No paints yet." />
      {:else if !paints.length}
        <p class="none">No paint matches “{find.trim()}”.</p>
      {:else}
        <ul class="grid" aria-labelledby="paints-h">
          {#each paints as p (p.id)}
            {@const shown = preview === 'name' && me ? myName : p.name}
            <li class="item paint">
              <span class="sample" aria-hidden={shown !== p.name}><PaintedName name={shown} paint={p} /></span>
              <span class="sub">{#if shown !== p.name}<bdi>{clean(p.name)}</bdi>{' · '}{/if}{kind[p.kind] ?? 'Paint'}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <p class="end">How do I get one? <a class="link inline" href="/help#get-cosmetics">Help</a> has the answer.</p>
  {/if}
</ColumnLayout>

<style>
  .head { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: space-between; gap: 16px 24px; }
  .lead { margin: 12px 0 0; max-width: 62ch; font-size: var(--t-body, 16px); line-height: 1.6; color: var(--muted); }

  .toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; margin-bottom: 24px; }
  .find { position: relative; display: block; width: 260px; }
  .find :global(svg) { position: absolute; left: 10px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 40px; padding-left: 32px; }

  .group + .group { margin-top: clamp(32px, 4vw, 48px); }
  .block-title { display: flex; align-items: baseline; gap: 10px; margin-bottom: 16px; }
  .n { font-size: var(--t-small); font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }

  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr)); }
  .item { display: grid; align-content: start; gap: 8px; min-width: 0; padding: 16px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .badge { grid-template-columns: auto minmax(0, 1fr); align-items: center; column-gap: 12px; }
  .art { display: grid; place-items: center; width: 52px; height: 52px; border-radius: var(--radius); background: var(--bg); }
  .txt { display: grid; min-width: 0; line-height: 1.35; }
  .txt strong, .txt .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .txt strong { font-weight: 600; }
  .sub { font-size: var(--t-fine); color: var(--dim); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* The badge as chat draws it: 18px before a name, on the chat ground. */
  .chat { grid-column: 1 / -1; display: flex; align-items: center; gap: 5px; min-width: 0; padding: 6px 10px; border-radius: var(--radius);
    background: var(--bg); font-size: 13px; }
  .who { font-weight: 700; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .msg { color: var(--muted); white-space: nowrap; }

  .paint { padding: 18px 16px 14px; }
  .sample { display: flex; align-items: center; min-height: 52px; padding: 0 12px; border-radius: var(--radius); background: var(--bg);
    font-size: 20px; font-weight: 700; letter-spacing: -.01em; min-width: 0; }
  .none { margin: 0; padding: 12px 0; color: var(--dim); font-size: var(--t-small); }
  .end { margin: clamp(32px, 4vw, 48px) 0 0; font-size: var(--t-small); color: var(--muted); }
  .link.inline { display: inline; }

  @media (max-width: 640px) {
    .toolbar { align-items: stretch; }
    .find { flex: 1 1 100%; width: auto; }
  }
</style>
