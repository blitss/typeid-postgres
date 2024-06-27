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

#[pg_extern]
fn typeid_generate(prefix: &str) -> TypeID {
    TypeID::new(TypeIDPrefix::new(prefix).unwrap(), Uuid::now_v7())
}

#[pg_extern]
fn typeid_to_uuid(typeid: TypeID) -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*typeid.uuid().as_bytes())
}

#[pg_extern]
fn uuid_to_typeid(prefix: &str, uuid: pgrx::Uuid) -> TypeID {
    TypeID::new(
        TypeIDPrefix::new(prefix).unwrap(),
        Uuid::from_slice(uuid.as_bytes()).unwrap(),
    )
}

#[pg_extern]
fn typeid_cmp(a: TypeID, b: TypeID) -> i32 {
    a.cmp(&b) as i32
}

#[pg_extern]
fn typeid_lt(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) < 0
}

#[pg_extern]
fn typeid_le(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) <= 0
}

#[pg_extern]
fn typeid_eq(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) == 0
}

#[pg_extern]
fn typeid_ge(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) >= 0
}

#[pg_extern]
fn typeid_gt(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) > 0
}

#[pg_extern]
fn typeid_ne(a: TypeID, b: TypeID) -> bool {
    typeid_cmp(a, b) != 0
}

#[pg_extern]
fn typeid_hash(typeid: TypeID) -> i32 {
    let mut hasher = gxhash::GxHasher::default();
    typeid.hash(&mut hasher);
    hasher.finish() as i32
}

#[pg_extern]
fn typeid_hash_extended(typeid: TypeID, seed: i64) -> i64 {
    let mut hasher = gxhash::GxHasher::with_seed(seed);

    typeid.hash(&mut hasher);
    hasher.finish() as i64
}

extension_sql! {
r#"
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

/// Generate a UUID v7, producing a Postgres uuid object
#[pg_extern]
fn uuid_generate_v7() -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*Uuid::now_v7().as_bytes())
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use crate::TypeID;
    use pgrx::prelude::*;
    use uuid::Uuid;

    #[pg_test]
    fn test_typeid_generate() {
        let typeid = crate::typeid_generate("test");
        assert_eq!(typeid.type_prefix(), "test");
    }

    #[pg_test]
    fn test_uuid() {
        let uuid: pgrx::Uuid = crate::uuid_generate_v7();
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

    fn oid_for_type(type_name: &str) -> Result<Option<PgOid>, pgrx::spi::Error> {
        use crate::pg_sys::Oid;

        let oid = Spi::get_one_with_args::<u32>(
            "SELECT oid FROM pg_type WHERE typname = $1",
            vec![(PgBuiltInOids::TEXTOID.oid(), type_name.into_datum())],
        )?;
        Ok(oid.map(|oid| PgOid::from(Oid::from(oid))))
    }

    fn insert_answer(typeid: &TypeID, reference: &TypeID) {
        let query = format!(
            "INSERT INTO {} (id, question) VALUES ($1::typeid, $2::typeid)",
            "answer"
        );
        let oid = oid_for_type("typeid")
            .unwrap()
            .expect("expected to find oid");

        Spi::run_with_args(
            &query,
            Some(vec![
                (oid, typeid.clone().into_datum()),
                (oid, reference.clone().into_datum()),
            ]),
        )
        .unwrap();
    }

    fn insert_into_table(table_name: &str, typeid: &TypeID) {
        let query = format!("INSERT INTO {} (id) VALUES ($1::typeid)", table_name);
        let oid = oid_for_type("typeid").unwrap();

        Spi::run_with_args(
            &query,
            Some(vec![(
                oid.expect("expected to find oid"),
                typeid.clone().into_datum(),
            )]),
        )
        .unwrap();
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
