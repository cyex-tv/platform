// Copying text, with a short "copied" state for the button that did it.
//   const copied = useCopied();
//   <button onclick={() => copied.copy(url)}>…</button>  {copied.state}: 'idle' | 'copied' | 'failed'

/** Copies `text`; true when it reached the clipboard. Falls back to a hidden textarea. */
export async function copy(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Denied or not focused: try the old way below.
  }
  if (typeof document === 'undefined') return false;
  try {
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.setAttribute('readonly', '');
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.append(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    return ok;
  } catch {
    return false;
  }
}

export type CopyState = 'idle' | 'copied' | 'failed';

/** A copy action whose `state` reads 'copied' or 'failed' for `ms` after each copy, then 'idle'. */
export function useCopied(ms = 1600) {
  let state = $state<CopyState>('idle');
  let timer: ReturnType<typeof setTimeout> | undefined;
  return {
    get state(): CopyState {
      return state;
    },
    get copied(): boolean {
      return state === 'copied';
    },
    get failed(): boolean {
      return state === 'failed';
    },
    async copy(text: string): Promise<boolean> {
      const ok = await copy(text);
      state = ok ? 'copied' : 'failed';
      clearTimeout(timer);
      timer = setTimeout(() => (state = 'idle'), ms);
      return ok;
    },
    reset() {
      clearTimeout(timer);
      state = 'idle';
    },
  };
}
