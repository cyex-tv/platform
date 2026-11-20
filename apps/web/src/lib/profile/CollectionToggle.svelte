<script lang="ts">
  // "Show my collection on my profile": one switch, saved at once to your account
  // (setCollectionPublic). Off by default. Used on the Collection tab and in Settings → Account.
  import { gql } from '$lib/api';
  import type { AccountCollectionQuery } from '$lib/gql/graphql';
  import { SET_COLLECTION_PUBLIC } from '$lib/queries';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Collection = NonNullable<AccountCollectionQuery['accountCollection']>;
  type Props = { value: boolean; onchange?: (c: Collection) => void; id?: string };
  let { value, onchange, id = 'collection-public' }: Props = $props();
  let busy = $state(false);

  async function change(el: HTMLInputElement) {
    const want = el.checked;
    busy = true;
    try {
      const c = (await gql(SET_COLLECTION_PUBLIC, { public: want })).setCollectionPublic;
      el.checked = c.public;
      notify(c.public ? 'Your collection is on your profile now.' : 'Your collection is private now.');
      onchange?.(c);
    } catch (err) {
      el.checked = value;
      fail(err);
    } finally {
      busy = false;
    }
  }
</script>

<label class="ui-row toggle" for={id}>
  <div>
    <strong>Show my collection on my profile</strong>
    <span class="ui-hint">Everyone sees the badges and paints you own, since when, and whether they came from the shop, a code, an event or staff. Never who gave them or what they cost.</span>
  </div>
  <input class="ui-switch" type="checkbox" {id} checked={value} disabled={busy} onchange={(e) => change(e.currentTarget)} />
</label>

<style>
  .toggle { cursor: pointer; padding: 12px 14px; }
  .toggle strong { font-size: var(--t-ui); }
  .toggle .ui-hint { display: block; margin-top: 2px; max-width: 64ch; }
</style>
