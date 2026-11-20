<script lang="ts">
  // When something happened, the cyex way everywhere an activity or a notification is listed: the
  // date and time (2026/11/19 14:05, 24-hour), with how long ago on hover. The look (size, colour)
  // comes from the line it sits in.
  import { dateTime } from '$lib/format';
  import { relative } from './activity';

  let { at }: { at: string } = $props();
</script>

<time datetime={at} title={relative(at)}>{dateTime(at)}</time>

<style>
  time { font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
