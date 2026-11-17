<script lang="ts">
  // One sample block. With `tabs` it is a language-switched request (the choice is shared by every
  // block on the page, see behaviour.ts); without, a single labelled block such as a response.
  type Tab = { lang: string; label: string; code: string };
  let {
    tabs,
    label = '',
    meta = '',
    code = '',
    ok = false,
    copy = 'Copy',
  }: { tabs?: Tab[]; label?: string; meta?: string; code?: string; ok?: boolean; copy?: string } = $props();
</script>

<div class="code-block">
  <div class="code-head">
    {#if tabs && tabs.length}
      <div class="lang-tabs" role="tablist" aria-label="Language">
        {#each tabs as t, i}
          <button type="button" role="tab" class="lang-tab" data-lang={t.lang} aria-selected={i === 0}>{t.label}</button>
        {/each}
      </div>
    {:else}
      <div class="resp-head">
        {#if ok}<span class="status-pip ok" aria-hidden="true"></span>{/if}
        <span class="resp-status">{label}</span>{#if meta}<span class="resp-type">{meta}</span>{/if}
      </div>
    {/if}
    <button type="button" class="code-copy" aria-label={copy} title={copy}>
      <svg class="icon-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="1.5" /><path d="M5 15V5.5A1.5 1.5 0 0 1 6.5 4H15" /></svg>
      <svg class="icon-done" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
    </button>
  </div>
  {#if tabs && tabs.length}
    {#each tabs as t, i}
      <pre data-lang={t.lang} hidden={i > 0}><code>{t.code}</code></pre>
    {/each}
  {:else}
    <pre><code>{code}</code></pre>
  {/if}
</div>
