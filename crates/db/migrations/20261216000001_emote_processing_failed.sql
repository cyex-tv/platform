-- An upload whose transcode job ran out of attempts shows as failed instead of processing
-- forever. Additive: one nullable column.
--
-- Set by the worker when `emote.transcode` fails for good while the file still has no variants.
-- An upload of the same bytes clears it and queues a new transcode.
ALTER TABLE files ADD COLUMN processing_failed_at timestamptz;
