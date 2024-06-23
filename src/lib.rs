pub mod base32;
pub mod typeid;

use pgrx::pg_extern;
use typeid::TypeID;
use uuid::Uuid;

use pgrx::prelude::*;

pgrx::pg_module_magic!();


#[pg_extern]
fn typeid_generate(prefix: &str) -> TypeID {
    TypeID::new(prefix.to_string(), Uuid::now_v7())
}

#[pg_extern]
fn typeid_to_uuid(typeid: TypeID) -> pgrx::Uuid {
    pgrx::Uuid::from_bytes(*typeid.uuid().as_bytes())
}

#[pg_extern]
fn uuid_to_typeid(prefix: &str, uuid: pgrx::Uuid) -> TypeID {
    TypeID::new(prefix.to_string(), Uuid::from_slice(uuid.as_bytes()).unwrap())
}

#[pg_extern]
fn typeid_cmp(a: TypeID, b: TypeID) -> i32 {
    match a.type_prefix().cmp(b.type_prefix()) {
        std::cmp::Ordering::Equal => a.uuid().cmp(b.uuid()) as i32,
        other => other as i32,
    }
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

extension_sql!{
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
