use pgrx::{aggregate::*, pg_aggregate, pg_sys};

use crate::typeid::TypeID;

pub struct TypeIDMin;
pub struct TypeIDMax;

#[pg_aggregate]
impl Aggregate for TypeIDMin {
    const NAME: &'static str = "min";
    type Args = TypeID;
    type State = Option<TypeID>;

    fn state(
        current: Self::State,
        arg: Self::Args,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::State {
        match current {
            None => Some(arg),
            Some(current) => Some(if arg < current { arg } else { current }),
        }
    }
}

#[pg_aggregate]
impl Aggregate for TypeIDMax {
    const NAME: &'static str = "max";
    type Args = TypeID;
    type State = Option<TypeID>;

    fn state(
        current: Self::State,
        arg: Self::Args,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::State {
        match current {
            None => Some(arg),
            Some(current) => Some(if arg > current { arg } else { current }),
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_typeid_min_max_aggregates() {
        Spi::connect(|mut client| {
            // Create a temporary table
            client
                .update("CREATE TEMPORARY TABLE test_typeid (id typeid)", None, None)
                .unwrap();

            // Insert some test data
            client.update("INSERT INTO test_typeid VALUES (typeid_generate('user')), (typeid_generate('user')), (typeid_generate('user'))", None, None).unwrap();

            // Test min aggregate
            let result = client
                .select("SELECT min(id) FROM test_typeid", None, None)
                .unwrap();

            assert_eq!(result.len(), 1);
            let min_typeid: TypeID = result
                .first()
                .get_one()
                .unwrap()
                .expect("didnt get min typeid");

            // Test max aggregate
            let result = client
                .select("SELECT max(id) FROM test_typeid", None, None)
                .unwrap();
            assert_eq!(result.len(), 1);
            let max_typeid: TypeID = result
                .first()
                .get_one()
                .unwrap()
                .expect("didnt get max typeid");

            // Verify that max is greater than min
            assert!(max_typeid > min_typeid);

            // Test with empty table
            client.update("TRUNCATE test_typeid", None, None).unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, None)
                .unwrap();
            assert_eq!(result.len(), 1);

            let (min_typeid, max_typeid): (Option<TypeID>, Option<TypeID>) =
                result.first().get_two().unwrap();
            assert_eq!(min_typeid, None);
            assert_eq!(max_typeid, None);

            // Test with single value
            client
                .update(
                    "INSERT INTO test_typeid VALUES (typeid_generate('user'))",
                    None,
                    None,
                )
                .unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, None)
                .unwrap();
            assert_eq!(result.len(), 1);
            let (min_typeid, max_typeid): (Option<TypeID>, Option<TypeID>) =
                result.first().get_two().unwrap();

            assert_eq!(min_typeid.unwrap(), max_typeid.unwrap());

            // Test with multiple prefixes
            client.update("TRUNCATE test_typeid", None, None).unwrap();
            client.update("INSERT INTO test_typeid VALUES (typeid_generate('user')), (typeid_generate('post')), (typeid_generate('comment'))", None, None).unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, None)
                .unwrap();
            assert_eq!(result.len(), 1);
            let (min_typeid, max_typeid): (Option<TypeID>, Option<TypeID>) =
                result.first().get_two().unwrap();

            assert!(min_typeid.unwrap().type_prefix() == "comment");
            assert!(max_typeid.unwrap().type_prefix() == "user");
        })
    }
}
