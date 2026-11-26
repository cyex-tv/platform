-- Emote upload rules are fixed for everyone (ADR 0031, amended), so a role no longer stores emote
-- limits. Removes the three keys #193 added from every role that holds them; the API already reads
-- unknown keys as unset, so this is housekeeping. Runs safely again (a role without them is
-- not touched).
UPDATE roles
SET limits = limits - 'emote_max_bytes' - 'emote_max_side' - 'emote_max_frames'
WHERE limits ?| ARRAY['emote_max_bytes', 'emote_max_side', 'emote_max_frames'];
