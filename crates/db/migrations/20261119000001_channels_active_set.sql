-- `channelCount` and `emoteChannels` join channels to set members on the active set.
CREATE INDEX channels_active_set ON channels (active_set_id) WHERE active_set_id IS NOT NULL;
