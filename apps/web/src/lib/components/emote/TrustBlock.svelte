<script lang="ts">
  // The review block: status as the API returns it, who uploaded it and when, a plain line about
  // review, and Report (for approved emotes).
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { date, iso } from '$lib/format';
  import ReportForm from './ReportForm.svelte';
  import { trust, type PageEmote } from './emote-page';
  import type { Viewer } from './viewer.svelte';

  let { emote: e, viewer }: { emote: PageEmote; viewer: Viewer } = $props();
  const t = $derived(trust(e.state));
  const uploaded = $derived(date(e.createdAt));
  let open = $state(false);
</script>

<section class="trust" aria-labelledby="trust-h">
  <h2 id="trust-h">Review</h2>
  <p class="status {t.tone}">
    <span class="dot" aria-hidden="true">{#if t.tone === 'ok'}<Icon name="check" size={12} stroke={2.2} />{/if}</span>
    {t.status}
  </p>
  {#if uploaded}
    <p class="by">Uploaded on <time datetime={e.createdAt}>{uploaded}</time>.</p>
  {/if}
  <p class="fine line">{t.line}</p>
  {#if e.state === 'APPROVED'}
    <Popover label="Report {iso(e.name)}" bind:open sheet width="340px">
      {#snippet trigger(props)}
        <button class="btn btn-s report" type="button" {...props}><Icon name="flag" />Report this emote</button>
      {/snippet}
      <ReportForm {viewer} name={e.name} oncancel={() => (open = false)} />
    </Popover>
  {/if}
</section>

<style>
  p { margin: 0; }
  .trust { display: grid; gap: 10px; justify-items: start; padding: 20px; border: 1px solid var(--border); border-radius: var(--radius-m);
    background: var(--surface); }
  h2 { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; }
  .status { display: inline-flex; align-items: center; gap: 8px; font-size: var(--t-ui); font-weight: 500; }
  .dot { display: inline-grid; place-items: center; width: 18px; height: 18px; border-radius: 50%; border: 1px solid var(--border-strong); }
  .ok .dot { background: var(--text); border-color: var(--text); color: var(--bg); }
  .wait .dot { border-style: dashed; border-color: var(--muted); }
  .off .dot { background: var(--muted); border-color: var(--muted); }
  .by { font-size: var(--t-small); overflow-wrap: anywhere; }
  .line { max-width: 52ch; text-wrap: pretty; }
  .report { margin-top: 4px; gap: 8px; }
</style>
