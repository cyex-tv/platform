<script lang="ts">
  import { ready, size, src, srcset, type Picture } from '$lib/emote';

  type Item = Picture & { id: string; name: string; owner?: { displayName: string } | null; shownName?: string };
  let { items, empty = 'Nothing here yet.' }: { items: Item[]; empty?: string } = $props();
</script>

{#if items.length === 0}
  <p class="muted none">{empty}</p>
{:else}
  <ul class="grid">
    {#each items as e (e.id + (e.shownName ?? ''))}
      <li>
        <a class="tile" href="/emotes/{e.id}">
          <span class="pic">
            {#if ready(e)}
              <img
                src={src(e, 1)}
                srcset={srcset(e)}
                alt=""
                width={size(e).width}
                height={size(e).height}
                loading="lazy"
                decoding="async"
              />
            {:else}
              <span class="fine">Processing</span>
            {/if}
          </span>
          <span class="name">{e.shownName ?? e.name}</span>
          {#if e.owner}<span class="by">{e.owner.displayName}</span>{/if}
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .none { padding: 24px 0; }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px;
    grid-template-columns: repeat(auto-fill, minmax(132px, 1fr)); }
  .tile { display: grid; justify-items: center; gap: 4px; padding: 18px 10px 14px; border-radius: var(--radius-m);
    border: 1px solid var(--border); background: var(--surface); min-height: 132px; text-align: center;
    transition: border-color var(--dur-hover), background-color var(--dur-hover), transform var(--dur-press) ease-out; }
  /* A link changes colour and presses; it doesn't rise (cyex-web rule 9). */
  .tile:hover { border-color: var(--border-strong); background: var(--surface-2); }
  .tile:active { transform: scale(var(--press)); }
  .pic { display: grid; place-items: center; height: 56px; width: 100%; }
  img { max-height: 56px; max-width: 100%; width: auto; height: auto; image-rendering: auto; }
  .name { font-size: var(--t-ui); font-weight: 500; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .by { font-size: var(--t-fine); color: var(--muted); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
