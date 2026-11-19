-- Which linked platform an account goes by (owner feedback 2026/11/19, D): its name and picture
-- everywhere on cyex.tv come from that identity. NULL is the old rule, Twitch first. Search keeps
-- matching every linked login. Unlinking that platform sets it back to NULL.
ALTER TABLE accounts
    ADD COLUMN display_platform text CHECK (display_platform IN ('twitch', 'kick'));
