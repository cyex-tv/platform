<script lang="ts">
  // The site announcement, one line under the header on every page, set by staff on
  // /admin/announcements. Flat on the page's surface with a hairline: the kind's icon and word
  // (News, Maintenance, Event), an optional small emote or badge, the line, and its link (the
  // only accent). × closes it on this device; a new announcement shows again. It scrolls away
  // with the page (the header alone stays) and never moves anything once shown: it only appears
  // on load, before the reader has started.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import { loadCatalog, type Badge } from '$lib/cosmetics/store.svelte';
  import { ready } from '$lib/emote';
  import { dur } from '$lib/motion';
  import { KIND, external, running, safeLink } from './announcement';
  import { banner } from './announcement.svelte';

  onMount(() => banner.load());

  const a = $derived(banner.shown && running(banner.shown, Date.now()) ? banner.shown : null);
  const href = $derived(safeLink(a?.linkUrl));
  let badge = $state<Badge | null>(null);
  $effect(() => {
    const id = a?.badgeId;
    badge = null;
    if (!id) return;
    loadCatalog()
      .then((c) => {
        if (a?.badgeId === id) badge = c.badges.get(id) ?? null;
      })
      .catch(() => {});
  });
</script>

{#if a}
  {@const kind = KIND[a.kind]}
  <aside class="announce" aria-label="Announcement" out:fade={{ duration: dur(140) }}>
    <div class="wrap row">
      <span class="kind"><Icon name={kind.icon} size={15} /><span class="word">{kind.label}</span></span>
      {#if a.emote && ready(a.emote)}
        <span class="art"><EmotePicture emote={a.emote} alt="" class="pic" eager /></span>
      {:else if badge}
        <span class="art"><BadgeIcon {badge} size={18} /></span>
      {/if}
      <p class="text">
        <bdi>{a.text}</bdi>
        {#if href}
          <a class="go" {href} target={external(href) ? '_blank' : undefined} rel={external(href) ? 'noopener noreferrer' : undefined}
            >{a.linkLabel || 'Learn more'}<Icon name="chevron-right" size={14} /></a
          >
        {/if}
      </p>
      <button class="close" type="button" aria-label="Close the announcement" title="Close" onclick={() => banner.close(a.id)}><Icon name="x" size={14} /></button>
    </div>
  </aside>
{/if}

<style>
  .announce { background: var(--surface); border-bottom: 1px solid var(--border); }
  .row { display: flex; align-items: center; gap: 12px; min-height: 44px; padding-block: 6px; }
  .kind { flex: none; display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: var(--t-fine); font-weight: 600; }
  .kind :global(svg) { color: var(--dim); }
  .art { flex: none; display: inline-flex; align-items: center; }
  .art :global(.pic) { height: 22px; width: auto; max-width: 66px; object-fit: contain; }
  .text { flex: 1; min-width: 0; margin: 0; font-size: var(--t-nav); font-weight: 500; color: var(--text); line-height: 1.4; overflow-wrap: anywhere; }
  .go { display: inline-flex; align-items: center; gap: 2px; margin-left: 8px; color: var(--accent-text); font-weight: 600; white-space: nowrap; }
  .go :global(svg) { transition: translate var(--fast) var(--ease); }
  .go:active { opacity: .6; }
  .close { position: relative; flex: none; display: grid; place-items: center; width: 32px; height: 32px; margin-right: -6px; padding: 0;
    border: 0; border-radius: var(--radius); background: none; color: var(--dim); cursor: pointer; transition: color var(--fast), background-color var(--fast); }
  .close::after { content: ''; position: absolute; inset: -6px; }
  .close:active { opacity: .6; }
  @media (hover: hover) and (pointer: fine) {
    .go:hover { text-decoration: underline; text-underline-offset: 3px; }
    .go:hover :global(svg) { translate: 2px 0; }
    .close:hover { color: var(--text); background: var(--row-hover); }
  }
  /* Phones: the kind's word goes (its icon stays), so the line gets the room. */
  @media (max-width: 560px) {
    .word { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
    .row { gap: 10px; align-items: flex-start; }
    .kind, .art { min-height: 32px; }
    .text { padding-top: 6px; }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .go :global(svg) { transition: none; } }
  :global(html[data-motion='reduce']) .go :global(svg) { transition: none; }
  @media (forced-colors: active) { .announce { border-bottom-color: CanvasText; } }
</style>
