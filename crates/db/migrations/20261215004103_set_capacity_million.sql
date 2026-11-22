-- Set capacity goes up to 1.000.000 (staff), from 100.000 (ADR 0017, amendment 2026/11/22). Users
-- set 1 to their `set_capacity_max` role limit (1.000 by default). Widening a check: every row
-- that passed the old one passes the new one.
ALTER TABLE emote_sets DROP CONSTRAINT IF EXISTS emote_sets_capacity_check;
ALTER TABLE emote_sets ADD CONSTRAINT emote_sets_capacity_check CHECK (capacity BETWEEN 1 AND 1000000);