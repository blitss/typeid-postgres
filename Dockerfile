ARG PG_VERSION=16
FROM postgres:${PG_VERSION}

RUN apt update && apt install -y curl
RUN curl -sSL https://raw.githubusercontent.com/blitss/typeid-postgres/main/install.sh | bash

# Enable the extension
RUN echo "shared_preload_libraries = 'typeid'" >> /usr/share/postgresql/postgresql.conf.sample