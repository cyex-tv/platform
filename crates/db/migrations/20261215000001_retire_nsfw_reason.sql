-- NSFW emotes aren't allowed (ADR 0020, amendment 2026/11/22), so the seeded reject reason that
-- asked for the flag says what is wrong instead. Only the untouched seed row changes: a reason
-- staff wrote themselves stays. The `emotes.nsfw` column stays (additive rule); the API no longer
-- sets it, and an emote still flagged is kept off the listings until staff save its flags.
UPDATE review_reasons
SET body = 'NSFW or sexual content (not allowed)', updated_at = now()
WHERE body = 'NSFW but not marked' AND updated_by IS NULL;
