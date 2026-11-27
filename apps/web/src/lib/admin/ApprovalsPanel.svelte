<script lang="ts">
  // An upload's two approvals (ADR 0038) for staff, after the first verdict: public listing and
  // personal use, each with its state and an Approve / Deny that changes it (reviewApprovals with
  // onlyPending false, audited, the owner is told). A denial asks for a reason the owner sees. It
  // replaces the old Emote flags panel in the review detail and on the account page. A reviewer
  // holds emotes.review for the listing and emotes.review_personal for personal use; a part they
  // can't change shows its state only. Rejected and taken-down emotes can't be reviewed this way.
  //   <ApprovalsPanel emote={e} onsaved={(a) => ...} />
  import { gql } from '$lib/api';
  import { clean } from '$lib/format';
  import { REVIEW_APPROVALS } from '$lib/queries';
  import { notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { APPROVAL_LABEL, approvalsInput, type Approvals, type Decision } from './queue';

  type Props = {
    emote: { id: string; name: string; state: string; approvals?: Approvals | null };
    /** The API took the change: the emote's approvals now. */
    onsaved?: (a: Approvals) => void;
    id?: string;
  };
  let { emote, onsaved, id = 'approvals' }: Props = $props();

  const PARTS = [
    { key: 'listing' as const, label: 'Public listing', field: 'publicListing' as const, perm: 'emotes.review', hint: 'Found in search and browse, and on its creator’s page.' },
    { key: 'personal' as const, label: 'Personal use', field: 'personalUse' as const, perm: 'emotes.review_personal', hint: 'May be a personal emote, shown in chat everywhere its owner talks.' },
  ];

  // What the API last said (the emote given, then each save's answer).
  let now = $state<Approvals>({ publicListing: 'PENDING', personalUse: 'PENDING' });
  $effect(() => {
    if (emote.approvals) now = { ...emote.approvals };
  });
  const locked = $derived(emote.state === 'REJECTED' || emote.state === 'TAKEN_DOWN');

  let saving = $state(false);
  let problem = $state('');
  // The denial being written: which part, and why.
  let denying = $state<'listing' | 'personal' | null>(null);
  let reason = $state('');

  async function save(d: Decision, why?: string) {
    if (saving) return;
    saving = true;
    problem = '';
    try {
      const out = (await gql(REVIEW_APPROVALS, { ids: [emote.id], input: { ...approvalsInput(d, why), onlyPending: false }, idempotencyKey: crypto.randomUUID() })).reviewApprovals;
      if (out.failed.length) {
        problem = out.failed[0].message || 'That didn’t go through.';
        return;
      }
      const next = { ...now };
      if (d.listing) next.publicListing = d.listing === 'APPROVE' ? 'APPROVED' : 'DENIED';
      if (d.personal) next.personalUse = d.personal === 'APPROVE' ? 'APPROVED' : 'DENIED';
      now = next;
      denying = null;
      reason = '';
      notify(`Approvals of ${clean(emote.name)} saved. Its owner is told.`, false);
      onsaved?.(next);
    } catch (err) {
      problem = err instanceof Error && err.message ? err.message : 'That didn’t save.';
    } finally {
      saving = false;
    }
  }
  const approve = (k: 'listing' | 'personal') => save(k === 'listing' ? { listing: 'APPROVE' } : { personal: 'APPROVE' });
  function deny(ev: SubmitEvent) {
    ev.preventDefault();
    const why = reason.trim();
    if (!denying || !why) return;
    void save(denying === 'listing' ? { listing: 'DENY' } : { personal: 'DENY' }, why);
  }
</script>

<div class="ap" id={id}>
  <ul class="rows" aria-label="Approvals">
    {#each PARTS as p (p.key)}
      {@const value = now[p.field]}
      <li>
        <div class="what">
          <span class="nm">{p.label}</span>
          <span class="st" class:ok={value === 'APPROVED'} class:bad={value === 'DENIED'}>{APPROVAL_LABEL[value] ?? value}</span>
        </div>
        <p class="ui-hint">{p.hint}</p>
        {#if !locked && can(p.perm)}
          <div class="acts">
            {#if value !== 'APPROVED'}<button class="ui-btn secondary small" type="button" disabled={saving} onclick={() => approve(p.key)}>Approve</button>{/if}
            {#if value !== 'DENIED'}<button class="ui-btn secondary small" type="button" disabled={saving} aria-expanded={denying === p.key} aria-controls="{id}-deny" onclick={() => ((denying = denying === p.key ? null : p.key), (reason = ''))}>Deny…</button>{/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
  {#if locked}
    <p class="ui-hint">A rejected or taken-down emote can’t be reviewed again.</p>
  {/if}
  {#if denying}
    <form class="deny" id="{id}-deny" onsubmit={deny}>
      <label class="ui-field">
        <span class="ui-label">Why {denying === 'listing' ? 'it isn’t listed' : 'it isn’t a personal emote'}</span>
        <textarea class="ui-input" rows="2" maxlength="500" bind:value={reason} disabled={saving} required placeholder="The owner sees this. Be short and concrete."></textarea>
      </label>
      <div class="ui-actions">
        <button class="ui-btn danger small" type="submit" disabled={saving || !reason.trim()}>{saving ? 'Saving…' : denying === 'listing' ? 'Deny public listing' : 'Deny personal use'}</button>
        <button class="ui-btn ghost small" type="button" disabled={saving} onclick={() => ((denying = null), (reason = ''))}>Cancel</button>
      </div>
    </form>
  {/if}
  {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
</div>

<style>
  .ap { display: grid; gap: 12px; padding: 12px 0 4px; }
  .ap p { margin: 0; }
  .rows { list-style: none; margin: 0; padding: 0; display: grid; border: 1px solid var(--border); }
  .rows li { display: grid; gap: 4px; padding: 10px 12px; }
  .rows li + li { border-top: 1px solid var(--border); }
  .what { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; }
  .nm { font-size: var(--t-small); font-weight: 600; }
  .st { font-size: var(--t-small); color: var(--muted); }
  .st.ok { color: var(--green); }
  .st.bad { color: var(--red); }
  .acts { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 4px; }
  .deny { display: grid; gap: 8px; }
</style>
