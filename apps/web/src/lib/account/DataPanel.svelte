<script lang="ts">
  // Your data: one button that asks the API for a copy of everything about the account and saves
  // it as cyex-tv-export-<date>.json. The API allows five an hour; past that it says so in words.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import { EXPORT_MY_DATA } from '$lib/queries';
  import { Gate } from '$lib/tool/gate.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { exportMessage } from './errors';
  import { exportFilename, saveJson } from './export';

  // One export at a time and a short rest after it: the API allows five an hour, a double click mustn't burn two.
  const gate = new Gate(1500);
  let problem = $state('');
  let preparing = $state(false);

  const run = () =>
    gate.run(async () => {
      preparing = true;
      problem = '';
      try {
        const json = (await gql(EXPORT_MY_DATA, {})).exportMyData;
        saveJson(json, exportFilename());
        notify('Your data was saved.');
      } catch (err) {
        problem = exportMessage(err);
      } finally {
        preparing = false;
      }
    });
</script>

<section class="ui-panel" aria-labelledby="data-h">
  <header><h2 id="data-h">Your data</h2></header>
  <p class="ui-hint pad">A copy of everything cyex.tv holds about your account (logins, emotes, sets, channels, editors, cosmetics, notifications), as one JSON file.</p>
  <div class="ui-actions row pad">
    <button class="ui-btn secondary" type="button" disabled={gate.busy} onclick={run}><Icon name="download" size={16} />{preparing ? 'Preparing…' : 'Export my data'}</button>
  </div>
  {#if problem}<p class="problem pad" role="alert">{problem}</p>{/if}
</section>

<style>
  .pad { padding: 10px 14px 12px; }
  .row { padding-top: 0; }
  .ui-hint.pad { padding-bottom: 12px; }
  .problem { padding-top: 0; font-size: var(--text-sub); color: var(--muted); }
</style>
