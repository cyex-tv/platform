<script lang="ts">
  // The Edit emote pop-up on the emote page (ADR 0038). Three parts, each only for who may use it:
  //   Details   the owner, account editors with Manage and staff with emotes.edit_any: name, tags,
  //             zero-width and private, saved with updateEmote;
  //   Approvals staff with emotes.review (public listing) and emotes.review_personal (personal
  //             use): Approve or Deny each, a denial with a reason (reviewApprovals, onlyPending
  //             false so a decision already made can change);
  //   Transfer  staff with emotes.transfer: search an account by name or paste its profile link,
  //             pick it, confirm. The API checks every permission again; its message is shown.
  //   <EditEmoteDialog bind:this={dlg} emote={e} canEdit />      dlg.open()
  import { invalidateAll } from '$app/navigation';
  import { gql } from '$lib/api';
  import Dialog from '$lib/components/Dialog.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, iso } from '$lib/format';
  import type { ApprovalDecision, ApprovalState } from '$lib/gql/graphql';
  import { ACCOUNT, EMOTE_REVIEW_APPROVALS, SEARCH_ACCOUNTS, TRANSFER_EMOTE, UPDATE_EMOTE } from '$lib/queries';
  import { TAG_MAX, cleanTag, nameProblem, tagsProblem } from '$lib/rules';
  import { can } from '$lib/tool/session.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { accountIdFromLink, approvalWord, approvalTone, type Approvals } from './approvals';
  import type { PageEmote } from './emote-page';

  type Props = { emote: PageEmote; canEdit: boolean };
  let { emote, canEdit }: Props = $props();

  let dialog: Dialog;
  export function open() {
    fill();
    dialog.open();
  }

  const uid = $props.id();
  const canListing = $derived(can('emotes.review'));
  const canPersonal = $derived(can('emotes.review_personal'));
  const canTransfer = $derived(can('emotes.transfer'));

  // Details.
  let name = $state('');
  let tags = $state<string[]>([]);
  let zeroWidth = $state(false);
  let isPrivate = $state(false);
  let saving = $state(false);
  let saveError = $state('');
  const nameError = $derived(nameProblem(name));
  const tagError = $derived(tagsProblem(tags));
  const changed = $derived(
    name.trim() !== emote.name ||
      [...tags].sort().join(',') !== [...emote.tags].sort().join(',') ||
      zeroWidth !== emote.flags.zeroWidth ||
      isPrivate !== emote.flags.private,
  );
  function fill() {
    name = emote.name;
    tags = [...emote.tags];
    zeroWidth = emote.flags.zeroWidth;
    isPrivate = emote.flags.private;
    saveError = '';
    approvalError = '';
    asking = null;
    reason = '';
    picked = null;
    confirming = false;
    transferError = '';
  }

  const message = (e: unknown) => (e instanceof Error && e.message ? e.message : 'That didn’t work.');

  async function save(ev: SubmitEvent) {
    ev.preventDefault();
    if (saving || nameError || tagError || !changed) return;
    saving = true;
    saveError = '';
    try {
      await gql(UPDATE_EMOTE, { id: emote.id, input: { name: name.trim(), tags, zeroWidth, private: isPrivate } });
      notify('Saved.');
      await invalidateAll();
    } catch (e) {
      saveError = message(e);
    } finally {
      saving = false;
    }
  }

  // Approvals.
  type Part = 'publicListing' | 'personalUse';
  const current = $derived<Approvals>(emote.approvals ?? { publicListing: 'PENDING', personalUse: 'PENDING' });
  let busyPart = $state<Part | null>(null);
  let asking = $state<Part | null>(null);
  let reason = $state('');
  let approvalError = $state('');

  async function decide(part: Part, decision: ApprovalDecision) {
    if (busyPart) return;
    if (decision === 'DENY' && !reason.trim()) return;
    busyPart = part;
    approvalError = '';
    try {
      const r = await gql(EMOTE_REVIEW_APPROVALS, {
        ids: [emote.id],
        input: { [part]: decision, reason: decision === 'DENY' ? reason.trim() : undefined, onlyPending: false },
      });
      if (r.reviewApprovals.failed.length) throw new Error(r.reviewApprovals.failed[0].message);
      notify(`${part === 'publicListing' ? 'Public listing' : 'Personal use'} ${decision === 'APPROVE' ? 'approved' : 'denied'}.`);
      asking = null;
      reason = '';
      await invalidateAll();
    } catch (e) {
      approvalError = message(e);
    } finally {
      busyPart = null;
    }
  }
  const rows = $derived<{ part: Part; label: string; show: boolean; state: ApprovalState }[]>([
    { part: 'publicListing', label: 'Public listing', show: canListing, state: current.publicListing },
    { part: 'personalUse', label: 'Personal use', show: canPersonal, state: current.personalUse },
  ]);

  // Transfer.
  type Found = { id: string; displayName: string };
  let query = $state('');
  let results = $state<Found[] | null>(null);
  let searching = $state(false);
  let picked = $state<Found | null>(null);
  let confirming = $state(false);
  let transferring = $state(false);
  let transferError = $state('');

  async function find(ev: SubmitEvent) {
    ev.preventDefault();
    const text = query.trim();
    if (!text || searching) return;
    searching = true;
    transferError = '';
    picked = null;
    confirming = false;
    try {
      const id = accountIdFromLink(text);
      if (id) {
        const a = (await gql(ACCOUNT, { id })).account;
        results = a ? [{ id: a.id, displayName: a.displayName }] : [];
      } else {
        const r = await gql(SEARCH_ACCOUNTS, { query: text, page: 1, perPage: 8 });
        results = r.searchAccounts.items.map((i) => ({ id: i.account.id, displayName: i.account.displayName }));
      }
    } catch (e) {
      results = null;
      transferError = message(e);
    } finally {
      searching = false;
    }
  }

  async function transfer() {
    if (!picked || transferring) return;
    transferring = true;
    transferError = '';
    try {
      await gql(TRANSFER_EMOTE, { id: emote.id, toAccountId: picked.id });
      notify(`${iso(emote.name)} now belongs to ${iso(clean(picked.displayName))}.`);
      dialog.close();
      await invalidateAll();
    } catch (e) {
      transferError = message(e);
      confirming = false;
    } finally {
      transferring = false;
    }
  }
  const busy = $derived(saving || busyPart !== null || transferring);
