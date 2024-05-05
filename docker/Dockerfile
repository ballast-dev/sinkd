FROM rust:1-slim-buster
RUN apt-get update && apt-get install --no-install-recommends --yes\
  build-essential \
  cmake \
  libssl-dev \
  pkg-config

### other libs
#openssl-libs-static openssl-dev perl make cmake
# openssl3-libs-static-3.0.7-r0
# openssl-libs-static-1.1.1s-r1
# llvm-dev
# clang
# clang-static

RUN mkdir /repo
WORKDIR /repo
COPY --chmod=555 perms.sh /usr/local/bin/perms.sh
CMD [ "/bin/bash" ]

