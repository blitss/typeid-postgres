# TypeID Postgres extension

[TypeID](https://github.com/jetify-com/typeid) is a modern extension of UUIDv7 which can be used instead of UUIDs. They are sortable and convertable to UUID.

This extension intends to add TypeIDs to the Postgres using [pgrx](https://github.com/pgcentralfoundation/pgrx).

Here's an example of a TypeID of type user:

```
user_2x4y6z8a0b1c2d3e4f5g6h7j8k
└──┘ └────────────────────────┘
type    uuid suffix (base32)
```

ID tag converts to `5d278df4-280b-0b04-d1b8-8f2c0d13c913` UUID holding a timestamp which allows to sort it.

This extension adds a TypeID type which outputs `user_2x4y6z8a0b1c2d3e4f5g6h7j8k` and stores a prefix and binary uuid inside. It also adds ability to use a TypeID as a primary key and allows to compare it to other TypeIDs.

Usage example:

```sql
create table users
(
  id typeid default typeid_generate('user') not null primary key,
  created_at timestamp default CURRENT_TIMESTAMP not null
);
```

You can insert some data:

```sql
WITH series AS (
    SELECT generate_series(1, 100000) AS id
)
INSERT INTO users (id)
SELECT typeid_generate('user')
FROM series;
```

Obviously it adds some overhead because of decoding/ encoding base52 (because the data is stored as UUID) so keep that in mind. But upon testing I don't think the performance implications are very noticable, inserting the 100k records took me around 800ms.

### Installation/ upgrade

There are multiple ways to use Postgres with TypeID extension:

1) Using pre-built postgres image with TypeID:

```bash
docker pull ghcr.io/blitss/typeid-pg:latest
```

You can see how it's built [here](https://github.com/blitss/typeid-postgres/blob/main/Dockerfile)

2) By using install script and downloading pre-built extension (must have Postgres installed and `pg_config` exposed in path)

```bash
curl -sSL https://github.com/blitss/typeid-postgres/blob/main/install.sh | sudo bash
```

Or you can specify the pg_config directly:

```bash
curl -sSL https://github.com/blitss/typeid-postgres/blob/main/install.sh | sudo bash -s -- /usr/pgsql-16/bin/pg_config
```

You can upgrade the same way and then run `ALTER EXTENSION typeid UPDATE;` in your Postgres database to run migration scripts. 

3) By building extension manually

Prerequisites:
* Postgres 13.x-17.x installed (probably from your package manager), including the "-server" package

* The Rust toolchain

* A 64bit Intel Architecture

Windows is not supported due to pgrx limitations.

Run these commands (replace pg13 with your pg version and `which pg_config` part with your pg path if necessary):

```bash
git clone https://github.com/blitss/typeid-postgres-extension.git
cd typeid-postgres-extension
cargo install cargo-pgx

cargo pgx init --pg13=`which pg_config`
cargo pgx install --release --sudo
```

After that use `CREATE EXTENSION typeid` to initialize an extension.

### Exposed functions

| Function | Return Type | Arguments | Description |
|---|---|---|---|
| `typeid_generate(prefix TEXT)` | `typeid` | `prefix TEXT` | Generate a new TypeID using the supplied prefix. The `prefix` must be lowercase ASCII letters, 1–63 chars. |
| `typeid_generate_nil()` | `typeid` | | Generate a new TypeID with an empty prefix. Equivalent to `typeid_generate('')`. |
| `typeid_is_valid(input TEXT)` | `BOOLEAN` | `input TEXT` | Check if a TypeID string is valid without parsing it. |
| `typeid_prefix(typeid)` | `TEXT` | `typeid typeid` | Extract the prefix part from a TypeID. |
| `typeid_to_uuid(typeid)` | `UUID` | `typeid typeid` | Convert a TypeID to a UUID. |
| `uuid_to_typeid(prefix TEXT, uuid UUID)` | `typeid` | `prefix TEXT`, `uuid UUID` | Convert a UUID to a TypeID with a given prefix. |
| `typeid_uuid_generate_v7()` | `UUID` | | Generate a UUID v7. |
| `typeid_has_prefix(typeid, prefix TEXT)` | `BOOLEAN` | `typeid typeid`, `prefix TEXT` | Check if a TypeID has a specific prefix. |
| `typeid_is_nil_prefix(typeid)` | `BOOLEAN` | `typeid typeid` | Check if a TypeID has a nil prefix. |
| `typeid_generate_batch(prefix TEXT, count INTEGER)` | `SETOF typeid` | `prefix TEXT`, `count INTEGER` | Generate a batch of TypeIDs. |

### Exposed Aggregates

| Aggregate | Return Type | Arguments | Description |
|---|---|---|---|
| `min(typeid)` | `typeid` | `typeid` | Returns the minimum TypeID in a group. |
| `max(typeid)` | `typeid` | `typeid` | Returns the maximum TypeID in a group. |

### Exposed Operators

This extension also creates a set of operators (`<`, `<=`, `=`, `>=`, `>`, `<>`) for comparing TypeIDs, and a `@>` operator for checking if a `typeid` has a certain prefix (e.g., `id @> 'user'`).
It also creates a `btree` operator class for indexing TypeIDs.