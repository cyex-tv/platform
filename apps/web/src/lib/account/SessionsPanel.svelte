<script lang="ts">
  // Where you are signed in: this device (marked), the website and apps elsewhere, each with a
  // Revoke. "Sign out everywhere else" ends all but this one and says how many. Revoking this
  // device signs you out here too, so that row says "Sign out" and arms first.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
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

<section class="ui-panel" aria-labelledby="sessions-h">
  <header>
    <h2 id="sessions-h">Sessions</h2>
    {#if L.data && L.data.some((s) => !s.current && !gone[s.id])}
      <button class="ui-btn secondary small" type="button" disabled={busy} onclick={others}>Sign out everywhere else</button>
    {/if}
  </header>
  <LoadGate source={L} label="Loading sessions…">
    {#snippet children(list)}
      {#if list.length === 0}
        <p class="ui-hint pad">No sessions to show.</p>
      {:else}
        <ul class="ui-list">
          {#each list as s (s.id)}
            <li class:current={s.current} class:gone={gone[s.id]}>
              <Icon name={s.kind === 'APP' ? 'tv' : 'user'} />
              <span class="what">
                <strong title={s.device}><bdi>{s.device}</bdi></strong>
                <span class="ui-hint meta">
                  {#if s.current}<span class="here">This device</span>{' · '}{/if}{s.kind === 'APP' ? 'App' : 'Website'} · Last seen <time datetime={s.lastSeenAt} title={relative(s.lastSeenAt)}>{dateTime(s.lastSeenAt)}</time>
                </span>
              </span>
              {#if s.current}
                <Confirm small label="Sign out" confirm="Click again to sign out" busy={busy} onconfirm={() => revoke(s.id, true)} />
              {:else if gone[s.id]}
                <span class="ui-hint done">Signed out</span>
              {:else}
                <button class="ui-btn secondary small" type="button" disabled={busy} onclick={() => revoke(s.id, false)}>Revoke</button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
  </LoadGate>
</section>

<style>
  .what { display: grid; flex: 1 1 10rem; min-width: 0; }
  .what strong { display: block; min-width: 0; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { overflow-wrap: anywhere; }
  .here { font-weight: 600; color: var(--text); }
  li > :global(svg) { color: var(--muted); }
  .current > :global(svg) { color: var(--accent-text); }
  .current .what strong { font-weight: 600; }
  .done { flex: none; padding-inline: 8px; white-space: nowrap; }
  .gone .what { opacity: .6; }
  .pad { padding: 10px 14px 12px; }
  @media (max-width: 560px) {
    .ui-list > li { flex-wrap: wrap; }
  }
  @media (max-width: 480px) {
    .ui-panel > header { flex-wrap: wrap; }
  }
</style>
