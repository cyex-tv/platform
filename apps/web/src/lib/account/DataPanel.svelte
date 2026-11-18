<script lang="ts">
  // Your data: one button that asks the API for a copy of everything about the account and saves
  // it as cyex-tv-export-<date>.json. The API allows five an hour; past that it says so in words.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import { EXPORT_MY_DATA } from '$lib/queries';
  import { notify } from '$lib/tool/notices.svelte';
  import { exportMessage } from './errors';
  import { exportFilename, saveJson } from './export';

  let busy = $state(false);
  let problem = $state('');

  async function run() {
    busy = true;
    problem = '';
    try {
      const json = (await gql(EXPORT_MY_DATA, {})).exportMyData;
      saveJson(json, exportFilename());
      notify('Your data was saved.');
    } catch (err) {
      problem = exportMessage(err);
    } finally {
      busy = false;
    }
  }
</script>

<section class="ui-card" aria-labelledby="data-h">
  <h2 id="data-h">Your data</h2>
  <p class="ui-hint">A copy of everything cyex.tv holds about your account (logins, emotes, sets, channels, editors, cosmetics, notifications), as one JSON file.</p>
  <div class="ui-actions row">
    <button class="ui-btn secondary" type="button" disabled={busy} onclick={run}><Icon name="download" size={16} />{busy ? 'Preparing…' : 'Export my data'}</button>
  </div>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</section>

<style>
  .row { margin-top: 12px; }
  .problem { margin-top: 10px; font-size: var(--text-sub); color: var(--muted); }
</style>
