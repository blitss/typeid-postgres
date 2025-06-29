pub mod aggregate;
pub mod base32;
pub mod typeid;

use pgrx::pg_extern;
use typeid::TypeID;
use typeid::TypeIDPrefix;
use uuid::Uuid;

use pgrx::prelude::*;

use std::hash::{Hash, Hasher};

pgrx::pg_module_magic!();

/// Generate a new **TypeID** using the supplied prefix.
///
/// # Usage
/// ```sql
/// -- Generate an ID in the "user" prefix
/// SELECT typeid_generate('user');
/// ```
///
/// * The `prefix` must be lowercase ASCII letters and underscores,  
///   1 – 63 chars (same rules as `TypeIDPrefix::new`).
/// * The UUID part is version-7, so IDs sort by creation time.
/// * Result is a value of the custom SQL type **`typeid`**.
///
/// The function is marked **VOLATILE** (depends on `now_v7()`),  
/// **STRICT** (NULL in ⇒ NULL out), and **PARALLEL SAFE**.
#[pg_extern(strict, volatile, parallel_safe)]
fn typeid_generate(prefix: &str) -> TypeID {
    match TypeIDPrefix::new(prefix) {
        Ok(prefix) => TypeID::new(prefix, Uuid::now_v7()),
        Err(err) => panic!("Invalid TypeID prefix: {}", err),
    }
}

/// Generate a new **TypeID** with empty prefix (UUID-only).
///
/// # Usage
/// ```sql
/// -- Generate an ID with no prefix
/// SELECT typeid_generate_nil();
/// ```
///
/// This is equivalent to `typeid_generate('')` but more explicit.
#[pg_extern(strict, volatile, parallel_safe)]
fn typeid_generate_nil() -> TypeID {
    TypeID::new(TypeIDPrefix::new("").unwrap(), Uuid::now_v7())
}

/// Check if a TypeID string is valid without parsing it.
///
/// # Usage
/// ```sql
/// SELECT typeid_is_valid('user_01h455vb4pex5vsknk084sn02q'); -- true
/// SELECT typeid_is_valid('invalid_id'); -- false
/// ```
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_is_valid(input: &str) -> bool {
    TypeID::from_string(input).is_ok()
}

/// Extract the prefix part (`TEXT`) from a TypeID.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_prefix(typeid: TypeID) -> String {
    typeid.type_prefix().to_string()
}

/// Loss-less conversions between UUID and TypeID.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_to_uuid(typeid: TypeID) -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*typeid.uuid().as_bytes())
}

#[pg_extern(strict, immutable, parallel_safe)]
fn uuid_to_typeid(prefix: &str, uuid: pgrx::Uuid) -> TypeID {
    let type_prefix = match TypeIDPrefix::new(prefix) {
        Ok(prefix) => prefix,
        Err(err) => panic!("Invalid TypeID prefix: {}", err),
    };

    let uuid = match Uuid::from_slice(uuid.as_bytes()) {
        Ok(uuid) => uuid,
        Err(err) => panic!("Invalid UUID: {}", err),
    };

    TypeID::new(type_prefix, uuid)
}

/// Comparison helpers — all pure, so *IMMUTABLE STRICT PARALLEL SAFE*.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_cmp(a: TypeID, b: TypeID) -> i32 {
    a.cmp(&b) as i32
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_lt(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) < 0
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_le(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) <= 0
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_eq(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) == 0
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_ge(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) >= 0
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_gt(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) > 0
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_ne(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) != 0
}

/// Hash helpers — deterministic, so also *IMMUTABLE*.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_hash(typeid: TypeID) -> i32 {
    let mut hasher = gxhash::GxHasher::default();
    typeid.hash(&mut hasher);
    hasher.finish() as i32
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_hash_extended(typeid: TypeID, seed: i64) -> i64 {
    let mut hasher = gxhash::GxHasher::with_seed(seed);
    typeid.hash(&mut hasher);
    hasher.finish() as i64
}

/// Generate a UUID v7, producing a Postgres uuid object
#[pg_extern]
fn typeid_uuid_generate_v7() -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*Uuid::now_v7().as_bytes())
}

