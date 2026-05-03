#!/usr/bin/env bash
# Emit a deterministic file-tree manifest for a directory:
#   <sha256>  ./relative/path
# Lines are sorted by path so the manifests of equivalent trees are bytewise
# identical and `diff -u` can compare them directly.
#
# usage: sinkd-snapshot <abs-dir>
set -euo pipefail

DIR="${1:?usage: sinkd-snapshot DIR}"

if [[ ! -d "$DIR" ]]; then
    echo "sinkd-snapshot: not a directory: $DIR" >&2
    exit 2
fi

cd "$DIR"

# Exclude the .events_done sentinel and any rsync partial-transfer artefacts so
# the manifest reflects the steady-state tree only.
find . -type f \
       ! -name '.events_done' \
       ! -name '.*.partial' \
       -print0 \
    | LC_ALL=C sort -z \
    | xargs -0 -r sha256sum
