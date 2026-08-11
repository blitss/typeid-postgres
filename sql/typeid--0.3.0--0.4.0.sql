/*
 * Give the comparison operators the selectivity estimators they were missing.
 *
 * Until now none of them declared RESTRICT or JOIN, so the planner had no way
 * to estimate how many rows a predicate matches and fell back to a fixed guess
 * of ~50% of the table — even for equality on a unique primary key. At that
 * estimate a sequential scan always looks cheaper than an index scan, so
 * indexes on typeid columns were built, maintained, and then never used.
 *
 * Catalog-only: no table is rewritten and no data is touched. It applies to
 * plans compiled after this runs, so connections with cached plans should be
 * recycled (or the pool bounced) to pick it up.
 */

ALTER OPERATOR =  (typeid, typeid) SET (RESTRICT = eqsel,       JOIN = eqjoinsel);
ALTER OPERATOR <> (typeid, typeid) SET (RESTRICT = neqsel,      JOIN = neqjoinsel);
ALTER OPERATOR <  (typeid, typeid) SET (RESTRICT = scalarltsel, JOIN = scalarltjoinsel);
ALTER OPERATOR <= (typeid, typeid) SET (RESTRICT = scalarlesel, JOIN = scalarlejoinsel);
ALTER OPERATOR >  (typeid, typeid) SET (RESTRICT = scalargtsel, JOIN = scalargtjoinsel);
ALTER OPERATOR >= (typeid, typeid) SET (RESTRICT = scalargesel, JOIN = scalargejoinsel);

/*
 * Commutators and negators for the ordering operators, so the planner can flip
 * predicates into index-friendly form. ALTER OPERATOR only accepts these when
 * they are not already set, which is the case here.
 */
ALTER OPERATOR <  (typeid, typeid) SET (COMMUTATOR = >,  NEGATOR = >=);
ALTER OPERATOR <= (typeid, typeid) SET (COMMUTATOR = >=, NEGATOR = >);
ALTER OPERATOR >  (typeid, typeid) SET (COMMUTATOR = <,  NEGATOR = <=);
ALTER OPERATOR >= (typeid, typeid) SET (COMMUTATOR = <=, NEGATOR = <);
ALTER OPERATOR <> (typeid, typeid) SET (COMMUTATOR = <>, NEGATOR = =);
