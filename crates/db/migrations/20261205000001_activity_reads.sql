-- Public activity reads: the api asks SQL to drop feed rows about sets that aren't listed, so
-- hidden rows don't use up its read rounds. Audit rows name their target as the ID's 26-character
-- text form (a ULID, ADR 0002); this reads that back as the uuid the tables use. NULL for text
-- that isn't one.
CREATE FUNCTION ulid_to_uuid(t text) RETURNS uuid
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    alphabet constant text := '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
    bits varbit := ''::varbit;
    rest varbit;
    v integer;
    hex text := '';
    i integer;
BEGIN
    IF length(t) <> 26 THEN
        RETURN NULL;
    END IF;
    FOR i IN 1..26 LOOP
        v := position(upper(substr(t, i, 1)) IN alphabet) - 1;
        IF v < 0 OR (i = 1 AND v > 7) THEN
            RETURN NULL;
        END IF;
        bits := bits || v::bit(5)::varbit;
    END LOOP;
    -- 26 * 5 = 130 bits; the first two are always zero.
    rest := substring(bits FROM 3);
    FOR i IN 0..31 LOOP
        hex := hex || to_hex(substring(rest FROM i * 4 + 1 FOR 4)::bit(4)::integer);
    END LOOP;
    RETURN hex::uuid;
END;
$$;
