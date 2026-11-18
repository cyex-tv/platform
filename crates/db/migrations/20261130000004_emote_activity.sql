-- Emote activity (`emoteActivity`, additive): the public activity rows about one emote are its
-- approval (found by target) and the set changes naming it in their JSON (found here).
CREATE INDEX audit_log_emote_activity
    ON audit_log ((coalesce(after_json ->> 'emote_id', before_json ->> 'emote_id')), id DESC)
    WHERE subject_account_id IS NOT NULL;
