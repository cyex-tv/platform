<script lang="ts">
  // Settings → Notifications, saved to your account: per kind, whether it notifies you at all
  // (off: cyex.tv doesn't keep it, so it isn't in the bell or the list either) and whether it pops
  // up while you are on the site. Hand-over requests and takedowns always notify. Each switch
  // saves on its own; a failed save puts it back.
  import { onMount } from 'svelte';
  import Problem from '$lib/components/Problem.svelte';
  import Section from '$lib/components/Section.svelte';
  import State from '$lib/components/State.svelte';
  import { groupPrefs } from '$lib/account/prefs';
  import { loadPrefs, notePrefs, savePref } from '$lib/account/prefs.svelte';
  import { fail } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  onMount(() => void loadPrefs(true));

  const groups = $derived(notePrefs.items ? groupPrefs(notePrefs.items) : []);

  async function change(el: HTMLInputElement, kind: string, what: 'notify' | 'toast') {
    const want = el.checked;
    try {
      await savePref(kind, { [what]: want });
    } catch (err) {
      fail(err);
    }
    // Whatever was saved (or not) is what shows.
    const now = notePrefs.items?.find((p) => p.kind === kind);
    if (now) el.checked = now[what];
  }
</script>

<svelte:head><title>Notifications - Settings - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Notifications" line="What cyex.tv tells you about, and what pops up while you’re here. Saved to your account." />

  {#if notePrefs.items === null && notePrefs.error}
    <Problem cause="server" onretry={() => loadPrefs(true)} />
  {:else if notePrefs.items === null}
    <State kind="loading" title="Loading your settings…" />
  {:else}
    {#each groups as g (g.id)}
      <Section title={g.label}>
        {#snippet actions()}<span class="cols" aria-hidden="true"><span>Notify</span><span>Pop up</span></span>{/snippet}
        {#each g.items as p (p.kind)}
          <div class="pref">
            <span class="what" id="k-{p.kind}">
              <strong>{p.label}</strong>
              <span class="hint">{p.hint}</span>
            </span>
            <span class="switches">
              <label class="sw" title={p.required ? 'Always on' : undefined}>
                <span class="lab">Notify</span>
                <input
                  class="ui-switch"
                  type="checkbox"
                  checked={p.notify}
                  disabled={p.required || notePrefs.saving !== null}
                  aria-describedby="k-{p.kind}"
                  aria-label="{p.label}: notify me{p.required ? ' (always on)' : ''}"
                  onchange={(e) => change(e.currentTarget, p.kind, 'notify')}
                />
              </label>
              <label class="sw" title={!p.notify ? 'Turn Notify on first' : undefined}>
                <span class="lab">Pop up</span>
                <input
                  class="ui-switch"
                  type="checkbox"
                  checked={p.notify && p.toast}
                  disabled={!p.notify || notePrefs.saving !== null}
                  aria-describedby="k-{p.kind}"
                  aria-label="{p.label}: pop up"
                  onchange={(e) => change(e.currentTarget, p.kind, 'toast')}
                />
              </label>
            </span>
          </div>
        {/each}
      </Section>
    {/each}
    <p class="foot">Turned off, a kind isn’t kept at all: it won’t be in the bell or on your notifications page later either.</p>
  {/if}
</div>

<style>
  .cols { display: flex; font-size: var(--t-fine); font-weight: 600; color: var(--muted); }
  .cols span { width: 64px; text-align: center; }
  .pref { display: flex; align-items: center; gap: 16px; min-height: 52px; padding: 10px 0; border-bottom: 1px solid var(--border); }
  .what { display: grid; gap: 2px; flex: 1 1 12rem; min-width: 0; }
  .what strong { font-weight: 500; font-size: var(--t-ui); }
  .hint { font-size: var(--t-small); line-height: 1.45; color: var(--muted); text-wrap: pretty; }
  .switches { flex: none; display: flex; }
  .sw { display: grid; justify-items: center; width: 64px; cursor: pointer; }
  .sw:has(input:disabled) { cursor: default; }
  .sw input:disabled { opacity: .5; cursor: default; }
  .lab { display: none; font-size: var(--t-fine); color: var(--muted); }
  .foot { margin: 12px 0 0; font-size: var(--t-small); color: var(--muted); }
  /* Phones: the column heads go; each switch says what it is next to it. */
  @media (max-width: 560px) {
    .cols { display: none; }
    .pref { flex-wrap: wrap; }
    .switches { width: 100%; justify-content: flex-start; gap: 16px; }
    .sw { display: flex; align-items: center; gap: 8px; width: auto; min-height: 44px; }
    .lab { display: inline; white-space: nowrap; }
  }
</style>
