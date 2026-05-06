#!/usr/bin/env bash
# Build a .deb with a static musl sinkd (amd64/arm64) into pkg/artifacts/.
# Requires: dpkg-deb, bump. Binary: target/*/release/sinkd for the musl triple, or set SINKD_BIN.
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)

if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "dpkg-deb not found. Install dpkg-dev (e.g. apt install dpkg-dev)." >&2
  exit 1
fi

# Allow callers (CI, container builds) to inject the version directly to avoid
# requiring `bump` on the build host.
if [[ -n "${SINKD_VERSION:-}" ]]; then
  VERSION="${SINKD_VERSION}"
elif command -v bump >/dev/null 2>&1; then
  VERSION=$(bump -b)
else
  # Last-resort fallback: lift the base version from Cargo.toml.
  VERSION=$(awk -F'"' '/^version[[:space:]]*=/ { split($2, a, "+"); print a[1]; exit }' "${ROOT}/client/Cargo.toml")
fi

if [[ -z "${VERSION}" ]]; then
  echo "could not determine version (set SINKD_VERSION, install bump, or fix Cargo.toml)." >&2
  exit 1
fi

DEB_ARCH=$(dpkg --print-architecture)

MUSL_TARGET=x86_64-unknown-linux-musl
if [[ "${DEB_ARCH}" == "arm64" ]]; then
  MUSL_TARGET=aarch64-unknown-linux-musl
fi

OUT="${ROOT}/pkg/artifacts"
mkdir -p "${OUT}"

STAGE=$(mktemp -d)
cleanup() { rm -rf "${STAGE}"; }
trap cleanup EXIT

install -d "${STAGE}/usr/bin" \
  "${STAGE}/etc" \
  "${STAGE}/usr/share/sinkd" \
  "${STAGE}/usr/share/doc/sinkd" \
  "${STAGE}/etc/skel/.config/sinkd" \
  "${STAGE}/DEBIAN"

TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target}"
BIN="${SINKD_BIN:-${TARGET_DIR}/${MUSL_TARGET}/release/sinkd}"
BIN_SRV="${SINKD_SRV_BIN:-${TARGET_DIR}/${MUSL_TARGET}/release/sinkd-srv}"

if [[ ! -f "${BIN}" ]]; then
  echo "Missing binary at ${BIN}. Build sinkd for ${MUSL_TARGET} first or set SINKD_BIN." >&2
  exit 1
fi
if [[ ! -f "${BIN_SRV}" ]]; then
  echo "Missing binary at ${BIN_SRV}. Build sinkd-srv for ${MUSL_TARGET} first or set SINKD_SRV_BIN." >&2
  exit 1
fi

install "${BIN}" "${STAGE}/usr/bin/sinkd"
install "${BIN_SRV}" "${STAGE}/usr/bin/sinkd-srv"
install -m 644 LICENSE "${STAGE}/usr/share/doc/sinkd/copyright"
install -m 644 "${ROOT}/cfg/system/sinkd.conf" "${STAGE}/etc/sinkd.conf"
install -m 644 "${ROOT}/cfg/user/sinkd.conf" "${STAGE}/usr/share/sinkd/sinkd.user.conf.example"

# Init templates consumed by `sinkd init` / `sinkd-srv init` (disk-first;
# binary falls back to embedded copies if these are missing).
install -m 644 "${ROOT}/server/conf.tmpl" "${STAGE}/usr/share/sinkd/sinkd.conf"
install -m 644 "${ROOT}/client/conf.tmpl"   "${STAGE}/usr/share/sinkd/sinkd.user.conf"

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
