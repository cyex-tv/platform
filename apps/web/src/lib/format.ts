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

/** An RFC 3339 time as `2026/11/17`, in the reader's time zone. */
export function date(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const p = (v: number) => v.toString().padStart(2, '0');
  return `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())}`;
}

/**
 * A name or text from someone else, safe to show: the bidi embedding, override and isolate
 * controls (U+202A-202E, U+2066-2069) are removed, so a name can't flip the text around it.
 */
export function clean(s: string | null | undefined): string {
  return (s ?? '').replace(/[\u202A-\u202E\u2066-\u2069]/g, '');
}
