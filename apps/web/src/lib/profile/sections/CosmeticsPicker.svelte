<script lang="ts">
  // Pick the badge and paint an account shows in chat (yours, or one you edit with PROFILE), from
  // what it owns. Two radio groups (7TV's cosmetics page): None first, then each owned badge
  // (picture + name) or paint (its name drawn in the paint, or your name with "Your name"). Each
  // has a search; the options sit in one grid (no layout switch). The current pick is bold with the accent radio, never a
  // filled box. Picking saves at once (selectCosmetics); a failure puts the old pick back.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import BadgeOriginal from '$lib/cosmetics/BadgeOriginal.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadOwned, picked, type Owned } from '$lib/cosmetics/store.svelte';
  import { clean, iso } from '$lib/format';
  import { SELECT_COSMETICS } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Props = { accountId: string; mine: boolean; name: string };
  let { accountId, mine, name }: Props = $props();

  const L = loader(() => loadOwned(accountId, mine));

  let badgeFind = $state('');
  let paintFind = $state('');
  let preview = $state<'paint' | 'name'>('paint');
  // The badge looked at (hovered or focused), shown large in the panel beside the options; with none, the picked one.
  let looked = $state<string | null>(null);
  let saving = $state(false);
  // Picks go one after the other, and a failure puts back only its own field, and only while no
  // later pick of that field has been made.
  let queue: Promise<unknown> = Promise.resolve();
  const latest = { badge: 0, paint: 0 };
  let pending = 0;

  const match = (q: string) => (n: string) => n.toLowerCase().includes(q.trim().toLowerCase());

  function choose(owned: Owned, kind: 'badge' | 'paint', id: string | null) {
    const cur = L.data ?? owned;
    const before = kind === 'badge' ? (cur.badgeId ?? null) : (cur.paintId ?? null);
    if (before === id) return;
    const mineSeq = ++latest[kind];
    const set = (c: Owned, v: string | null) =>
      kind === 'badge' ? picked(accountId, c, v, c.paintId ?? null) : picked(accountId, c, c.badgeId ?? null, v);
    L.set(set(cur, id));
    pending++;
    saving = true;
    queue = queue.then(async () => {
      try {
        const vars = kind === 'badge' ? { badgeId: id ?? '' } : { paintId: id ?? '' };
        await gql(SELECT_COSMETICS, { ...vars, ownerId: mine ? null : accountId });
        if (mineSeq === latest[kind]) {
          const label = kind === 'badge' ? cur.badges.find((b) => b.id === id)?.name : cur.paints.find((p) => p.id === id)?.name;
          notify(id ? `${iso(label)} on.` : `No ${kind} now.`);
        }
      } catch (e) {
        if (mineSeq === latest[kind] && L.data) L.set(set(L.data, before));
        fail(e);
      } finally {
        if (--pending === 0) saving = false;
      }
    });
  }
</script>

