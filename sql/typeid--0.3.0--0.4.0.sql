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
 * predicates into index-friendly form.
 *
 * Only possible on PostgreSQL 17 and newer: before that, ALTER OPERATOR
 * accepts nothing but RESTRICT and JOIN, and asking for a commutator fails
 * with "operator attribute \"commutator\" cannot be changed" no matter what
 * the operator currently looks like. On 14-16 an upgraded installation
 * therefore keeps the estimators above but not these links. Freshly created
 * 0.4.0 extensions get them from CREATE OPERATOR on every supported version.
 *
 * Even on 17+ each link is set only when absent, because Postgres refuses to
 * change one that already exists and it fills them in behind our back: naming
 * NEGATOR = '<>' on = links <> back to = as well, and setting one side of a
 * pair completes the other side automatically. Which links a given
 * installation already has depends on the order 0.3.0 created its operators.
 */
DO $$
DECLARE
    pairs text[][] := ARRAY[
        -- operator, commutator, negator
        ARRAY['<',  '>',  '>='],
        ARRAY['<=', '>=', '>'],
        ARRAY['>',  '<',  '<='],
        ARRAY['>=', '<=', '<'],
        ARRAY['<>', '<>', '=']
    ];
    i        int;
    op       text;
    has_com  boolean;
    has_neg  boolean;
BEGIN
    IF current_setting('server_version_num')::int < 170000 THEN
        RAISE NOTICE
            'typeid: skipping commutator/negator links - ALTER OPERATOR gained them in PostgreSQL 17 (this server is %)',
            current_setting('server_version');
        RETURN;
    END IF;

    FOR i IN 1 .. array_length(pairs, 1) LOOP
        op := pairs[i][1];

        SELECT o.oprcom <> 0, o.oprnegate <> 0
          INTO has_com, has_neg
          FROM pg_operator o
         WHERE o.oprname  = op
           AND o.oprleft  = 'typeid'::regtype
           AND o.oprright = 'typeid'::regtype;

        IF has_com IS NULL THEN
            CONTINUE;  -- operator absent; nothing to link
        END IF;

        IF NOT has_com THEN
            EXECUTE format(
                'ALTER OPERATOR %s (typeid, typeid) SET (COMMUTATOR = %s)',
                op, pairs[i][2]);
        END IF;

        IF NOT has_neg THEN
            EXECUTE format(
                'ALTER OPERATOR %s (typeid, typeid) SET (NEGATOR = %s)',
                op, pairs[i][3]);
        END IF;
    END LOOP;
END $$;

/*
 * Fill in the @< shell operator.
 *
 * @> named @< as its commutator without anything ever defining it, so every
 * installation carries a shell that errors on use:
 *
 *   SELECT 'user' @< id;
 *   ERROR:  operator is only a shell: text @< typeid
 *
 * CREATE OPERATOR fills an existing shell rather than conflicting with it, so
 * this is safe to run against databases that already have one.
 */
CREATE FUNCTION typeid_prefix_matches(text, typeid) RETURNS boolean
    LANGUAGE sql
    IMMUTABLE
    PARALLEL SAFE
    STRICT
    AS $$ SELECT typeid_has_prefix($2, $1) $$;

COMMENT ON FUNCTION typeid_prefix_matches(text, typeid) IS 'Commutator form of typeid_has_prefix - backs the @< operator';

CREATE OPERATOR @< (
    LEFTARG = text,
    RIGHTARG = typeid,
    PROCEDURE = typeid_prefix_matches,
    COMMUTATOR = '@>',
    RESTRICT = contsel,
    JOIN = contjoinsel
);

/* Containment estimators for the prefix operator, absent until now. */
ALTER OPERATOR @> (typeid, text) SET (RESTRICT = contsel, JOIN = contjoinsel);
