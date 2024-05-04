FROM rust:1-slim-buster
#openssl-libs-static openssl-dev perl make cmake
RUN apt-get update && apt-get install \
    libssl-dev \
    pkg-config \
    cmake 


### other libs
# openssl3-libs-static-3.0.7-r0
# openssl-libs-static-1.1.1s-r1
# llvm-dev
# clang
# clang-static

RUN mkdir /repo
WORKDIR /repo
COPY --chmod=555 perms.sh /usr/local/bin/perms.sh
# CMD ["perms.sh"]
