<script lang="ts">
  // Where you are signed in: this device (marked), the website and apps elsewhere, each with a
  // Revoke. "Sign out everywhere else" ends all but this one and says how many. Revoking this
  // device signs you out here too, so that row says "Sign out" and arms first.
  import { gql } from '$lib/api';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import { dateTime } from '$lib/format';
  import { relative } from '$lib/profile/activity';
  import { REVOKE_OTHER_SESSIONS, REVOKE_SESSION, MY_SESSIONS } from '$lib/queries';
  import { leftSession } from '$lib/shell/signout';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { Gate } from '$lib/tool/gate.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const L = loader(async ({ fetch }) => (await gql(MY_SESSIONS, {}, fetch)).mySessions);
  // One request at a time, and a short rest after it. A revoked row stays where it is and says
  // "Signed out", so the list never shifts under a second click.
  const gate = new Gate(400);
  const busy = $derived(gate.busy);
  let gone = $state<Record<string, true>>({});

  const revoke = (id: string, current: boolean) =>
    gate.run(async () => {
      try {
        await gql(REVOKE_SESSION, { id });
        if (current) {
          await leftSession();
          return;
        }
        gone[id] = true;
        notify('Signed out of that device.');
      } catch (err) {
        fail(err);
      }
    });

  const others = () =>
    gate.run(async () => {
      try {
        const n = (await gql(REVOKE_OTHER_SESSIONS, {})).revokeOtherSessions;
        for (const s of L.data ?? []) if (!s.current) gone[s.id] = true;
        notify(n === 0 ? 'You weren’t signed in anywhere else.' : n === 1 ? 'Signed out of 1 other device.' : `Signed out of ${n} other devices.`);
      } catch (err) {
        fail(err);
      }
    });
</script>

<Section title="Sessions" count={L.data?.length} id="sessions-h">
  {#snippet actions()}
    {#if L.data && L.data.some((s) => !s.current && !gone[s.id])}
      <button class="ui-btn secondary small" type="button" disabled={busy} onclick={others}>Sign out everywhere else</button>
    {/if}
  {/snippet}
  <LoadGate source={L} label="Loading sessions…">
    {#snippet children(list)}
      {#if list.length === 0}
        <p class="empty">No sessions to show.</p>
      {:else}
        {#each list as s (s.id)}
          <div class="s" class:current={s.current} class:gone={gone[s.id]}>
            <Row label={s.device} icon={s.kind === 'APP' ? 'tv' : s.kind === 'CONSOLE' ? 'key' : 'user'}>
              {#snippet hint()}
                {#if s.current}<span class="tag">This device</span>{' · '}{/if}{s.kind === 'APP' ? 'App' : s.kind === 'CONSOLE' ? 'API token' : 'Website'} · Last seen <time class="ui-mono" datetime={s.lastSeenAt} title={relative(s.lastSeenAt)}>{dateTime(s.lastSeenAt)}</time>
              {/snippet}
              {#if s.current}
                <Confirm small label="Sign out" confirm="Click again to sign out" busy={busy} onconfirm={() => revoke(s.id, true)} />
              {:else if gone[s.id]}
                <span class="done">Signed out</span>
              {:else}
                <button class="ui-btn secondary small" type="button" disabled={busy} onclick={() => revoke(s.id, false)}>Revoke</button>
              {/if}
            </Row>
          </div>
        {/each}
      {/if}
    {/snippet}
  </LoadGate>
</Section>

<style>
  .tag { font-weight: 600; color: var(--text); }
  .done { font-size: var(--t-small); color: var(--muted); white-space: nowrap; }
  .gone { opacity: .6; }
  .current :global(.ico) { color: var(--accent-text); }
  .empty { margin: 8px 0; font-size: var(--t-small); color: var(--muted); }
</style>
