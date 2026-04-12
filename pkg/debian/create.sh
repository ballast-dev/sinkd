#!/usr/bin/env bash
# Build a .deb with a static musl sinkd (amd64/arm64) into artifacts/.
# Requires: dpkg-deb, cargo, rustup, musl toolchain (e.g. apt install musl-tools).
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)

if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "dpkg-deb not found. Install dpkg-dev (e.g. apt install dpkg-dev)." >&2
  exit 1
fi

VERSION=$(bump -b)
DEB_ARCH=$(dpkg --print-architecture)

MUSL_TARGET=x86_64-unknown-linux-musl
if [[ "${DEB_ARCH}" == "arm64" ]]; then
  MUSL_TARGET=aarch64-unknown-linux-musl
fi

OUT="${ROOT}/artifacts"
mkdir -p "${OUT}"

STAGE=$(mktemp -d)
cleanup() { rm -rf "${STAGE}"; }
trap cleanup EXIT

install -d "${STAGE}/usr/bin" \
  "${STAGE}/etc" \
  "${STAGE}/usr/share/sinkd" \
  "${STAGE}/etc/skel/.config/sinkd" \
  "${STAGE}/DEBIAN"

TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target}"
BIN="${TARGET_DIR}/${MUSL_TARGET}/release/sinkd"

install "${BIN}" "${STAGE}/usr/bin/sinkd"
install -m 644 LICENSE "${STAGE}/usr/share/doc/sinkd/copyright"
install -m 644 "${ROOT}/cfg/system/sinkd.conf" "${STAGE}/etc/sinkd.conf"
install -m 644 "${ROOT}/cfg/user/sinkd.conf" "${STAGE}/usr/share/sinkd/sinkd.user.conf"
# install -m 644 "${ROOT}/cfg/user/sinkd.conf" "${STAGE}/etc/skel/.config/sinkd/sinkd.conf"

cat >"${STAGE}/DEBIAN/control" <<EOF
Package: sinkd
Version: ${VERSION}
Architecture: ${DEB_ARCH}
Maintainer: Tony <krakjn@gmail.com>
Section: utils
Priority: optional
Depends: rsync
Description: Deployable Cloud, file sync daemon.
 bring everything and the kitchen sink to your local cloud.
EOF

dpkg-deb --root-owner-group --build "${STAGE}" "${OUT}/sinkd_${VERSION}_${DEB_ARCH}.deb"
