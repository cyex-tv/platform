<script lang="ts">
  // The item on narrow windows (under 1100px), where the shop has no right panel: ItemBuy in a
  // modal dialog. It opens and closes with `item`; every opening starts clean (keyed by the item).
  // Starting the purchase closes it (Paddle's checkout opens above the page, and a modal would
  // hide it); the shop page then shows the progress (CheckoutStatus).
  import { tick } from 'svelte';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import ItemBuy from './ItemBuy.svelte';

  type Props = {
    item: ShopItemFieldsFragment | null;
    /** Open on "Gift to someone" (the card's Gift button, or an owned item). */
    gift?: boolean;
    name: string;
    signedIn: boolean;
    onclose: () => void;
  };
  let { item, gift = false, name, signedIn, onclose }: Props = $props();

  let dialog: HTMLDialogElement;
  let buy = $state<ItemBuy>();

  $effect(() => {
    if (item && !dialog.open) {
      // The form is drawn for this item on the next tick; opening after it lets focus land on the
      // first control (who gets it, or the login for a gift), not the dialog itself.
      tick().then(() => {
        if (!item || dialog.open) return;
        dialog.showModal();
        buy?.focusStart();
      });
    } else if (!item && dialog.open) dialog.close();
  });
</script>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="shop-item-title" onclose={onclose}>
  {#if item}
    {#key `${item.id}:${gift}`}
      <ItemBuy bind:this={buy} {item} {gift} {name} {signedIn} onclose={() => dialog.close()} onstarted={() => dialog.close()} showing={() => !!dialog?.open} />
    {/key}
  {/if}
</dialog>

<style>
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  .wide :global(form) { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
</style>
