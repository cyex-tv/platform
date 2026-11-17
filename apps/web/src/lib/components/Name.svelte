<script lang="ts">
  // A name someone chose (an emote, alias, set or role name, a display name or login, a report
  // reason, a search echo), shown in its own bidi context: <bdi> isolates it and picks its
  // direction from its first strong character, so a right-to-left name or a stray control can't
  // reorder the text around it. `clean` also drops the controls themselves.
  //   by <Name value={e.owner.displayName} />
  import { clean } from '$lib/format';

  type Props = { value: string | null | undefined; fallback?: string; class?: string; title?: string };
  let { value, fallback = '', class: cls, title }: Props = $props();
  const text = $derived(clean(value) || fallback);
</script>

<bdi class={cls} {title}>{text}</bdi>
