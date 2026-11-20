<script lang="ts" module>
  import { can } from '$lib/tool/session.svelte';

  /** Who sees the panel: `emotes.flags`, or `emotes.edit_any` which covers it (ADR 0024). */
  export const canEditFlags = () => can('emotes.flags') || can('emotes.edit_any');
</script>

<script lang="ts">
  // Emote flags, for staff with emotes.flags (ADR 0024): the visibility (taking an emote off the
  // listings makes it unlisted), zero-width and NSFW of anyone's emote, in any state. Saved with
  // setEmoteFlags, which audits it and tells the owner when the visibility changes. Renders
  // nothing for anyone else. `bare` leaves out the panel frame, for use inside another panel.
  //   <EmoteFlagsPanel emote={e} onsaved={(next) => ...} />
  import { gql, type EmoteVisibility } from '$lib/api';
  import { btnClass, btnFamily } from '$lib/components/btn';
  import { clean } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { SET_EMOTE_FLAGS } from '$lib/queries';
  import EmoteFlags from '$lib/tool/EmoteFlags.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { visibilityOf } from '$lib/tool/visibility';

  type Flagged = { id: string; name: string; visibility?: EmoteVisibility | null; flags: { private: boolean; nsfw: boolean; zeroWidth: boolean } };
  type Props = { emote: Flagged; onsaved?: (e: EmoteFieldsFragment) => void; id?: string; bare?: boolean };
  let { emote, onsaved, id = 'flags', bare = false }: Props = $props();

  // The emote page is a public page, the review and user pages are tool pages: the buttons follow.
  const family = btnFamily();
  const shown = $derived(canEditFlags());

  type Flags = { visibility: EmoteVisibility; nsfw: boolean; zeroWidth: boolean };
  const of = (e: Flagged): Flags => ({ visibility: visibilityOf(e), nsfw: e.flags.nsfw, zeroWidth: e.flags.zeroWidth });
  // What the API last said (the emote given, then each save's answer) and what the form holds.
  let base = $state<Flags>({ visibility: 'PUBLIC', nsfw: false, zeroWidth: false });
  let visibility = $state<EmoteVisibility>('PUBLIC');
  let nsfw = $state(false);
  let zeroWidth = $state(false);
  let saving = $state(false);
  let problem = $state('');
  function load(f: Flags) {
    base = f;
    visibility = f.visibility;
    nsfw = f.nsfw;
    zeroWidth = f.zeroWidth;
    problem = '';
  }
  $effect(() => load(of(emote)));
  const changed = $derived(visibility !== base.visibility || nsfw !== base.nsfw || zeroWidth !== base.zeroWidth);

  async function save(ev: SubmitEvent) {
    ev.preventDefault();
    if (!changed || saving) return;
    saving = true;
    problem = '';
    const moved = visibility !== base.visibility;
    try {
      const next = (await gql(SET_EMOTE_FLAGS, { id: emote.id, input: { visibility, nsfw, zeroWidth } })).setEmoteFlags;
      load(of(next));
      notify(`Flags of ${clean(emote.name)} saved.${moved ? ' Its owner is told about the new visibility.' : ''}`, false);
      onsaved?.(next);
    } catch (err) {
      problem = err instanceof Error && err.message ? err.message : 'That didn’t save.';
    } finally {
      saving = false;
    }
  }
</script>

{#snippet form()}
  <form class="body" onsubmit={save}>
    <EmoteFlags {id} staff bind:visibility bind:nsfw bind:zeroWidth disabled={saving} />
    <p class="ui-hint">Changes are audited. The owner is told when the visibility changes.</p>
    {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    <div class="ui-actions">
      <button class={btnClass(family, 'primary')} type="submit" disabled={!changed || saving}>{saving ? 'Saving…' : 'Save flags'}</button>
      {#if changed && !saving}<button class={btnClass(family, 'ghost')} type="button" onclick={() => load(base)}>Undo changes</button>{/if}
    </div>
  </form>
{/snippet}

{#if shown}
  {#if bare}
    {@render form()}
  {:else}
    <section class="ui-panel" aria-labelledby="{id}-h">
      <header><h2 id="{id}-h">Emote flags</h2></header>
      {@render form()}
    </section>
  {/if}
{/if}

<style>
  .body { display: grid; gap: 12px; padding: 12px 16px 16px; }
  .body > p { margin: 0; }
</style>
