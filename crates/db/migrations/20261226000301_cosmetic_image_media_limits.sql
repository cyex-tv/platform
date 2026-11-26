-- The media limits a cosmetic picture was uploaded under (`{"max_bytes", "max_side", "max_frames"}`,
-- from the uploader's roles, `setRoleLimits`). Making the picture again (`remakeCosmeticImage`) and
-- the worker use these, not the limits of whoever asks later, so a role changing meanwhile can't
-- make a good picture fail. Null: the limits the API and worker are configured with.
-- Role limits themselves need no change: `roles.limits` already holds any key the API accepts.
ALTER TABLE cosmetic_images ADD COLUMN media_limits jsonb;
ALTER TABLE cosmetic_images ADD CONSTRAINT cosmetic_images_media_limits_object
    CHECK (media_limits IS NULL OR jsonb_typeof(media_limits) = 'object');
