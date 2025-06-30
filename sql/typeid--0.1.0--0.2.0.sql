/*
  typeid v0.1.0 -> v0.2.0
  This script handles the following changes:
  1. Drops the old `min`/`max` aggregates which are incompatible with the new library.
  2. Drops the old state functions for those aggregates.
  3. Creates new state and combine functions for parallel-safe `min`/`max` aggregates.
  4. Creates the new parallel-safe `min`/`max` aggregates.
  5. Adds binary `SEND`/`RECEIVE` functions to the `TypeID` type for replication.
  6. Adds all new v0.2.0 utility functions (`typeid_prefix`, `typeid_is_valid`, etc.).
  7. Adds the new `@>` prefix-matching operator.
  8. Adds an implicit `CAST` from `text` to `typeid`.
  9. Marks all existing functions as `IMMUTABLE` and `PARALLEL SAFE`.
  10. Adds comments to the new functions and operators.
*/

-- Step 1: Drop old aggregates and their functions from v0.1.0
DROP AGGREGATE IF EXISTS min(TypeID);
DROP AGGREGATE IF EXISTS max(TypeID);
DROP FUNCTION IF EXISTS type_id_min_state(TypeID, TypeID);
DROP FUNCTION IF EXISTS type_id_max_state(TypeID, TypeID);

-- Step 2: Create new functions for v0.2.0

-- Binary protocol functions
CREATE FUNCTION typeid_recv(internal)
RETURNS TypeID IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_recv_wrapper';

CREATE FUNCTION typeid_send(TypeID)
RETURNS bytea IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_send_wrapper';

-- New utility functions
CREATE FUNCTION typeid_prefix(typeid TypeID)
RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_prefix_wrapper';

CREATE FUNCTION typeid_generate_nil()
RETURNS TypeID STRICT VOLATILE PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_generate_nil_wrapper';

CREATE FUNCTION typeid_is_valid(input TEXT)
RETURNS bool IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_is_valid_wrapper';

CREATE FUNCTION typeid_has_prefix(typeid TypeID, prefix TEXT)
RETURNS bool IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_has_prefix_wrapper';

CREATE FUNCTION typeid_is_nil_prefix(typeid TypeID)
RETURNS bool IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_is_nil_prefix_wrapper';

CREATE FUNCTION typeid_generate_batch(prefix TEXT, count INT)
RETURNS TypeID[] STRICT VOLATILE PARALLEL SAFE
LANGUAGE c AS 'MODULE_PATHNAME', 'typeid_generate_batch_wrapper';

-- New aggregate helper functions
CREATE FUNCTION "type_id_max_type_id_max_combine"(
    "this" TypeID,
    "v" TypeID
) RETURNS TypeID
LANGUAGE c AS 'MODULE_PATHNAME', 'type_id_max_type_id_max_combine_wrapper';

CREATE FUNCTION "type_id_max_type_id_max_state"(
    "this" TypeID,
    "arg_one" TypeID
) RETURNS TypeID
LANGUAGE c AS 'MODULE_PATHNAME', 'type_id_max_type_id_max_state_wrapper';

CREATE FUNCTION "type_id_min_type_id_min_combine"(
    "this" TypeID,
    "v" TypeID
) RETURNS TypeID
LANGUAGE c AS 'MODULE_PATHNAME', 'type_id_min_type_id_min_combine_wrapper';

CREATE FUNCTION "type_id_min_type_id_min_state"(
    "this" TypeID,
    "arg_one" TypeID
) RETURNS TypeID
LANGUAGE c AS 'MODULE_PATHNAME', 'type_id_min_type_id_min_state_wrapper';


-- Step 3: Update the TypeID type definition
ALTER TYPE TypeID SET (RECEIVE = typeid_recv, SEND = typeid_send);

-- Step 4: Re-create aggregates with new parallel-safe functions
CREATE AGGREGATE max (TypeID) (
    SFUNC = "type_id_max_type_id_max_state",
    STYPE = TypeID,
    COMBINEFUNC = "type_id_max_type_id_max_combine",
    PARALLEL = SAFE
);

CREATE AGGREGATE min (TypeID) (
    SFUNC = "type_id_min_type_id_min_state",
    STYPE = TypeID,
    COMBINEFUNC = "type_id_min_type_id_min_combine",
    PARALLEL = SAFE
);

-- Step 5: Update existing functions to be parallel safe
ALTER FUNCTION typeid_in(cstring) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_out(TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_generate(TEXT) VOLATILE PARALLEL SAFE;
ALTER FUNCTION uuid_to_typeid(TEXT, uuid) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_to_uuid(TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_cmp(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_lt(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_le(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_eq(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_ge(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_gt(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_ne(TypeID, TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_hash(TypeID) IMMUTABLE PARALLEL SAFE;
ALTER FUNCTION typeid_hash_extended(TypeID, bigint) IMMUTABLE PARALLEL SAFE;


-- Step 6: Create new operators and casts
CREATE CAST (text AS TypeID)
WITH INOUT AS IMPLICIT;

CREATE OPERATOR @> (
    LEFTARG = typeid,
    RIGHTARG = text,
    PROCEDURE = typeid_has_prefix
);

-- Step 7: Add comments for new objects
COMMENT ON FUNCTION typeid_prefix(typeid) IS 'Extract the prefix from a TypeID for indexing and filtering.';
COMMENT ON FUNCTION typeid_has_prefix(typeid, text) IS 'Check if a TypeID has a specific prefix.';
COMMENT ON FUNCTION typeid_is_valid(text) IS 'Validate if a string is a valid TypeID representation.';
COMMENT ON FUNCTION typeid_generate_nil() IS 'Generate a TypeID with an empty prefix.';
COMMENT ON FUNCTION typeid_generate_batch(text, int) IS 'Generate a batch of TypeIDs with the same prefix.';
COMMENT ON OPERATOR @>(typeid, text) IS 'Does the TypeID have the specified prefix?';
