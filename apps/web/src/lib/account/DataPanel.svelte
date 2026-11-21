<script lang="ts">
  // Your data: one button that asks the API for a copy of everything about the account and saves
  // it as cyex-tv-export-<date>.json. The API allows five an hour; past that it says so in words.
  import { gql } from '$lib/api';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
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

<Section title="Your data" id="data-h">
  <Row label="Export my data" hint="A copy of everything cyex.tv holds about your account (logins, emotes, sets, channels, editors, cosmetics, notifications), as one JSON file.">
    <button class="ui-btn secondary small" type="button" disabled={gate.busy} onclick={run}><Icon name="download" size={16} />{preparing ? 'Preparing…' : 'Export my data'}</button>
  </Row>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</Section>

<style>
  .problem { margin: 8px 0 0; font-size: var(--t-small); color: var(--muted); }
</style>
