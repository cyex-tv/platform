// The header search's pure parts: keys, row movement and what the status line reads out.
import { count } from '$lib/format';

/** Whether a key press was typed into a field (so "/" there types a slash). */
export function typing(target: EventTarget | null): boolean {
  if (!target || typeof Element === 'undefined' || !(target instanceof Element)) return false;
  const el = target as HTMLElement;
  return el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName);
}

/** Moves the active row by `step`, wrapping at both ends; -1 means none is active. */
export function move(active: number, step: number, length: number): number {
  if (length === 0) return -1;
  if (active < 0) return step > 0 ? 0 : length - 1;
  return (active + step + length) % length;
}

/** What the status line reads once results settle: "5 emotes, 2 people.", or that nothing matched. */
export function announce(emotes: number, people: number, text: string): string {
  const parts = [
    emotes ? `${count(emotes)} ${emotes === 1 ? 'emote' : 'emotes'}` : '',
    people ? `${count(people)} ${people === 1 ? 'person' : 'people'}` : '',
  ].filter(Boolean);
  if (parts.length) return `${parts.join(', ')}.`;
  return text ? emptyText(text) : '';
}

/** The line shown (and read) when nothing matches. */
export function emptyText(text: string): string {
  return `No emotes or people match “${text}”.`;
}

/** Where Enter goes with no row picked: the full search for the text. */
export function searchHref(text: string): string {
  return `/search?q=${encodeURIComponent(text.trim())}`;
}
