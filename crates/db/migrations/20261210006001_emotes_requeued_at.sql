-- Review queue undo (additive): an approval a reviewer takes back within its undo time sends the
-- emote to the end of the queue, for every reviewer, not back to its old place. The queue's
-- upload-time order reads `coalesce(requeued_at, created_at)`; a verdict clears it.
ALTER TABLE emotes ADD COLUMN requeued_at timestamptz;
