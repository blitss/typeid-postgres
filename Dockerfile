ARG PG_VERSION=18
# The Debian release under Postgres. Pin it to keep a data directory's glibc: another glibc collates text differently,
# so text indexes built before would need a REINDEX (Postgres warns "collation version mismatch").
ARG DEBIAN=trixie
FROM postgres:${PG_VERSION}-${DEBIAN}

# The release to install, without the leading "v" (e.g. 0.4.2); its assets must exist (the release job runs first).
ARG RELEASE_VERSION

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY install.sh /tmp/install.sh
RUN test -n "${RELEASE_VERSION}" || { echo "build-arg RELEASE_VERSION is required (e.g. 0.4.2)" >&2; exit 1; } \
 && RELEASE_VERSION="${RELEASE_VERSION}" bash /tmp/install.sh && rm /tmp/install.sh

# Enable the extension
RUN echo "shared_preload_libraries = 'typeid'" >> /usr/share/postgresql/postgresql.conf.sample
