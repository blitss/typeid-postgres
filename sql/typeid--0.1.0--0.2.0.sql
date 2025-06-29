/*─────────────────────────────────────────────────────────────────────────────
  typeid 0.1.0  →  0.2.0  upgrade
  * preserves camel-case TypeID identifier *
─────────────────────────────────────────────────────────────────────────────*/
BEGIN;

-- 1 ── binary protocol -------------------------------------------------------
CREATE FUNCTION typeid_recv(internal)
RETURNS TypeID
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_recv_wrapper';

CREATE FUNCTION typeid_send(TypeID)
RETURNS bytea
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_send_wrapper';

ALTER TYPE TypeID
    SET (RECEIVE = typeid_recv, SEND = typeid_send);

-- 2 ── helper: prefix as text ------------------------------------------------
CREATE FUNCTION typeid_prefix("typeid" TypeID)
RETURNS text
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_prefix_wrapper';

-- 3 ── implicit cast  text → TypeID -----------------------------------------
CREATE CAST (text AS TypeID)
    WITH INOUT
    AS IMPLICIT;

-- 4 ── parallel-safe aggregates ---------------------------------------------
CREATE FUNCTION type_id_min_type_id_min_combine(this TypeID, v TypeID)
RETURNS TypeID
LANGUAGE c
AS 'MODULE_PATHNAME', 'type_id_min_type_id_min_combine_wrapper';

CREATE FUNCTION type_id_max_type_id_max_combine(this TypeID, v TypeID)
RETURNS TypeID
LANGUAGE c
AS 'MODULE_PATHNAME', 'type_id_max_type_id_max_combine_wrapper';

DROP AGGREGATE IF EXISTS min(TypeID);
CREATE AGGREGATE min (TypeID)
(
    SFUNC       = type_id_min_type_id_min_state,
    STYPE       = TypeID,
    COMBINEFUNC = type_id_min_type_id_min_combine,
    PARALLEL    = SAFE
);

DROP AGGREGATE IF EXISTS max(TypeID);
CREATE AGGREGATE max (TypeID)
(
    SFUNC       = type_id_max_type_id_max_state,
    STYPE       = TypeID,
    COMBINEFUNC = type_id_max_type_id_max_combine,
    PARALLEL    = SAFE
);

-- 5 ── mark helpers IMMUTABLE & PARALLEL SAFE --------------------------------
ALTER FUNCTION typeid_cmp(TypeID,TypeID)              IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_lt(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_le(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_eq(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_ge(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_gt(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_ne(TypeID,TypeID)               IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_hash(TypeID)                    IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_hash_extended(TypeID,bigint)    IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION uuid_to_typeid(text,uuid)              IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_to_uuid(TypeID)                 IMMUTABLE PARALLEL SAFE;

-- 6 ── NEW v0.2.0 utility functions ------------------------------------------

-- Generate TypeID with empty prefix (UUID-only format)
CREATE FUNCTION typeid_generate_nil()
RETURNS TypeID
STRICT VOLATILE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_generate_nil_wrapper';

-- Validate TypeID format without parsing
CREATE FUNCTION typeid_is_valid("input" TEXT)
RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_is_valid_wrapper';

-- Check if TypeID has a specific prefix
CREATE FUNCTION typeid_has_prefix("typeid" TypeID, "prefix" TEXT)
RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_has_prefix_wrapper';

-- Check if TypeID has empty prefix
CREATE FUNCTION typeid_is_nil_prefix("typeid" TypeID)
RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_is_nil_prefix_wrapper';



-- Generate multiple TypeIDs efficiently
CREATE FUNCTION typeid_generate_batch("prefix" TEXT, "count" INT)
RETURNS TypeID[]
STRICT VOLATILE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'typeid_generate_batch_wrapper';

-- 7 ── NEW v0.2.0 operators and documentation --------------------------------

-- Create prefix matching operator (follows PostgreSQL "contains" semantics)
CREATE OPERATOR @> (
    LEFTARG = typeid,
    RIGHTARG = text,
    PROCEDURE = typeid_has_prefix,
    COMMUTATOR = '@<'
);

-- Add function documentation
COMMENT ON FUNCTION typeid_prefix(typeid) IS 'Extract the prefix from a TypeID for indexing and filtering';
COMMENT ON FUNCTION typeid_has_prefix(typeid, text) IS 'Check if TypeID has a specific prefix - useful for filtering';
COMMENT ON FUNCTION typeid_is_valid(text) IS 'Validate TypeID format without parsing - useful for constraints';
COMMENT ON FUNCTION typeid_generate_nil() IS 'Generate TypeID with empty prefix (UUID-only format)';
COMMENT ON FUNCTION typeid_generate_batch(text, int) IS 'Generate multiple TypeIDs with the same prefix efficiently';

COMMIT;
