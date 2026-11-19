<script lang="ts">
  // /guidelines: what may be uploaded to cyex.tv, in our own words. The upload form links here
  // (in a new tab) and asks you to accept it. Each rule has an anchor, so a moderator's note or a
  // help answer can point at one (/guidelines#nsfw). A short reading page: one column, no rail.
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

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
</script>

<svelte:head>
  <title>Emote guidelines - cyex.tv</title>
  <meta name="description" content="What you may upload to cyex.tv: your own work, nothing hateful, sexual content only behind NSFW, no personal information, and emotes that read at chat size." />
  <meta property="og:title" content="Emote guidelines" />
</svelte:head>

<ColumnLayout>
  <header class="page-head">
    <h1 class="page-title">Emote guidelines</h1>
    <p class="lead">What you can upload to cyex.tv. Every new emote is checked by a moderator before anyone else sees it; these are the things they look for.</p>
  </header>

  <nav class="toc" aria-label="On this page">
    <ol>
      {#each RULES as r, i (r.id)}<li><a href="#{r.id}"><span class="n ui-num">{i + 1}</span>{r.title}</a></li>{/each}
    </ol>
  </nav>

  <div class="rules">
    {#each RULES as r, i (r.id)}
      <section class="rule" id={r.id} aria-labelledby="{r.id}-title">
        <h2 id="{r.id}-title"><span class="n ui-num" aria-hidden="true">{i + 1}</span>{r.title}</h2>
        {#each r.body as p, j (j)}<p>{p}</p>{/each}
      </section>
    {/each}

    <section class="rule after" aria-labelledby="after-title">
      <h2 id="after-title">When something breaks these</h2>
      <p>A moderator rejects the upload or takes the emote down, and you get a notification saying why. Emotes already in channels stop showing there.</p>
      <p>Uploading again what was taken down, or breaking the rules again and again, can stop your account from uploading. You can report an emote from its page.</p>
      <p>These guidelines may change as cyex.tv grows; the version here is the one that counts.</p>
    </section>
  </div>
</ColumnLayout>

<style>
  .lead { margin: 12px 0 0; max-width: 62ch; font-size: var(--t-body, 16px); line-height: 1.6; color: var(--muted); }
  .toc { max-width: 720px; margin-bottom: 32px; padding: 12px 16px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .toc ol { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 24px; margin: 0; padding: 0; list-style: none; }
  .toc a { display: flex; align-items: center; gap: 10px; min-height: 36px; color: var(--muted); font-size: var(--t-small); transition: color var(--fast); }
  .toc a:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) { .toc a:hover { color: var(--text); } }
  .n { flex: none; display: inline-grid; place-items: center; min-width: 22px; height: 22px; border-radius: var(--radius); border: 1px solid var(--border-strong);
    font-size: 12px; font-weight: 600; color: var(--muted); }
  .rules { display: grid; gap: 28px; max-width: 720px; }
  .rule { scroll-margin-top: calc(var(--nav-h) + 20px); }
  .rule h2 { display: flex; align-items: center; gap: 10px; margin: 0 0 8px; font-size: 18px; line-height: 1.35; letter-spacing: -.01em; }
  .rule p { margin: 0 0 8px; max-width: 68ch; line-height: 1.65; color: var(--muted); }
  .rule p:last-child { margin-bottom: 0; }
  .after { padding-top: 24px; border-top: 1px solid var(--border); }
  @media (max-width: 640px) {
    .toc ol { grid-template-columns: minmax(0, 1fr); }
    .rule h2 { font-size: 17px; align-items: flex-start; }
    .rule h2 .n { margin-top: 1px; }
  }
  @media (pointer: coarse) { .toc a { min-height: 44px; } }
</style>
