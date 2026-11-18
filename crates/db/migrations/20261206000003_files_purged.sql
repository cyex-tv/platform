-- Deleted accounts take their uploaded files with them (sign-off code review). Additive: one
-- nullable column.
--
-- Set when a `files.delete` job is queued for a file no live emote uses. The worker deletes the
-- objects only while it is still set; an upload of the same bytes clears it, stores the original
-- again and queues a new transcode.
ALTER TABLE files ADD COLUMN purged_at timestamptz;