extension_sql! {
r#"
/* ──────────────────────────────────────────────────────────────
 * Implicit cast: text → typeid
 *   Allows:     SELECT 'user_01h…' = id;
 *   Context:    IMPLICIT  (works everywhere a typeid is expected)
 *   Safety:     relies on typeid_in for validation; bad literals
 *               still fail with ERROR.
 * ──────────────────────────────────────────────────────────────*/
CREATE CAST (text AS typeid)
    WITH INOUT
    AS IMPLICIT;

/* ──────────────────────────────────────────────────────────────
 * Additional utility functions for better SQL integration
 * ──────────────────────────────────────────────────────────────*/

-- Create an operator for prefix matching to enable efficient queries
CREATE OPERATOR @> (
    LEFTARG = typeid,
    RIGHTARG = text,
    PROCEDURE = typeid_has_prefix,
    COMMUTATOR = '@<'
);

-- Create a functional index helper for prefix-based queries
-- Usage: CREATE INDEX idx_user_ids ON users (typeid_prefix(id)) WHERE typeid_has_prefix(id, 'user');
COMMENT ON FUNCTION typeid_prefix(typeid) IS 'Extract the prefix from a TypeID for indexing and filtering';
COMMENT ON FUNCTION typeid_has_prefix(typeid, text) IS 'Check if TypeID has a specific prefix - useful for filtering';
COMMENT ON FUNCTION typeid_is_valid(text) IS 'Validate TypeID format without parsing - useful for constraints';
COMMENT ON FUNCTION typeid_generate_nil() IS 'Generate TypeID with empty prefix (UUID-only format)';

   CREATE OPERATOR < (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_lt
    );

    CREATE OPERATOR <= (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_le
    );

    CREATE OPERATOR = (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_eq,
        COMMUTATOR = '=',
        NEGATOR = '<>',
        HASHES,
        MERGES
    );

    CREATE OPERATOR >= (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_ge
    );

    CREATE OPERATOR > (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_gt
    );

    CREATE OPERATOR <> (
        LEFTARG = typeid,
        RIGHTARG = typeid,
        PROCEDURE = typeid_ne
    );

    CREATE OPERATOR CLASS typeid_ops DEFAULT FOR TYPE typeid USING btree AS
        OPERATOR 1 < (typeid, typeid),
        OPERATOR 2 <= (typeid, typeid),
        OPERATOR 3 = (typeid, typeid),
        OPERATOR 4 >= (typeid, typeid),
        OPERATOR 5 > (typeid, typeid),
        FUNCTION 1 typeid_cmp(typeid, typeid);

        CREATE OPERATOR FAMILY typeid_hash_ops USING hash;

        CREATE OPERATOR CLASS typeid_hash_ops DEFAULT FOR TYPE typeid USING hash AS
            OPERATOR 1 = (typeid, typeid),
            FUNCTION 1 typeid_hash(typeid),
            FUNCTION 2 typeid_hash_extended(typeid, bigint);
    "#,
  name = "create_typeid_operator_class",
  finalize,
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use crate::TypeID;
    use pgrx::{datum::DatumWithOid, prelude::*};
    use uuid::Uuid;

    #[pg_test]
    fn test_typeid_generate() {
        let typeid = crate::typeid_generate("test");
        assert_eq!(typeid.type_prefix(), "test");
    }

    #[pg_test]
    fn test_uuid() {
        let uuid: pgrx::Uuid = crate::typeid_uuid_generate_v7();
        let converted: Uuid = Uuid::from_slice(uuid.as_bytes()).unwrap();

        println!("UUID: {:?}", uuid.to_string());

        assert_eq!(converted.get_version_num(), 7);
    }

    #[pg_test]
    fn test_hashing() {
        use crate::typeid_hash;
        use crate::TypeID;

        let id = TypeID::from_string("qual_01j1acv2aeehk8hcapaw7qyjvq").unwrap();
        let id2 = TypeID::from_string("qual_01j1acv2aeehk8hcapaw7qyjvq").unwrap();

        let hash = typeid_hash(id);
        let hash2 = typeid_hash(id2);
        println!("UUID: {:?}", hash);

        assert_eq!(
            hash, hash2,
            "Hashes should be consistent for the same input"
        );
    }

    #[pg_test]
    fn test_custom_type_in_query() {
        use crate::typeid_generate;
        // Create tables
        Spi::run("CREATE TABLE question (id typeid);").unwrap();
        Spi::run("CREATE TABLE answer (id typeid, question typeid);").unwrap();

        // Generate and insert test data
        let typeid1 = typeid_generate("qual");
        let typeid2 = typeid_generate("answer");
        let typeid3 = typeid_generate("answer");

        insert_into_table("question", &typeid1);

        insert_answer(&typeid2, &typeid1);
        insert_answer(&typeid3, &typeid1);

        // Execute the query and check results
        let result = Spi::get_one::<i64>(
            "SELECT COUNT(*) FROM answer WHERE question IN (SELECT id FROM question)",
        )
        .unwrap();
        assert_eq!(result, Some(2));
    }

    fn insert_answer(typeid: &TypeID, reference: &TypeID) {
        let query = format!(
            "INSERT INTO {} (id, question) VALUES ($1::typeid, $2::typeid)",
            "answer"
        );
        Spi::run_with_args(
            &query,
            &[
                DatumWithOid::from(typeid.clone()),
                DatumWithOid::from(reference.clone()),
            ],
        )
        .unwrap();
    }

    fn insert_into_table(table_name: &str, typeid: &TypeID) {
        let query = format!("INSERT INTO {} (id) VALUES ($1::typeid)", table_name);

        Spi::run_with_args(&query, &[DatumWithOid::from(typeid.clone())]).unwrap();
    }

    #[pg_test]
    fn literal_without_cast() {
        use pgrx::prelude::*;

        // Create a table with one value
        Spi::run("CREATE TEMP TABLE t(id typeid)").unwrap();

        let id = crate::typeid_generate("qual");

        Spi::run_with_args(
            "INSERT INTO t(id) VALUES ($1::typeid)",
            &[DatumWithOid::from(id.clone())],
        )
        .unwrap();

        let round =
            Spi::get_one::<i32>(&format!("SELECT 1 FROM t WHERE id = '{}'", id.to_string()))
                .unwrap();

        assert_eq!(round, Some(1), "text → typeid cast should round-trip");
    }

    #[pg_test]
    fn test_text_cast_roundtrip() {
        use crate::typeid_generate;

        // 1. generate a valid id as text
        let id = typeid_generate("user");
        let literal = id.to_string(); // "user_01h…"

        // 2. send it through text → typeid implicit cast
        let round: TypeID = Spi::get_one::<TypeID>(
            &format!("SELECT '{}'::text::typeid", literal), // explicit text, implicit to typeid
        )
        .unwrap()
        .unwrap();

        assert_eq!(round, id, "text → typeid cast should round-trip");
    }

    #[pg_test]
    fn implicit_text_to_typeid_cast_roundtrip() {
        use crate::typeid_generate;
        use pgrx::prelude::*;

        // ── 1. get a real TypeID and turn it into TEXT ───────────────────────────
        let id = typeid_generate("user"); // TypeID
        let id_text = id.to_string(); // String → TEXT literal

        // ── 2. compare TEXT with TYPEID via the = operator (expects typeid,typeid)
        //       This will parse only if Postgres finds an *implicit* cast from
        //       TEXT to TYPEID (the one you created with CREATE CAST … AS IMPLICIT)
        // ------------------------------------------------------------------------
        let same: bool = Spi::get_one_with_args(
            "SELECT $1::text = $2", // $1 is TEXT, $2 is TYPEID
            &[
                DatumWithOid::from(id_text.as_str()), // param $1  (TEXT)
                DatumWithOid::from(id.clone()),       // param $2  (TYPEID)
            ],
        )
        .expect("SPI failure") // Result<_, SpiError>
        .expect("NULL result"); // Option<bool>

        assert!(same, "TEXT literal should implicitly cast to typeid");
    }

    #[pg_test]
    fn test_new_utility_functions() {
        use crate::{
            typeid_generate, typeid_generate_nil, typeid_has_prefix, typeid_is_valid,
            typeid_to_uuid,
        };

        // Test nil generation
        let nil_id = typeid_generate_nil();
        assert_eq!(nil_id.type_prefix(), "");

        // Test validation
        assert!(typeid_is_valid("user_01h455vb4pex5vsknk084sn02q"));
        assert!(!typeid_is_valid("invalid_id"));
        assert!(!typeid_is_valid("User_01h455vb4pex5vsknk084sn02q")); // uppercase
        assert!(!typeid_is_valid("user_invalid")); // bad suffix

        // Test prefix checking
        let user_id = typeid_generate("user");
        assert!(typeid_has_prefix(user_id.clone(), "user"));
        assert!(!typeid_has_prefix(user_id.clone(), "admin"));

        // Test UUID extraction (use existing typeid_to_uuid function)
        let uuid = typeid_to_uuid(user_id.clone());
        let uuid_str = uuid.to_string();
        assert_eq!(uuid_str.len(), 36); // Standard UUID string length
        assert!(uuid_str.contains("-")); // Should be formatted UUID
    }

    #[pg_test]
    fn test_error_messages() {
        // Test invalid prefix errors
        let results = std::panic::catch_unwind(|| {
            crate::typeid_generate("User") // uppercase
        });
        assert!(results.is_err());

        let results = std::panic::catch_unwind(|| {
            crate::typeid_generate("user123") // numbers
        });
        assert!(results.is_err());

        let results = std::panic::catch_unwind(|| {
            crate::typeid_generate("_user") // starts with underscore
        });
        assert!(results.is_err());
    }

    #[pg_test]
    fn test_prefix_operator() {
        use pgrx::prelude::*;

        Spi::run("CREATE TEMP TABLE test_table (id typeid)").unwrap();

        let user_id = crate::typeid_generate("user");
        let admin_id = crate::typeid_generate("admin");

        Spi::run_with_args(
            "INSERT INTO test_table VALUES ($1), ($2)",
            &[
                DatumWithOid::from(user_id.clone()),
                DatumWithOid::from(admin_id.clone()),
            ],
        )
        .unwrap();

        // Test the @> operator for prefix matching
        let count: i64 = Spi::get_one("SELECT COUNT(*) FROM test_table WHERE id @> 'user'")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        let count: i64 = Spi::get_one("SELECT COUNT(*) FROM test_table WHERE id @> 'admin'")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        let count: i64 = Spi::get_one("SELECT COUNT(*) FROM test_table WHERE id @> 'nonexistent'")
            .unwrap()
            .unwrap();
        assert_eq!(count, 0);
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}

/// Check if a TypeID has a specific prefix.
///
/// # Usage
/// ```sql
/// SELECT * FROM users WHERE typeid_has_prefix(id, 'user');
/// ```
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_has_prefix(typeid: TypeID, prefix: &str) -> bool {
    typeid.type_prefix() == prefix
}

/// Check if a TypeID has an empty prefix (nil prefix).
///
/// # Usage
/// ```sql
/// SELECT typeid_is_nil_prefix(typeid_generate_nil()); -- true
/// SELECT typeid_is_nil_prefix(typeid_generate('user')); -- false
/// ```
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_is_nil_prefix(typeid: TypeID) -> bool {
    typeid.is_nil_prefix()
}

/// Generate multiple TypeIDs with the same prefix efficiently.
/// Useful for batch operations.
///
/// # Usage
/// ```sql
/// SELECT unnest(typeid_generate_batch('user', 5));
/// ```
#[pg_extern(strict, volatile, parallel_safe)]
fn typeid_generate_batch(prefix: &str, count: i32) -> Vec<TypeID> {
    if count <= 0 {
        return vec![];
    }

    let type_prefix = match TypeIDPrefix::new(prefix) {
        Ok(prefix) => prefix,
        Err(err) => panic!("Invalid TypeID prefix: {}", err),
    };

    (0..count)
        .map(|_| TypeID::new(type_prefix.clone(), Uuid::now_v7()))
        .collect()
}
