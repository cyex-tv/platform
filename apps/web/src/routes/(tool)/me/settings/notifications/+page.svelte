<script lang="ts">
  // Settings → Notifications, saved to your account: per kind, whether it notifies you at all
  // (off: cyex.tv doesn't keep it, so it isn't in the bell or the list either) and whether it pops
  // up while you are on the site. Hand-over requests and takedowns always notify. Each switch
  // saves on its own; a failed save puts it back.
  import { onMount } from 'svelte';
  import Problem from '$lib/components/Problem.svelte';
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
    <div class="ui-stack">
      {#each groups as g (g.id)}
        <section class="ui-panel" aria-labelledby="g-{g.id}">
          <header class="head">
            <h2 id="g-{g.id}">{g.label}</h2>
            <span class="cols" aria-hidden="true"><span>Notify</span><span>Pop up</span></span>
          </header>
          <ul class="ui-list">
            {#each g.items as p (p.kind)}
              <li class="pref">
                <span class="what" id="k-{p.kind}">
                  <strong>{p.label}</strong>
                  <span class="ui-hint">{p.hint}</span>
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
              </li>
            {/each}
          </ul>
        </section>
      {/each}
      <p class="ui-hint foot">Turned off, a kind isn’t kept at all: it won’t be in the bell or on your notifications page later either.</p>
    </div>
  {/if}
</div>

<style>
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .cols { display: flex; gap: 0; font-size: var(--t-fine); font-weight: 600; color: var(--muted); }
  .cols span { width: 64px; text-align: center; }
  .pref { display: flex; align-items: center; gap: 12px; }
  .what { display: grid; flex: 1 1 12rem; min-width: 0; }
  .what strong { font-weight: 500; }
  .switches { flex: none; display: flex; }
  .sw { display: grid; justify-items: center; width: 64px; cursor: pointer; }
  .sw:has(input:disabled) { cursor: default; }
  .sw input:disabled { opacity: .5; cursor: default; }
  .lab { display: none; font-size: var(--t-fine); color: var(--muted); }
  .foot { margin: 0; padding: 0 2px; }
  /* Phones: the column heads go; each switch says what it is under it. */
  @media (max-width: 560px) {
    .cols { display: none; }
    .pref { flex-wrap: wrap; }
    .switches { width: 100%; justify-content: flex-end; }
    .sw { display: flex; align-items: center; gap: 4px; width: auto; padding-left: 12px; }
    .lab { display: inline; }
  }
</style>
