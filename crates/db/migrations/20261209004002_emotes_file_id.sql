-- no-transaction
-- Duplicate checks (additive): the emotes made from one stored file, for the review queue's
-- "possible duplicate" strip and the uploader's "looks like" warning. CONCURRENTLY, so a live
-- table isn't locked for writes while it builds; one statement per file.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emotes_file_id ON emotes (file_id) WHERE deleted_at IS NULL;
