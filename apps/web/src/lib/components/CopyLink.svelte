<script lang="ts">
  // The one "Copy link" button (profile, channel, set): the same words everywhere ("Copy link",
  // then "Copied" or "Couldn’t copy"), the width of the longest so it never jumps, and a status
  // line for screen readers. `kind` picks the button family of the surface it sits on.
  import { useCopied } from '$lib/clipboard.svelte';
  import Icon from './Icon.svelte';

  type Props = {
    /** The address to copy, read at the moment of the click. */
    url: () => string;
    /** `site`: the public pages' .btn.btn-s; `tool`: the dense pages' .ui-btn.secondary. */
    kind?: 'site' | 'tool';
  };
  let { url, kind = 'site' }: Props = $props();
  const link = useCopied();
</script>

<button class={kind === 'site' ? 'btn btn-s copy' : 'ui-btn secondary copy'} type="button" onclick={() => link.copy(url())}>
  <Icon name={link.copied ? 'check' : 'copy'} size={16} />
  <span class="labels">
    <span class:off={link.state !== 'idle'}>Copy link</span>
    <span class:off={!link.copied} aria-hidden="true">Copied</span>
    <span class:off={!link.failed} aria-hidden="true">Couldn’t copy</span>
  </span>
</button>
<span class="sr-only" role="status">{link.copied ? 'Link copied' : link.failed ? 'Couldn’t copy' : ''}</span>

<style>
  .labels { display: inline-grid; }
  .labels > span { grid-area: 1 / 1; text-align: left; }
  .off { visibility: hidden; }
</style>