</script>

<Dialog bind:this={dialog} title="Edit emote" titleId="{uid}-title" wide {busy} class="edit-emote">
  <div class="parts">
    {#if canEdit}
      <form class="part" onsubmit={save} novalidate aria-label="Details">
        <h3 class="lbl">Details</h3>
        <label class="field" for="{uid}-name">
          <span class="k">Name</span>
          <input id="{uid}-name" class="ui-input" bind:value={name} maxlength="100" spellcheck="false" autocomplete="off" aria-invalid={nameError ? 'true' : undefined} />
        </label>
        {#if nameError}<p class="ui-error">{nameError}</p>{/if}
        <div class="field">
          <span class="k">Tags</span>
          <TagInput label="Tags" {tags} onchange={(t) => (tags = t)} max={TAG_MAX} clean={cleanTag} placeholder="cat, dance" id="{uid}-tags" disabled={saving} />
        </div>
        {#if tagError}<p class="ui-error">{tagError}</p>{/if}
        <label class="check"><input type="checkbox" bind:checked={zeroWidth} disabled={saving} /><span><strong>Zero-width</strong><span class="hint">Drawn over the emote before it, like a hat or rain.</span></span></label>
        <label class="check"><input type="checkbox" bind:checked={isPrivate} disabled={saving} /><span><strong>Private</strong><span class="hint">Only for your own channel: nobody else can find it or add it to their sets.</span></span></label>
        <p class="hint">Listing is decided in review. A renamed or retagged emote goes back to review.</p>
        {#if saveError}<p class="ui-error" role="alert">{saveError}</p>{/if}
        <div><button class="ui-btn primary" type="submit" disabled={saving || !changed || !!nameError || !!tagError}>{saving ? 'Saving…' : 'Save'}</button></div>
      </form>
    {/if}

    {#if canListing || canPersonal}
      <section class="part" aria-label="Approvals">
        <h3 class="lbl">Approvals</h3>
        <ul class="rows">
          {#each rows.filter((r) => r.show) as r (r.part)}
            <li class="arow" data-part={r.part}>
              <div class="line">
                <span class="k">{r.label}</span>
                <span class="state {approvalTone(r.state)}">{#if r.state === 'APPROVED'}<Icon name="check" size={12} stroke={2.4} />{/if}{approvalWord(r.state)}</span>
                <span class="btns">
                  <button class="ui-btn small secondary" type="button" disabled={busyPart !== null || r.state === 'APPROVED'} onclick={() => decide(r.part, 'APPROVE')}>
                    {busyPart === r.part && asking !== r.part ? 'Approving…' : 'Approve'}
                  </button>
                  <button class="ui-btn small danger" type="button" disabled={busyPart !== null || r.state === 'DENIED'} aria-expanded={asking === r.part}
                    onclick={() => { asking = asking === r.part ? null : r.part; reason = ''; }}>Deny</button>
                </span>
              </div>
              {#if asking === r.part}
                <div class="deny">
                  <label class="k" for="{uid}-reason-{r.part}">Reason (the owner sees it)</label>
                  <textarea id="{uid}-reason-{r.part}" class="ui-input" rows="2" maxlength="500" bind:value={reason}></textarea>
                  <div class="btns">
                    <button class="ui-btn small danger" type="button" disabled={busyPart !== null || !reason.trim()} onclick={() => decide(r.part, 'DENY')}>{busyPart === r.part ? 'Denying…' : 'Confirm deny'}</button>
                    <button class="ui-btn small ghost" type="button" onclick={() => (asking = null)}>Cancel</button>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        {#if approvalError}<p class="ui-error" role="alert">{approvalError}</p>{/if}
      </section>
    {/if}

    {#if canTransfer}
      <section class="part" aria-label="Transfer ownership">
        <h3 class="lbl">Transfer ownership</h3>
        <form class="find" onsubmit={find} role="search">
          <input class="ui-input" type="search" bind:value={query} placeholder="Name, or a profile link" aria-label="Find an account by name or profile link" autocomplete="off" spellcheck="false" />
          <button class="ui-btn secondary" type="submit" disabled={searching || !query.trim()}>{searching ? 'Searching…' : 'Search'}</button>
        </form>
        {#if results}
          {#if results.length === 0}
            <p class="hint">No account found.</p>
          {:else}
            <ul class="rows" aria-label="Accounts found">
              {#each results as a (a.id)}
                <li>
                  <button type="button" class="pick" aria-pressed={picked?.id === a.id} onclick={() => { picked = a; confirming = false; }}>
                    <bdi>{clean(a.displayName)}</bdi>{#if picked?.id === a.id}<Icon name="check" size={14} />{/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
        {#if picked}
          <div class="btns">
            {#if confirming}
              <button class="ui-btn danger" type="button" disabled={transferring} onclick={transfer}>{transferring ? 'Moving…' : `Confirm: give to ${clean(picked.displayName)}`}</button>
              <button class="ui-btn ghost" type="button" onclick={() => (confirming = false)}>Cancel</button>
            {:else}
              <button class="ui-btn secondary" type="button" onclick={() => (confirming = true)}>Transfer to {clean(picked.displayName)}</button>
            {/if}
          </div>
        {/if}
        {#if transferError}<p class="ui-error" role="alert">{transferError}</p>{/if}
      </section>
    {/if}
  </div>
  {#snippet footer()}
    <button class="ui-btn secondary" type="button" disabled={busy} onclick={() => dialog.close()}>Close</button>
  {/snippet}
</Dialog>

<style>
  :global(.edit-emote) { width: min(560px, calc(100vw - 24px)); }
  .parts { display: grid; gap: 20px; }
  .part { display: grid; gap: 10px; min-width: 0; padding-top: 16px; border-top: 1px solid var(--border); }
  .part:first-child { padding-top: 0; border-top: 0; }
  .lbl { margin: 0; font-size: var(--t-label, 12px); font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .field { display: grid; gap: 4px; min-width: 0; }
  .k { font-size: var(--t-small); font-weight: 600; color: var(--muted); }
  .hint { font-size: var(--t-small); color: var(--muted); margin: 0; }
  .check { display: flex; align-items: flex-start; gap: 10px; min-height: 44px; padding: 4px 0; cursor: pointer; }
  .check input { flex: none; width: 18px; height: 18px; margin: 2px 0 0; accent-color: var(--accent); }
  .check > span { display: grid; gap: 1px; min-width: 0; }
  .rows { list-style: none; margin: 0; padding: 0; display: grid; }
  .arow { display: grid; gap: 8px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .line { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; }
  .line .k { min-width: 100px; }
  .state { display: inline-flex; align-items: center; gap: 4px; font-size: var(--t-ui); font-weight: 600; flex: 1; min-width: 80px; }
  .state.ok :global(.icon) { color: var(--accent-text); }
  .state.wait, .state.off { color: var(--muted); }
  .btns { display: flex; flex-wrap: wrap; gap: 6px; }
  .deny { display: grid; gap: 6px; }
  .deny textarea { height: auto; padding: 8px 10px; resize: vertical; }
  .find { display: flex; gap: 8px; }
  .find input { flex: 1; min-width: 0; }
  .pick { width: 100%; display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding: 0 10px; border: 0; border-bottom: 1px solid var(--border);
    background: none; color: var(--text); font: inherit; font-size: var(--t-ui); text-align: left; cursor: pointer; }
  .pick[aria-pressed='true'] { font-weight: 700; }
  .pick[aria-pressed='true'] :global(.icon) { color: var(--accent-text); }
  .pick:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .pick:hover { background: var(--surface-2); } }
  @media (pointer: coarse) { .pick { min-height: 44px; } }
  p { margin: 0; }
</style>