{#snippet find(label: string, value: string, set: (v: string) => void)}
  <label class="find">
    <span class="sr-only">Search {label}</span>
    <Icon name="search" size={15} />
    <input class="input" type="search" placeholder="Search" {value} oninput={(e) => set(e.currentTarget.value)} autocomplete="off" />
  </label>
{/snippet}

<PageHead title="Cosmetics" line={mine ? 'The badge and paint you show next to your name in Twitch and Kick chat, for everyone with cyex.tv.' : `What ${iso(clean(name))} shows in chat. You edit their profile.`}>
  {#snippet actions()}
    {#if mine}<a class="btn btn-s" href="/shop"><Icon name="shopping-bag" size={15} />Get more in the shop</a>{/if}
  {/snippet}
</PageHead>

<LoadGate source={L} label="Loading cosmetics…">
  {#snippet children(owned)}
    {@const badges = owned.badges.filter((b) => match(badgeFind)(b.name))}
    {@const paints = owned.paints.filter((p) => match(paintFind)(p.name))}
    <section class="panel" aria-labelledby="badge-h">
      <header class="bar">
        <h2 id="badge-h">Badge</h2>
        {#if owned.badges.length}
          <div class="tools">
            {@render find('badges', badgeFind, (v) => (badgeFind = v))}
          </div>
        {/if}
      </header>
      {#if !owned.badges.length}
        <State bare title="No badges yet.">{#if mine}Find them in <a href="/shop">the shop</a>, or redeem a <a href="/redeem">code</a>.{:else}They’re granted by the cyex.tv team for now.{/if}</State>
      {:else}
        {@const look = owned.badges.find((b) => b.id === (looked ?? owned.badgeId)) ?? null}
        <div class="split">
        <div class="opts" role="radiogroup" aria-labelledby="badge-h" aria-busy={saving}>
          {#if !badgeFind.trim()}
            <label class="opt" class:on={!owned.badgeId}>
              <input type="radio" name="badge" disabled={saving} checked={!owned.badgeId} onchange={() => choose(owned, 'badge', null)} />
              <span class="dot" aria-hidden="true"></span><Icon name="ban" size={18} /><span class="lb">None</span>
            </label>
          {/if}
          {#each badges as b (b.id)}
            <label class="opt" class:on={owned.badgeId === b.id} onmouseenter={() => (looked = b.id)} onmouseleave={() => (looked = null)}>
              <input type="radio" name="badge" disabled={saving} checked={owned.badgeId === b.id} onchange={() => choose(owned, 'badge', b.id)}
                onfocus={() => (looked = b.id)} onblur={() => (looked = null)} />
              <span class="dot" aria-hidden="true"></span><BadgeIcon badge={b} size={28} />
              <span class="lb"><bdi>{clean(b.name)}</bdi></span>
            </label>
          {:else}
            <p class="none">No badge matches “{badgeFind.trim()}”.</p>
          {/each}
        </div>
        <aside class="view" aria-label="Badge preview">
          <h3 class="label">Preview</h3>
          {#if look}
            {#key look.image.id}<BadgeOriginal badge={look} caption />{/key}
            <p class="bn"><bdi>{clean(look.name)}</bdi></p>
            {#if look.tooltip && look.tooltip !== look.name}<p class="tip">“<bdi>{clean(look.tooltip)}</bdi>”</p>{/if}
          {:else}
            <p class="none">No badge. Point at one to see it large.</p>
          {/if}
        </aside>
        </div>
      {/if}
    </section>

    <section class="panel" aria-labelledby="paint-h">
      <header class="bar">
        <h2 id="paint-h">Paint</h2>
        {#if owned.paints.length}
          <div class="tools">
            <Segmented label="Preview" size="s" options={[{ value: 'paint', label: 'Paint name', icon: 'brush' }, { value: 'name', label: mine ? 'Your name' : 'Their name', icon: 'user' }]}
              value={preview} onchange={(v) => (preview = v)} />
            {@render find('paints', paintFind, (v) => (paintFind = v))}
          </div>
        {/if}
      </header>
      {#if !owned.paints.length}
        <State bare title="No paints yet.">{#if mine}Find them in <a href="/shop">the shop</a>, or redeem a <a href="/redeem">code</a>.{:else}They’re granted by the cyex.tv team for now.{/if}</State>
      {:else}
        <div class="opts" role="radiogroup" aria-labelledby="paint-h" aria-busy={saving}>
          {#if !paintFind.trim()}
            <label class="opt" class:on={!owned.paintId}>
              <input type="radio" name="paint" disabled={saving} checked={!owned.paintId} onchange={() => choose(owned, 'paint', null)} />
              <span class="dot" aria-hidden="true"></span><Icon name="ban" size={18} /><span class="lb">None</span>
            </label>
          {/if}
          {#each paints as p (p.id)}
            <label class="opt" class:on={owned.paintId === p.id}>
              <input type="radio" name="paint" disabled={saving} checked={owned.paintId === p.id} onchange={() => choose(owned, 'paint', p.id)} aria-label={p.name} />
              <span class="dot" aria-hidden="true"></span>
              <span class="lb paint" aria-hidden="true">
                <PaintedName name={preview === 'paint' ? p.name : name} paint={p} />
              </span>
            </label>
          {:else}
            <p class="none">No paint matches “{paintFind.trim()}”.</p>
          {/each}
        </div>
      {/if}
    </section>
  {/snippet}
</LoadGate>

<style>
  .panel { margin-bottom: 16px; padding: 14px 16px 16px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; margin-bottom: 12px; }
  .bar h2 { flex: 1; margin: 0; font-size: 15px; font-weight: 600; }
  .tools { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .find { position: relative; display: block; width: 200px; }
  .find :global(svg) { position: absolute; left: 10px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 34px; padding-left: 32px; font-size: var(--t-small); }

  /* Options left, the large preview in a flush panel on the right (the Inspector pattern); stacks under 900 px. */
  .split { display: grid; grid-template-columns: minmax(0, 1fr) minmax(220px, 360px); gap: 16px; align-items: start; }
  .view { position: sticky; top: 12px; display: grid; gap: 8px; min-width: 0; padding-left: 16px; border-left: 1px solid var(--border); }
  .label { margin: 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .bn { margin: 0; font-size: var(--t-small); font-weight: 600; overflow-wrap: anywhere; }
  .tip { margin: 0; font-size: var(--t-fine); color: var(--muted); overflow-wrap: anywhere; }
  .view .none { padding: 0; }
  .opts { display: grid; gap: 2px 8px; grid-template-columns: repeat(auto-fill, minmax(min(200px, 100%), 1fr)); }
  .opt { position: relative; display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 4px 10px; border-radius: var(--radius);
    cursor: pointer; color: var(--muted); font-size: var(--t-small); font-weight: 500; transition: background-color var(--fast), color var(--fast); }
  .opt input { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .opt > :global(svg) { flex: none; color: var(--dim); }
  .dot { flex: none; width: 16px; height: 16px; border-radius: 50%; border: 1.5px solid var(--field-border); display: grid; place-items: center; }
  .on { color: var(--text); font-weight: 700; }
  .on .dot { border-color: var(--accent-text); }
  .on .dot::after { content: ''; width: 8px; height: 8px; border-radius: 50%; background: var(--accent-text); }
  .opt:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .lb { display: grid; min-width: 0; line-height: 1.3; }
  .lb > bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .paint { font-size: var(--t-ui); }
  .none { margin: 0; padding: 10px; color: var(--dim); font-size: var(--t-small); }
  @media (hover: hover) and (pointer: fine) { .opt:hover { color: var(--text); } }
  @media (max-width: 899.98px) {
    .split { grid-template-columns: minmax(0, 1fr); }
    .view { position: static; padding: 12px 0 0; border-left: 0; border-top: 1px solid var(--border); }
  }
  @media (max-width: 639.98px) {
    .tools { width: 100%; }
    .find { order: 3; flex: 1 1 100%; width: auto; }
  }
  @media (forced-colors: active) { .on .dot::after { background: Highlight; } .on .dot { border-color: Highlight; } }
</style>
