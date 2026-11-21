// Numbers and dates the cyex way (cyex-design): 1.000.000, 1,5 MB, 1,2K, 2026/10/23, 24-hour time.

/** `1234567` → `1.234.567`. */
export function count(n: number): string {
  const sign = n < 0 ? '-' : '';
  return sign + Math.trunc(Math.abs(n)).toString().replace(/\B(?=(\d{3})+(?!\d))/g, '.');
}

/** `1234` → `1,2K`, `2500000` → `2,5M`; below 1.000 as is. */
export function compact(n: number): string {
  const abs = Math.abs(n);
  const one = (v: number, unit: string) => {
    const s = (Math.round(v * 10) / 10).toString().replace('.', ',');
    return `${n < 0 ? '-' : ''}${s.replace(/,0$/, '')}${unit}`;
  };
  if (abs >= 1_000_000) return one(abs / 1_000_000, 'M');
  if (abs >= 1_000) return one(abs / 1_000, 'K');
  return count(n);
}

/** Bytes as `900 B`, `1,5 KB`, `2,3 MB`. */
export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const kb = n / 1024;
  if (kb < 1024) return `${(Math.round(kb * 10) / 10).toString().replace('.', ',')} KB`;
  return `${(Math.round((kb / 1024) * 10) / 10).toString().replace('.', ',')} MB`;
}

/** Two digits, zero-padded: `7` → `07`. */
export const pad = (v: number): string => String(v).padStart(2, '0');

/** An RFC 3339 time as `2026/11/17`, in the reader's time zone. */
export function date(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())}`;
}

/** An RFC 3339 time as `22:40` (the time of day alone), in the reader's time zone, 24-hour. */
export function clock(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** An RFC 3339 time as `2026/11/17 22:40`, in the reader's time zone, 24-hour. */
export function dateTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return `${date(iso)} ${clock(iso)}`;
}

/** An RFC 3339 time as `2026/11/16 20:01:02`, in the reader's time zone, 24-hour; '' if invalid. */
export function dateTimeSeconds(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return `${dateTime(iso)}:${pad(d.getSeconds())}`;
}

/** A `datetime-local` value (`2026-12-01T18:30`) from an RFC 3339 time or a Date, or ''. */
export function toLocalInput(v: string | Date | null | undefined): string {
  if (!v) return '';
  const d = v instanceof Date ? v : new Date(v);
  if (Number.isNaN(d.getTime())) return '';
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** An RFC 3339 time from a `datetime-local` value (read in the reader's time zone), or null. */
export function fromLocalInput(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/**
 * A name or text from someone else, safe to show: the bidi embedding, override and isolate
 * controls (U+202A-202E, U+2066-2069) are removed, so a name can't flip the text around it.
 */
export function clean(s: string | null | undefined): string {
  return (s ?? '').replace(/[\u202A-\u202E\u2066-\u2069]/g, '');
}

/**
 * `clean(s)` wrapped in FIRST STRONG ISOLATE ... POP DIRECTIONAL ISOLATE (U+2068, U+2069), for a
 * name inside a plain string (toasts, titles, labels): a right-to-left name, or an old row that
 * still holds controls, can't reorder the words around it. `clean` removes any isolate controls
 * the name brings, so it can't close the isolate early. In markup use `<Name>` instead.
 */
export function iso(s: string | null | undefined): string {
  return `⁨${clean(s)}⁩`;
}

/**
 * A sentence built with `iso`, split into plain pieces and isolated names, so a page can put the
 * names in `<bdi>` and keep the isolate marks out of what a screen reader reads or a link is called.
 */
export function isoParts(text: string): { text: string; name: boolean }[] {
  const out: { text: string; name: boolean }[] = [];
  for (const [i, piece] of text.split(/⁨([^⁩]*)⁩/).entries()) {
    if (piece) out.push({ text: piece, name: i % 2 === 1 });
  }
  return out;
}
