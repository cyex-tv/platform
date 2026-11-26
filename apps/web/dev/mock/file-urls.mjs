// Mock API: file entries carry `url` and `mime` like the real API's (`{host}/{name}`, and the
// format's media type), added to every object that has a `host` and a `files` list on its way out.
const MIME = { WEBP: 'image/webp', AVIF: 'image/avif', webp: 'image/webp', avif: 'image/avif' };

export function withFileUrls(value) {
  if (Array.isArray(value)) return value.map(withFileUrls);
  if (!value || typeof value !== 'object') return value;
  const out = {};
  for (const [k, v] of Object.entries(value)) out[k] = withFileUrls(v);
  if (typeof out.host === 'string' && Array.isArray(out.files)) {
    out.files = out.files.map((f) =>
      f && typeof f.name === 'string' && f.format ? { ...f, url: `${out.host}/${f.name}`, mime: MIME[f.format] ?? 'application/octet-stream' } : f,
    );
  }
  return out;
}
