#!/bin/bash
set -euo pipefail

# Checks that install_all.sh starts clean and stops on the first failure without touching
# seqa_core/data/ or S3. wget and aws are replaced by stubs, so nothing is downloaded or uploaded.

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
SCRIPTS_DIR="$(dirname "${TESTS_DIR}")"
DATA_DIR="${SCRIPTS_DIR}/../seqa_core/data"
SCRATCH_DIR="${TESTS_DIR}/scratch_halt_on_failure"

cleanup() {
  rm -rf "${SCRATCH_DIR}"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1"
  exit 1
}

cleanup
mkdir -p "${SCRATCH_DIR}/bin" "${SCRATCH_DIR}/work"

AWS_CALLS="${SCRATCH_DIR}/aws_calls"
cat > "${SCRATCH_DIR}/bin/wget" <<'STUB'
#!/bin/bash
echo "wget stub: simulated download failure" >&2
exit 1
STUB
cat > "${SCRATCH_DIR}/bin/aws" <<STUB
#!/bin/bash
echo "\$@" >> "${AWS_CALLS}"
STUB
chmod +x "${SCRATCH_DIR}/bin/wget" "${SCRATCH_DIR}/bin/aws"

data_checksum() {
  (cd "${DATA_DIR}" && sha256sum -- *)
}

# Case 1: a failed download halts the run before copying or uploading.
data_before="$(data_checksum)"
cd "${SCRATCH_DIR}/work"
if PATH="${SCRATCH_DIR}/bin:${PATH}" "${SCRIPTS_DIR}/install_all.sh" > "${SCRATCH_DIR}/case1.log" 2>&1; then
  fail "install_all.sh exited 0 after a failed download"
fi
[[ ! -e "${AWS_CALLS}" ]] || fail "aws was called after a failed download"
[[ "$(data_checksum)" == "${data_before}" ]] || fail "seqa_core/data/ changed after a failed download"
echo "PASS: failed download halts the run"

# Case 2: output from an earlier run is removed before starting.
mkdir -p "${SCRATCH_DIR}/work/hg38"
touch "${SCRATCH_DIR}/work/hg38/stale.csv" "${SCRATCH_DIR}/work/mane.bb"
if PATH="${SCRATCH_DIR}/bin:${PATH}" "${SCRIPTS_DIR}/install_all.sh" > "${SCRATCH_DIR}/case2.log" 2>&1; then
  fail "install_all.sh exited 0 after a failed download"
fi
[[ ! -e "${SCRATCH_DIR}/work/hg38/stale.csv" ]] || fail "stale hg38 output was not removed"
[[ ! -e "${SCRATCH_DIR}/work/mane.bb" ]] || fail "stale mane.bb was not removed"
echo "PASS: earlier output is removed"
