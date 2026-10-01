#!/usr/bin/env bash
# Publish one workspace crate. A version already on crates.io is success so a
# re-run can continue with the crates that have not been uploaded yet.
# Trusted publishing cannot create a crate; the first version still needs an
# API token, and this script does not turn that 403 into success.
set -euo pipefail

crate=${1:?crate name}
log=$(mktemp)
set +e
cargo publish -p "$crate" --locked >"$log" 2>&1
code=$?
set -e
cat "$log"
if [ "$code" -eq 0 ]; then
  exit 0
fi
if grep -Eq 'already uploaded|already exists' "$log"; then
  echo "$crate is already published"
  exit 0
fi
exit "$code"
