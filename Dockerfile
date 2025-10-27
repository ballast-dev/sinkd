FROM rust:1.90-alpine

RUN apk update && apk add \
  build-base \
  cmake \
  fd \
  just \
  mosquitto \
  mosquitto-dev \
  openssh \
  openssl \
  openssl-dev \
  openssl-libs-static \
  rsync \
  sudo

# Allow wheel group to run sudo without password
RUN echo '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers


COPY <<'EOF' /entrypoint.sh
#!/bin/sh
USER=sinkd
PASSWORD=$(openssl passwd -1 sinkd)
adduser -D -h /home/sinkd -G wheel sinkd
PATH="/usr/local/rustup/toolchains/1.90.0-aarch64-unknown-linux-musl/bin:${PATH}"
PATH="/usr/local/rustup/toolchains/1.90.0-x86_64-unknown-linux-musl/bin:${PATH}"
echo "sinkd:${PASSWORD}" | chpasswd > /dev/null 2>&1
exec su -l "${USER}" -c "cd ${WORKDIR:-~}; PATH=${PATH} $*"
EOF

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
