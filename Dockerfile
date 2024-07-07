ARG PG_VERSION=16

# Stage 1: Build the extension
FROM rust:latest AS builder

ARG PG_VERSION=16

RUN apt-get update && apt-get install -y \
    wget \
    gnupg \
    lsb-release \
    && wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add - \
    && echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install -y postgresql-${PG_VERSION} postgresql-server-dev-${PG_VERSION}

WORKDIR /usr/src/typeid

COPY . .

RUN cargo install cargo-pgrx \
    && cargo pgrx init --pg${PG_VERSION} /usr/lib/postgresql/${PG_VERSION}/bin/pg_config \
    && cargo pgrx package --pg-config /usr/lib/postgresql/${PG_VERSION}/bin/pg_config

  RUN ls -R /usr/src/typeid/target/release/

# Stage 2: Create the final Postgres image with the extension
FROM postgres:${PG_VERSION}

ARG PG_VERSION=16

# Copy the built extension files from the builder stage
# Copy the built extension files
COPY --from=builder /usr/src/typeid/target/release/typeid-pg${PG_VERSION}/usr/lib/postgresql/${PG_VERSION}/lib/typeid.so /usr/lib/postgresql/${PG_VERSION}/lib/
COPY --from=builder /usr/src/typeid/target/release/typeid-pg${PG_VERSION}/usr/share/postgresql/${PG_VERSION}/extension/typeid.control /usr/share/postgresql/${PG_VERSION}/extension/
COPY --from=builder /usr/src/typeid/target/release/typeid-pg${PG_VERSION}/usr/share/postgresql/${PG_VERSION}/extension/typeid--0.1.0.sql /usr/share/postgresql/${PG_VERSION}/extension/

# Enable the extension
RUN echo "shared_preload_libraries = 'typeid'" >> /usr/share/postgresql/postgresql.conf.sample