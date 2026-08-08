#!/usr/bin/env bash
set -euo pipefail

LUMIS_REVISION=3507439b896fe281b878a830c7ccc25fac555ed8

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root_dir=$(dirname -- "$script_dir")
vendor_dir="$root_dir/vendor/lumis"
temporary=$(mktemp -d "${TMPDIR:-/tmp}/aster-lumis.XXXXXX")
trap 'rm -rf -- "$temporary"' EXIT

source_dir="$temporary/source"
mkdir -p -- "$source_dir"
curl --fail --location --silent --show-error \
  "https://codeload.github.com/leandrocp/lumis/tar.gz/$LUMIS_REVISION" \
  | tar -xzf - --strip-components=1 -C "$source_dir"

queries="$source_dir/queries/processed"
license="$source_dir/LICENSE"
if [[ ! -d "$queries" || ! -f "$license" ]]; then
  echo "Lumis archive does not contain the expected files" >&2
  exit 1
fi

rm -rf -- "$vendor_dir/queries"
cp -R -- "$queries" "$vendor_dir/queries"
cp -- "$license" "$vendor_dir/LICENSE"

echo "Synchronized Lumis $LUMIS_REVISION"
