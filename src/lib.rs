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
/// -- Generate an ID in the “user” prefix
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
    TypeID::new(TypeIDPrefix::new(prefix).unwrap(), Uuid::now_v7())
}

/// Extract the prefix part (`TEXT`) from a TypeID.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_prefix(id: TypeID) -> String {
    id.type_prefix().to_string()
}

/// Loss-less conversions between UUID and TypeID.
#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_to_uuid(id: TypeID) -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*id.uuid().as_bytes())
}

#[pg_extern(strict, immutable, parallel_safe)]
fn uuid_to_typeid(prefix: &str, uuid: pgrx::Uuid) -> TypeID {
    TypeID::new(
        TypeIDPrefix::new(prefix).unwrap(),
        Uuid::from_slice(uuid.as_bytes()).unwrap(),
    )
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
fn typeid_hash(id: TypeID) -> i32 {
    let mut hasher = gxhash::GxHasher::default();
    id.hash(&mut hasher);
    hasher.finish() as i32
}

#[pg_extern(strict, immutable, parallel_safe)]
fn typeid_hash_extended(id: TypeID, seed: i64) -> i64 {
    let mut hasher = gxhash::GxHasher::with_seed(seed);
    id.hash(&mut hasher);
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
