-- Header quick search (`quickSearch`, additive): word and initials matches on emote names.
-- `emote_name_words` splits a name at camelCase, acronym, letter/digit and `_ - . space`
-- boundaries and lowercases it (`PepePoint` -> `pepe point`, `KEKWait` -> `kek wait`).
CREATE FUNCTION emote_name_words(name text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT lower(btrim(regexp_replace(
        regexp_replace(
            regexp_replace(
                regexp_replace(
                    regexp_replace(name, '([A-Z]+)([A-Z][a-z])', '\1 \2', 'g'),
                    '([a-z])([A-Z])', '\1 \2', 'g'),
                '([A-Za-z])([0-9])', '\1 \2', 'g'),
            '([0-9])([A-Za-z])', '\1 \2', 'g'),
        '[ _.-]+', ' ', 'g')))
$$;

-- The first letter of each word: `pepe point` -> `pp`.
CREATE FUNCTION emote_name_initials(name text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT regexp_replace(emote_name_words(name), '([^ ])[^ ]*( |$)', '\1', 'g')
$$;

ALTER TABLE emotes
    ADD COLUMN name_words text GENERATED ALWAYS AS (emote_name_words(name)) STORED,
    ADD COLUMN name_initials text GENERATED ALWAYS AS (emote_name_initials(name)) STORED;

-- `' ' || name_words LIKE '% ' || q || '%'`: a word of the name starts with q.
CREATE INDEX emotes_name_words_trgm ON emotes USING gin ((' ' || name_words) gin_trgm_ops);
CREATE INDEX emotes_name_initials ON emotes (name_initials text_pattern_ops);
