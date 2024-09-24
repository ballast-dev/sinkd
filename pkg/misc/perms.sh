#!/bin/sh
REPO_ROOT=/repo
UID=$(stat -c "%u" ${REPO_ROOT})
GID=$(stat -c "%g" ${REPO_ROOT})

mkdir -p /home/sinkd
groupadd --non-unique --gid $GID sinkd
useradd --home-dir /home/sinkd \
	--gid $GID \
	--uid $UID \
	--password $(openssl passwd -1 sinkd) \
	--groups sudo \
	--shell /bin/bash \
	sinkd

echo "PATH=${PATH}" >>/home/sinkd/.bashrc

su --login sinkd
