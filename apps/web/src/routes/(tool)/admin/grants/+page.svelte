<script lang="ts">
  // Give one badge, paint or role to many accounts at once. Paste logins (one per line, or with
  // commas; `twitch:name` or `kick:name` when a login is on both), Check matches them to accounts
  // (accountsByLogin) and says which didn't match and why; Give then sends the existing grant or
  // assign call for each matched account, one after another (each audited as a single grant is),
  // and every row ends with what happened to it. Cosmetics need cosmetics.grant and a reason;
  // roles need roles.assign or roles.manage, and only roles you may give are offered.
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import { BATCH_MAX, loginProblem } from '$lib/admin/bulk';
  import { countLogins } from '$lib/admin/grant-logins';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog } from '$lib/cosmetics/store.svelte';
  import { clean, count } from '$lib/format';
  import type { AccountsByLoginQuery, CosmeticKind } from '$lib/gql/graphql';
  import { ACCOUNTS_BY_LOGIN, ALL_ROLES, ASSIGN_ROLE, GRANT_COSMETIC } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { ApiError } from '$lib/api';

  type What = 'BADGE' | 'PAINT' | 'ROLE';
  const mayCosmetics = can('cosmetics.grant');
  const mayRoles = can('roles.assign') || can('roles.manage');
  let what = $state<What>(mayCosmetics ? 'BADGE' : 'ROLE');
  let item = $state('');
  let reason = $state('');
  let text = $state('');

  const L = loader(async ({ fetch }) => {
    const [catalog, roles] = await Promise.all([
      mayCosmetics ? loadCatalog() : null,
      mayRoles ? gql(ALL_ROLES, {}, fetch).then((r) => r.roles.filter((x) => x.assignable)) : [],
    ]);
    return { badges: catalog ? [...catalog.badges.values()] : [], paints: catalog ? [...catalog.paints.values()] : [], roles };
  });
  const choices = $derived.by(() => {
    const d = L.data;
    if (!d) return [];
    if (what === 'BADGE') return d.badges.map((b) => ({ id: b.id, name: b.name }));
    if (what === 'PAINT') return d.paints.map((p) => ({ id: p.id, name: p.name }));
    return d.roles.map((r) => ({ id: r.id, name: r.name }));
  });
  $effect(() => {
    // A new kind: its first choice, unless the pick is still one of them.
    if (!choices.some((c) => c.id === item)) item = choices[0]?.id ?? '';
  });
  const options = $derived([
    ...(mayCosmetics ? [{ value: 'BADGE' as What, label: 'Badge' }, { value: 'PAINT' as What, label: 'Paint' }] : []),
    ...(mayRoles ? [{ value: 'ROLE' as What, label: 'Role' }] : []),
  ]);

  // The pasted list, read as you type; checked against accounts on Check.
  const parsed = $derived(countLogins(text));
  type Row = AccountsByLoginQuery['accountsByLogin'][number] & { result?: { ok: boolean; text: string } };
  let rows = $state<Row[] | null>(null);
  let checkedText = $state('');
  let checking = $state(false);
  let running = $state<{ done: number; total: number } | null>(null);
  const stale = $derived(rows !== null && checkedText !== text);
  const matched = $derived(rows?.filter((r) => r.account && !r.result) ?? []);
  const unmatched = $derived(rows?.filter((r) => !r.account).length ?? 0);
  const needsReason = $derived(what !== 'ROLE');
  const itemName = $derived(choices.find((c) => c.id === item)?.name ?? '');

  async function check(ev?: SubmitEvent) {
    ev?.preventDefault();
    if (!parsed.valid || checking) return;
    checking = true;
    try {
      rows = (await gql(ACCOUNTS_BY_LOGIN, { logins: parsed.logins })).accountsByLogin;
      checkedText = text;
    } catch (e) {
      fail(e);
    } finally {
      checking = false;
    }
  }

  async function give() {
    if (!rows || running || !item || (needsReason && !reason.trim())) return;
    const todo = matched;
    running = { done: 0, total: todo.length };
    let ok = 0;
    for (const [i, r] of todo.entries()) {
      try {
        if (what === 'ROLE') await gql(ASSIGN_ROLE, { accountId: r.account!.id, roleId: item });
        else await gql(GRANT_COSMETIC, { input: { accountId: r.account!.id, kind: what as CosmeticKind, cosmeticId: item, reason: reason.trim() } });
        r.result = { ok: true, text: what === 'ROLE' ? 'Role given' : 'Given' };
        ok++;
      } catch (e) {
        const code = e instanceof ApiError ? e.code : '';
        if (code === 'signed_out') {
          fail(e);
          break;
        }
        r.result = { ok: false, text: code === 'already_owned' ? 'Has it already' : e instanceof Error && e.message ? e.message : 'Didn’t go through' };
      }
      running = { done: i + 1, total: todo.length };
    }
    running = null;
    const left = todo.length - ok;
    notify(`${clean(itemName)} given to ${count(ok)} ${ok === 1 ? 'account' : 'accounts'}${left ? `; ${count(left)} didn’t take it` : ''}.`, left > 0 && ok === 0);
  }

  function reset() {
    rows = null;
    text = '';
    checkedText = '';
  }
