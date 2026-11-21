<script lang="ts">
  // Give one badge, paint or role to many accounts at once. Paste logins (one per line, or with
  // commas; `twitch:name` or `kick:name` when a login is on both), Check matches them to accounts
  // (accountsByLogin) and says which didn't match and why; Give then sends the existing grant or
  // assign call for each matched account, one after another (each audited as a single grant is),
  // and every row ends with what happened to it. Cosmetics need cosmetics.grant and a reason;
  // roles need roles.assign or roles.manage, and only roles you may give are offered. The form is
  // flat rows on the left; the inspector on the right sums up who will get it and how it went.
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
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
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
  const doneCount = $derived(rows?.filter((r) => r.result?.ok).length ?? 0);
  const failCount = $derived(rows?.filter((r) => r.result && !r.result.ok).length ?? 0);

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

{#if !mayCosmetics && !mayRoles}
  <Bar title="Give to many" />
  <State title="Nothing to give here.">This needs Give cosmetics, or Assign roles.</State>
{:else}
<Split label="Result" open={true} onclose={() => {}} stack="top">
  <Bar title="Give to many" count={parsed.valid ? `${count(parsed.valid)} ${parsed.valid === 1 ? 'login' : 'logins'}` : null} />
  <LoadGate source={L} label="Loading what you can give…">
  {#snippet children()}
    <form onsubmit={check} aria-label="Give to many">
      <section class="adm-flat" aria-labelledby="step-what">
        <h2 class="adm-label" id="step-what">What to give</h2>
        {#if options.length > 1}
          <div class="adm-lv">
            <span class="l">Kind</span>
            <div class="v"><Segmented label="What to give" size="s" options={options} value={what} onchange={(v) => (what = v)} /></div>
          </div>
        {/if}
        <label class="adm-lv">
          <span class="l">{what === 'BADGE' ? 'Badge' : what === 'PAINT' ? 'Paint' : 'Role'}</span>
          <span class="v">
            {#if choices.length}
              <select class="ui-input" bind:value={item} disabled={!!running}>
                {#each choices as c (c.id)}<option value={c.id}>{clean(c.name)}</option>{/each}
              </select>
            {:else}
              <span class="ui-hint">{what === 'ROLE' ? 'No role you may give.' : `No ${what === 'BADGE' ? 'badges' : 'paints'} yet.`}</span>
            {/if}
          </span>
        </label>
        {#if needsReason}
          <label class="adm-lv">
            <span class="l">Reason<small>Kept in the audit log</small></span>
            <span class="v"><input class="ui-input" type="text" bind:value={reason} maxlength="500" placeholder="Event prize, November" disabled={!!running} /></span>
          </label>
        {/if}
      </section>

      <section class="adm-flat" aria-labelledby="step-who">
        <h2 class="adm-label" id="step-who">People <span class="n" id="logins-count" aria-live="polite">
          {#if parsed.logins.length}{count(parsed.valid)} ready{#if parsed.odd.length}<span class="miss">{' · '}{count(parsed.odd.length)} can’t be a login</span>{/if}{#if parsed.extra}<span class="miss">{' · '}{count(parsed.extra)} over the {BATCH_MAX} a check takes</span>{/if}{:else}none yet{/if}
        </span></h2>
        <label class="adm-lv top">
          <span class="l">Logins<small>One per line, or separated by commas. Write <code>twitch:</code> or <code>kick:</code> first when a login is on both.</small></span>
          <span class="v">
            <textarea class="ui-input logins" rows="7" bind:value={text} spellcheck="false" autocomplete="off" disabled={!!running}
              placeholder={'nora\nkick:lumen_fox\ntwitch:cat_jammer'} aria-describedby="logins-count"></textarea>
          </span>
        </label>
        {#if parsed.odd.length}<p class="ui-hint odd"><bdi>{parsed.odd.slice(0, 3).join(', ')}{parsed.odd.length > 3 ? ', …' : ''}</bdi></p>{/if}
        <div class="adm-lv">
          <span class="l"></span>
          <div class="v">
            <button class="ui-btn secondary small" type="submit" disabled={!parsed.valid || checking || !!running}>
              {#if checking}<span class="ui-spinner small" aria-hidden="true"></span>{/if}Check logins
            </button>
            {#if rows}<button class="ui-btn ghost small" type="button" onclick={reset} disabled={!!running}>Start over</button>{/if}
          </div>
        </div>
      </section>
    </form>

    <section class="adm-flat" aria-labelledby="res-title" aria-busy={checking}>
      <h2 id="res-title" class="adm-label">Check
        {#if rows}<span class="n">{count(rows.length - unmatched)} found{#if unmatched}{' · '}<span class="miss">{count(unmatched)} not found</span>{/if}</span>{/if}
      </h2>
      {#if !rows}
        <p class="ui-hint none">Nothing checked yet. Each login shows the account it names, or why there is none.</p>
      {:else}
        {#if stale}<p class="ui-hint stale"><Icon name="refresh" size={14} /> The list changed since the check. Check again before giving.</p>{/if}
        <ul class="adm-rows res">
          {#each rows as r (r.login)}
            <li class:bad={!r.account}>
              <span class="login adm-num"><bdi>{r.login}</bdi></span>
              <span class="who">
                {#if r.account}<AccountLink id={r.account.id} name={r.account.displayName} />{:else}<span class="ui-error">{loginProblem(r.problem)}</span>{/if}
              </span>
              <span class="rs">
                {#if r.result}<span class="adm-flags"><span class:ok={r.result.ok} class:warn={!r.result.ok}>{r.result.text}</span></span>{/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/snippet}
  </LoadGate>

  {#snippet panel()}
    <div class="adm-ip-head"><span class="adm-label">Result</span></div>
    <div class="adm-ip-body">
      <section class="adm-sec">
        <h2 class="adm-name"><bdi>{itemName ? clean(itemName) : 'Nothing picked'}</bdi></h2>
        <p class="adm-flags"><span>{what === 'BADGE' ? 'Badge' : what === 'PAINT' ? 'Paint' : 'Role'}</span></p>
      </section>
      <section class="adm-sec">
        <h3 class="adm-label">Who gets it</h3>
        <dl class="adm-kv">
          <dt>Logins pasted</dt><dd>{count(parsed.valid)}</dd>
          <dt>Accounts found</dt><dd>{rows ? count(rows.length - unmatched) : 'Not checked'}</dd>
          {#if rows}<dt>Not found</dt><dd>{count(unmatched)}</dd>{/if}
          <dt>Will get it</dt><dd>{rows ? count(matched.length) : 'n/a'}</dd>
          {#if rows && doneCount + failCount > 0}<dt>Given</dt><dd>{count(doneCount)}</dd><dt>Didn’t take it</dt><dd>{count(failCount)}</dd>{/if}
        </dl>
      </section>
      <section class="adm-sec">
        {#if running}
          <p class="ui-hint adm-num" aria-live="polite">Giving {count(running.done)} of {count(running.total)}…</p>
        {:else if rows}
          <p class="ui-hint">{#if matched.length}{clean(itemName) || 'Nothing picked'} goes to {count(matched.length)} {matched.length === 1 ? 'account' : 'accounts'}.{:else}Nobody left to give it to.{/if}</p>
        {/if}
        <div class="adm-acts">
          <button class="ui-btn primary" type="button" onclick={give}
            disabled={!rows || !!running || stale || !matched.length || !item || (needsReason && !reason.trim())}>
            Give to {count(matched.length)}
          </button>
        </div>
      </section>
    </div>
  {/snippet}
</Split>
{/if}

<style>
  form { display: grid; }
  .logins { resize: vertical; min-height: 140px; width: 100%; font-family: var(--font-mono, ui-monospace, monospace); font-size: 13px; }
  @media (pointer: coarse) { .logins { font-size: 16px; } }
  code { font-size: 12px; }
  .adm-lv .v .ui-input { flex: 1 1 200px; min-width: 0; }
  .adm-lv.top { align-items: start; }
  .odd { margin: 0; overflow-wrap: anywhere; }
  .none { margin: 0; }
  .miss { color: var(--red); }
  .stale { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--yellow); }
  .res li { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto; gap: 12px; align-items: center; min-height: 40px; padding: 0; }
  .login { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who .ui-error { margin: 0; font-size: var(--t-small); }
  .ui-spinner.small { width: 13px; height: 13px; }
  @media (max-width: 520px) {
    .res li { grid-template-columns: minmax(0, 1fr) auto; padding-block: 6px; }
    .who { grid-column: 1 / -1; grid-row: 2; }
  }
</style>
