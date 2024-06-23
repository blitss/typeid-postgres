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

### Installation
Installation should be performed from source.

Prerequisites:
* Postgres 11.x-16.x installed (probably from your package manager), including the "-server" package

* The Rust toolchain

* A 64bit Intel Architecture

Windows is not supported due to pgrx limitations.

Run these commands (replace pg12 with your pg version and `which pg_config` part with your pg path if necessary):

```bash
git clone https://github.com/blitss/typeid-postgres-extension.git
cd typeid-postgres-extension
cargo install cargo-pgx

cargo pgx init --pg12=`which pg_config`
cargo pgx install --release
```

After that use `CREATE EXTENSION typeid` to initialize an extension.

### Exposed functions

```
 Schema |       Name       | Result data type | Argument data types | Type | Volatility | Parallel |   Owner   | Security | Access privileges | Langu
age |      Internal name       | Description 
--------+------------------+------------------+---------------------+------+------------+----------+-----------+----------+-------------------+------
----+--------------------------+-------------
 public | typeid_cmp       | integer          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c    
    | typeid_cmp_wrapper       | 
 public | typeid_eq        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c    
    | typeid_eq_wrapper        | 
 public | typeid_ge        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c    
    | typeid_ge_wrapper        | 
 public | typeid_generate  | typeid           | prefix text         | func | volatile   | unsafe   | codespace | invoker  |                   | c    
    | typeid_generate_wrapper  | 
 public | typeid_gt        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c    
    | typeid_gt_wrapper        | 
 public | typeid_in        | typeid           | input cstring       | func | immutable  | safe     | codespace | invoker  |                   | c        | typeid_in_wrapper        | 
 public | typeid_le        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c        | typeid_le_wrapper        | 
 public | typeid_lt        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c        | typeid_lt_wrapper        | 
 public | typeid_ne        | boolean          | a typeid, b typeid  | func | volatile   | unsafe   | codespace | invoker  |                   | c        | typeid_ne_wrapper        | 
 public | typeid_out       | cstring          | input typeid        | func | immutable  | safe     | codespace | invoker  |                   | c        | typeid_out_wrapper       | 
 public | uuid_generate_v7 | uuid             |                     | func | volatile   | unsafe   | codespace | invoker  |                   | c        | uuid_generate_v7_wrapper | 
(11 rows)
```