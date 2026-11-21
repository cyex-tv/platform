<script lang="ts">
  // The site announcement, one line under the header on every page, set by staff on
  // /admin/announcements. Flat on the page's surface with a hairline: the kind's icon and word
  // (News, Maintenance, Event), an optional small emote or badge, the line, and its link (the
  // only accent). × closes it on this device; a new announcement shows again. It scrolls away
  // with the page (the header alone stays) and never moves anything: the root layout's server
  // load reads it (and this device's closed list), so it is in the first paint or not at all. A
  // prerendered page can't know it: there the browser asks, and the place is held from the first
  // paint when this device saw a live one last time (app.html's `data-announce` hint).
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import { loadCatalog, type Badge } from '$lib/cosmetics/store.svelte';
  import { ready } from '$lib/emote';
  import { dur } from '$lib/motion';
  import { KIND, external, running, safeLink } from './announcement';
  import { banner, type Announcement } from './announcement.svelte';

  type Server = { announcement?: Announcement | null; announceClosed?: string[] };
  const server = $derived(page.data as Server);
  // Prerendered pages send no answer (undefined): the browser asks for itself.
  const unknown = $derived(server.announcement === undefined && banner.current === undefined);
  onMount(() => banner.load(server.announcement === undefined));
  // The hint for the next prerendered page: which one is live, if any (once an answer is in).
  $effect(() => {
    if (!unknown) banner.remember(banner.current !== undefined ? banner.current : (server.announcement ?? null));
  });

  const shown = $derived(banner.shown(server));
  const a = $derived(shown && running(shown, Date.now()) ? shown : null);
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

{#if !a && unknown}
  <div class="hold" aria-hidden="true"></div>
{/if}
{#if a}
  {@const kind = KIND[a.kind]}
  <aside class="announce" aria-label="Announcement" out:fade={{ duration: dur(140) }}>
    <div class="wrap row">
      <span class="kind"><Icon name={kind.icon} size={15} /><span class="word">{kind.label}</span></span>
      {#if a.emote && ready(a.emote)}
        <span class="art"><EmotePicture emote={a.emote} alt="" class="pic" eager /></span>
      {:else if a.badgeId}
        <!-- The badge's place is kept while the catalogue loads, so the line never moves. -->
        <span class="art badge">{#if badge}<BadgeIcon {badge} size={18} />{/if}</span>
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
  /* The bar's place while a prerendered page asks, when this device saw one live last time. */
  .hold { display: none; }
  :global(html[data-announce]) .hold { display: block; height: 45px; background: var(--surface); border-bottom: 1px solid var(--border); }
  .row { display: flex; align-items: center; gap: 12px; min-height: 44px; padding-block: 6px; }
  .kind { flex: none; display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: var(--t-fine); font-weight: 600; }
  .kind :global(svg) { color: var(--dim); }
  .art { flex: none; display: inline-flex; align-items: center; }
  .art.badge { width: 18px; height: 18px; }
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
    :global(html[data-announce]) .hold { height: 58px; }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .go :global(svg) { transition: none; } }
  :global(html[data-motion='reduce']) .go :global(svg) { transition: none; }
  @media (forced-colors: active) { .announce { border-bottom-color: CanvasText; } }
</style>
