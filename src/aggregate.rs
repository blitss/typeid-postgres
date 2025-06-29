use crate::typeid::TypeID;
use pgrx::{aggregate::*, pg_aggregate, pg_sys, AggregateName};

#[derive(AggregateName)]
#[aggregate_name = "min"]
pub struct TypeIDMin;

#[derive(AggregateName)]
#[aggregate_name = "max"]
pub struct TypeIDMax;

#[pg_aggregate(parallel_safe, strict)]
impl Aggregate<TypeIDMin> for TypeIDMin {
    const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
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

    /// Allow parallel aggregation
    fn combine(
        left: Self::State,
        right: Self::State,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::State {
        match (left, right) {
            (None, s) | (s, None) => s,
            (Some(a), Some(b)) => Some(if a < b { a } else { b }),
        }
    }
}

#[pg_aggregate(parallel_safe, strict)]
impl Aggregate<TypeIDMax> for TypeIDMax {
    const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
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

    fn combine(
        left: Self::State,
        right: Self::State,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::State {
        match (left, right) {
            (None, s) | (s, None) => s,
            (Some(a), Some(b)) => Some(if a > b { a } else { b }),
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
        Spi::connect_mut(|client| {
            // Create a temporary table
            client
                .update("CREATE TEMPORARY TABLE test_typeid (id typeid)", None, &[])
                .unwrap();

            // Insert some test data
            client.update("INSERT INTO test_typeid VALUES (typeid_generate('user')), (typeid_generate('user')), (typeid_generate('user'))", None, &[]).unwrap();

            // Test min aggregate
            let result = client
                .select("SELECT min(id) FROM test_typeid", None, &[])
                .unwrap();

            assert_eq!(result.len(), 1);
            let min_typeid: TypeID = result
                .first()
                .get_one()
                .unwrap()
                .expect("didnt get min typeid");

            // Test max aggregate
            let result = client
                .select("SELECT max(id) FROM test_typeid", None, &[])
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
            client.update("TRUNCATE test_typeid", None, &[]).unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, &[])
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
                    &[],
                )
                .unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, &[])
                .unwrap();
            assert_eq!(result.len(), 1);
            let (min_typeid, max_typeid): (Option<TypeID>, Option<TypeID>) =
                result.first().get_two().unwrap();

            assert_eq!(min_typeid.unwrap(), max_typeid.unwrap());

            // Test with multiple prefixes
            client.update("TRUNCATE test_typeid", None, &[]).unwrap();
            client.update("INSERT INTO test_typeid VALUES (typeid_generate('user')), (typeid_generate('post')), (typeid_generate('comment'))", None, &[]).unwrap();
            let result = client
                .select("SELECT min(id), max(id) FROM test_typeid", None, &[])
                .unwrap();
            assert_eq!(result.len(), 1);
            let (min_typeid, max_typeid): (Option<TypeID>, Option<TypeID>) =
                result.first().get_two().unwrap();

            assert!(min_typeid.unwrap().type_prefix() == "comment");
            assert!(max_typeid.unwrap().type_prefix() == "user");
        })
    }
}