</script>

<svelte:head><title>Give to many - Admin - cyex.tv</title></svelte:head>

<PageHead title="Give to many" line="One badge, paint or role for a list of accounts: pick what, paste who, check, give." />

{#if !mayCosmetics && !mayRoles}
  <State title="Nothing to give here.">This needs Give cosmetics, or Assign roles.</State>
{:else}
<LoadGate source={L} label="Loading what you can give…">
{#snippet children()}
  <div class="grid">
    <form class="steps" onsubmit={check}>
      <section class="ui-panel step" aria-labelledby="step-what">
        <h2 class="adm-label" id="step-what"><span class="no ui-num">1</span>What</h2>
        {#if options.length > 1}
          <Segmented label="What to give" options={options} value={what} onchange={(v) => (what = v)} />
        {/if}
        <label class="ui-field">
          <span class="ui-label">{what === 'BADGE' ? 'Badge' : what === 'PAINT' ? 'Paint' : 'Role'}</span>
          {#if choices.length}
            <select class="ui-input" bind:value={item} disabled={!!running}>
              {#each choices as c (c.id)}<option value={c.id}>{clean(c.name)}</option>{/each}
            </select>
          {:else}
            <span class="ui-hint">{what === 'ROLE' ? 'No role you may give.' : `No ${what === 'BADGE' ? 'badges' : 'paints'} yet.`}</span>
          {/if}
        </label>
        {#if needsReason}
          <label class="ui-field">
            <span class="ui-label">Reason</span>
            <input class="ui-input" type="text" bind:value={reason} maxlength="500" placeholder="Event prize, November" disabled={!!running} />
            <span class="ui-hint">Kept with each grant in the audit log.</span>
          </label>
        {/if}
      </section>

      <section class="ui-panel step" aria-labelledby="step-who">
        <h2 class="adm-label" id="step-who"><span class="no ui-num">2</span>Who</h2>
        <label class="ui-field">
          <span class="ui-label">Logins</span>
          <textarea class="ui-input logins" rows="7" bind:value={text} spellcheck="false" autocomplete="off" disabled={!!running}
            placeholder={'nora\nkick:lumen_fox\ntwitch:cat_jammer'} aria-describedby="logins-hint logins-count"></textarea>
          <span class="ui-hint" id="logins-hint">One per line, or separated by commas. Write <code>twitch:</code> or <code>kick:</code> in front when a login is on both.</span>
        </label>
        <p class="tally ui-num" id="logins-count" aria-live="polite">
          {#if parsed.logins.length}
            <b>{count(parsed.valid)}</b> {parsed.valid === 1 ? 'login' : 'logins'} ready{#if parsed.odd.length}<span class="miss">{' · '}{count(parsed.odd.length)} can’t be a login</span>{/if}{#if parsed.extra}<span class="miss">{' · '}{count(parsed.extra)} over the {BATCH_MAX} a check takes</span>{/if}
          {:else}
            Nothing pasted yet.
          {/if}
        </p>
        {#if parsed.odd.length}<p class="ui-hint odd"><bdi>{parsed.odd.slice(0, 3).join(', ')}{parsed.odd.length > 3 ? ', …' : ''}</bdi></p>{/if}
        <div class="ui-actions">
          <button class="ui-btn secondary" type="submit" disabled={!parsed.valid || checking || !!running}>
            {#if checking}<span class="ui-spinner small" aria-hidden="true"></span>{/if}Check logins
          </button>
          {#if rows}<button class="ui-btn ghost" type="button" onclick={reset} disabled={!!running}>Start over</button>{/if}
        </div>
      </section>
    </form>

    <section class="ui-panel results" aria-labelledby="res-title" aria-busy={checking}>
      <div class="rhead">
        <h2 id="res-title" class="adm-label"><span class="no ui-num">3</span>Check and give</h2>
        {#if rows}
          <span class="ui-hint ui-num">{count(rows.length - unmatched)} found{#if unmatched}{' · '}<span class="miss">{count(unmatched)} not found</span>{/if}</span>
        {/if}
      </div>
      {#if !rows}
        <State bare title="Nothing checked yet.">Check the logins: each one shows the account it names, or why there is none.</State>
      {:else}
        {#if stale}<p class="ui-hint stale"><Icon name="refresh" size={14} /> The list changed since the check. Check again before giving.</p>{/if}
        <ul class="adm-rows res">
          {#each rows as r (r.login)}
            <li class:bad={!r.account}>
              <span class="login ui-mono"><bdi>{r.login}</bdi></span>
              <span class="who">
                {#if r.account}<AccountLink id={r.account.id} name={r.account.displayName} />{:else}<span class="ui-error">{loginProblem(r.problem)}</span>{/if}
              </span>
              <span class="rs">
                {#if r.result}<span class="ui-chip" class:ok={r.result.ok} class:no={!r.result.ok}>{r.result.text}</span>{/if}
              </span>
            </li>
          {/each}
        </ul>
        <footer class="give">
          {#if running}
            <span class="ui-hint ui-num" aria-live="polite">Giving {count(running.done)} of {count(running.total)}…</span>
          {:else}
            <span class="ui-hint">{#if matched.length}{clean(itemName) || 'Nothing picked'} goes to {count(matched.length)} {matched.length === 1 ? 'account' : 'accounts'}.{:else}Nobody left to give it to.{/if}</span>
          {/if}
          <button class="ui-btn primary" type="button" onclick={give}
            disabled={!!running || stale || !matched.length || !item || (needsReason && !reason.trim())}>
            Give to {count(matched.length)}
          </button>
        </footer>
      {/if}
    </section>
  </div>
{/snippet}
</LoadGate>
{/if}

<style>
  .grid { display: grid; grid-template-columns: minmax(280px, 400px) minmax(0, 1fr); gap: 16px; align-items: start; }
  @media (max-width: 860px) { .grid { grid-template-columns: minmax(0, 1fr); } }
  .steps { display: grid; gap: 16px; min-width: 0; }
  .step, .results { display: grid; gap: 12px; padding: 16px; min-width: 0; }
  .step h2, .results h2 { display: flex; align-items: center; gap: 8px; }
  .no { display: inline-grid; place-items: center; width: 18px; height: 18px; border: 1px solid var(--border-strong); border-radius: var(--radius); font-size: 11px; letter-spacing: 0; color: var(--muted); }
  .logins { resize: vertical; min-height: 140px; font-family: var(--font-mono, ui-monospace, monospace); font-size: 13px; }
  @media (pointer: coarse) { .logins { font-size: 16px; } }
  code { font-size: 12px; }
  .tally { margin: 0; font-size: var(--t-small); color: var(--muted); }
  .tally b { color: var(--text); }
  .odd { margin: -6px 0 0; overflow-wrap: anywhere; }
  .rhead { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; }
  .rhead h2 { margin: 0; }
  .miss { color: var(--red); }
  .stale { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--yellow); }
  .res li { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto; gap: 12px; align-items: center; min-height: 44px; padding: 0 12px; }
  .login { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .who { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who .ui-error { margin: 0; font-size: var(--t-small); }
  .rs .ui-chip { text-transform: none; letter-spacing: 0; }
  .rs .ok { color: var(--green); }
  .rs .no { color: var(--red); }
  .give { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; padding-top: 4px; }
  .ui-spinner.small { width: 13px; height: 13px; }
  @media (max-width: 520px) {
    .res li { grid-template-columns: minmax(0, 1fr) auto; padding-block: 6px; }
    .who { grid-column: 1 / -1; grid-row: 2; }
  }
</style>
