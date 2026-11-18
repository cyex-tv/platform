// Your data as a download. The API returns the whole document as JSON text; the browser saves it
// as a file named for the day.

/** `cyex-tv-export-2026-11-17.json`, the day in the reader's time zone. */
export function exportFilename(now: Date = new Date()): string {
  const p = (v: number) => v.toString().padStart(2, '0');
  return `cyex-tv-export-${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}.json`;
}

/** Saves `json` as a file through a temporary link. */
export function saveJson(json: string, filename: string): void {
  const url = URL.createObjectURL(new Blob([json], { type: 'application/json' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.rel = 'noopener';
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}
