<script lang="ts">
  // /guidelines: what may be uploaded to cyex.tv, in our own words. The upload form links here
  // (in a new tab) and asks you to accept it. Each rule has an anchor, so a moderator's note or a
  // help answer can point at one (/guidelines#nsfw). A reading page on the cyex docs layout, like
  // /help and /changelog: the rules listed in the index on the left, which follows the reader.
  import '$lib/docs/docs.css';
  import { onMount } from 'svelte';
  import { initDocs } from '$lib/docs/behaviour';
  import RailLayout from '$lib/shell/RailLayout.svelte';

  type Rule = { id: string; title: string; body: string[] };
  const RULES: Rule[] = [
    {
      id: 'yours',
      title: 'Upload what you made or may use',
      body: [
        'Draw it yourself, have it made for you, or use art whose owner lets you. Don’t upload other people’s emotes, art or channel graphics as your own.',
        'Logos, characters and screenshots from games, shows and brands belong to someone. A small edit of a well-known picture is usually fine; a straight copy of an artist’s work is not.',
        'If something you own was uploaded without asking, see Takedowns in the footer.',
      ],
    },
    {
      id: 'hate',
      title: 'No hate or harassment',
      body: [
        'Nothing that attacks people for who they are: race, ethnicity, religion, gender, sexual orientation, disability, age or nationality. That includes slurs, hate symbols and “jokes” built on them, even spelled cleverly or hidden in a frame.',
        'No emotes made to mock, threaten or pile on a real person, a streamer or a chatter.',
      ],
    },
    {
      id: 'nsfw',
      title: 'Sexual or graphic content only with NSFW, and never explicit',
      body: [
        'Suggestive emotes are fine when marked NSFW: they stay out of browsing unless people turn NSFW on.',
        'Nudity, sex acts and gore are never allowed, flagged or not. Nothing sexual involving minors or anyone who looks like one, ever; we report it.',
        'Twitch’s and Kick’s own rules still apply in their chats, whatever the flag says.',
      ],
    },
    {
      id: 'private-info',
      title: 'No personal information',
      body: [
        'No names, addresses, phone numbers, faces of private people, screenshots of private messages or anything else that identifies someone who didn’t agree to it.',
      ],
    },
    {
      id: 'impersonation',
      title: 'Don’t pretend to be someone else',
      body: [
        'No emotes that pass as cyex.tv, Twitch, Kick, a moderator or another streamer’s official emotes or badges. Fan art of a streamer is fine when it’s clearly fan art.',
      ],
    },
    {
      id: 'harm',
      title: 'Nothing illegal or dangerous',
      body: [
        'No self-harm, drugs being taken, weapons aimed at the viewer, scams, ads or links hidden in the picture.',
        'No flashing that could trigger seizures: fast full-screen flashes between bright and dark colours, more than three a second.',
      ],
    },
    {
      id: 'readable',
      title: 'Make it work in chat',
      body: [
        'Chat shows emotes 28 px tall. Check the preview: if it turns into a blur at 1x, crop it closer or simplify it.',
        'Square works best; wide is fine. Very tall pictures end up tiny. Trim empty borders and keep the background transparent where you can.',
        'Zero-width emotes are drawn over the emote before them, so they need a transparent background and should fit over a square emote.',
      ],
    },
    {
      id: 'names',
      title: 'Names and tags that say what it is',
      body: [
        'The name is what people type in chat; keep it short and free of slurs. Tags help people find it: describe what’s in the picture, not what’s popular. No tag spam.',
      ],
    },
  ];

  let root: HTMLDivElement;
  onMount(() => initDocs(root, { filter: false }));
</script>

<svelte:head>
  <title>Emote guidelines - cyex.tv</title>
  <meta name="description" content="What you may upload to cyex.tv: your own work, nothing hateful, sexual content only behind NSFW, no personal information, and emotes that read at chat size." />
  <meta property="og:title" content="Emote guidelines" />
</svelte:head>

<div class="docs guidelines" bind:this={root}>
  <RailLayout>
    {#snippet rail()}
    <aside class="docs-nav" aria-label="Guidelines index">
      <details class="nav-toc">
        <summary>Contents</summary>
        <nav class="nav-scroll" aria-label="Contents">
          <div class="nav-group">
            <div class="nav-group-title"><span class="group-name">The rules</span></div>
            {#each RULES as r, i (r.id)}
              <a class="nav-item" href="#{r.id}"><span class="n" aria-hidden="true">{i + 1}</span><span class="nav-label">{r.title}</span></a>
            {/each}
          </div>
          <div class="nav-group">
            <div class="nav-group-title"><span class="group-name">After</span></div>
            <a class="nav-item" href="#after"><span class="nav-label">When something breaks these</span></a>
          </div>
        </nav>
      </details>
    </aside>
    {/snippet}

    <div class="docs-body" id="rules" tabindex="-1">
      <header class="docs-title">
        <div>
          <h1>Emote guidelines</h1>
          <p>What you can upload to cyex.tv. Every new emote is checked by a moderator before anyone else sees it; these are the things they look for.</p>
        </div>
      </header>

      {#each RULES as r, i (r.id)}
        <section class="guide rule" id={r.id} aria-labelledby="{r.id}-title">
          <p class="num" aria-hidden="true">Rule {i + 1}</p>
          <h2 id="{r.id}-title">{r.title}</h2>
          {#each r.body as p, j (j)}<p>{p}</p>{/each}
        </section>
      {/each}

      <section class="guide rule after" id="after" aria-labelledby="after-title">
        <h2 id="after-title">When something breaks these</h2>
        <p>A moderator rejects the upload or takes the emote down, and you get a notification saying why. Emotes already in channels stop showing there.</p>
        <p>Uploading again what was taken down, or breaking the rules again and again, can stop your account from uploading. You can report an emote from its page.</p>
        <p>These guidelines may change as cyex.tv grows; the version here is the one that counts.</p>
      </section>

      <p class="docs-end">Questions about a rule? See <a href="/help">Help</a>. Something of yours uploaded without asking? Write to abuse@cyex.tv.</p>
    </div>
  </RailLayout>
</div>

<style>
  .guidelines :global(.nav-group-title) { display: flex; align-items: center; min-height: 32px; margin-bottom: 2px; }
  .guidelines :global(.nav-item) { padding-block: 8px; align-items: flex-start; }
  .guidelines :global(.nav-label) { white-space: normal; }
  .n { flex: none; min-width: 14px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .rule { max-width: var(--measure); margin-bottom: 36px; }
  .rule + .rule { padding-top: 28px; border-top: 1px solid var(--border); }
  .num { margin: 0 0 4px; font-size: var(--t-fine); font-weight: 600; color: var(--dim); font-variant-numeric: tabular-nums; }
  .rule h2 { margin: 0 0 10px; }
  .rule p:not(.num) { margin: 0 0 8px; line-height: 1.65; color: var(--muted); }
  .rule p:last-child { margin-bottom: 0; }
</style>
