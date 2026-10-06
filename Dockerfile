ARG PG_VERSION=18
# The Debian release under Postgres. Pin it to keep a data directory's glibc: another glibc collates text differently,
# so text indexes built before would need a REINDEX (Postgres warns "collation version mismatch").
ARG DEBIAN=trixie
FROM postgres:${PG_VERSION}-${DEBIAN}

ARG PG_VERSION
# The release whose binaries to install, without the leading "v" (e.g. 0.4.1).
ARG RELEASE_VERSION
# Set by buildx: amd64 | arm64, as the release assets are named.
ARG TARGETARCH

# The release tarball is laid out as the Debian Postgres paths (usr/lib/postgresql/<pg>/lib, usr/share/postgresql/<pg>/extension).
RUN test -n "${RELEASE_VERSION}" || { echo "build-arg RELEASE_VERSION is required (e.g. 0.4.1)" >&2; exit 1; } \
 && apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/* \
 && curl -fsSL --retry 3 "https://github.com/blitss/typeid-postgres/releases/download/v${RELEASE_VERSION}/typeid-pg${PG_VERSION}-linux-${TARGETARCH}.tar.gz" \
    | tar -xz -C / \
 && test -f "/usr/lib/postgresql/${PG_VERSION}/lib/typeid.so"

# Enable the extension
RUN echo "shared_preload_libraries = 'typeid'" >> /usr/share/postgresql/postgresql.conf.sample
