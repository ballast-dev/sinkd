FROM rust:1.90-alpine

RUN apk update && apk add \
  build-base \
  cmake \
  mosquitto \
  mosquitto-dev \
  openssl \
  openssl-dev \
  openssl-libs-static \
  rsync \
  openssh \
  sudo

# Allow wheel group to run sudo without password
RUN echo '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

COPY <<'EOF' /entrypoint.sh
#!/bin/sh
USER=${1:-sinkd}
PASSWORD=$(openssl passwd -1 ${2:-sinkd})

adduser -D -h /home/${USER} -G wheel "${USER}"
echo "${USER}:${PASSWORD}" | chpasswd > /dev/null 2>&1
exec su - "${USER}" sh -c "$@"
EOF

RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
