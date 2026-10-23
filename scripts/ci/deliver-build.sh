#!/usr/bin/env bash
# Where CI builds go. Artifacts and logs of a public repository are public, so:
#   - with BUILDS_R2_* set, the file is copied to that PRIVATE R2 bucket (any environment);
#   - without them, dev builds fall back to a 1-day workflow artifact (the workflow step does the upload);
#   - prod builds are never left as artifacts: without a private bucket the job stops.
# Usage: deliver-build.sh --check <env>            (fails early when prod has no private destination)
#        deliver-build.sh <env> <file> <name>      (uploads when a private bucket is configured)
set -euo pipefail

have_bucket() {
  [ -n "${BUILDS_R2_BUCKET:-}" ] && [ -n "${BUILDS_R2_ACCOUNT_ID:-}" ] \
    && [ -n "${BUILDS_R2_ACCESS_KEY_ID:-}" ] && [ -n "${BUILDS_R2_SECRET_ACCESS_KEY:-}" ]
}

if [ "${1:-}" = "--check" ]; then
  env_name="${2:?environment}"
  if [ "$env_name" = prod ] && ! have_bucket; then
    echo "prod builds need a private bucket: set BUILDS_R2_ACCOUNT_ID, BUILDS_R2_BUCKET, BUILDS_R2_ACCESS_KEY_ID and BUILDS_R2_SECRET_ACCESS_KEY in the prod environment."
    exit 1
  fi
  exit 0
fi

env_name="${1:?environment}"; file="${2:?file}"; name="${3:?name}"
[ -f "$file" ] || { echo "Build output not found: $file"; exit 1; }
if ! have_bucket; then
  echo "No private bucket configured; the workflow keeps a 1-day artifact (dev only)."
  exit 0
fi
AWS_ACCESS_KEY_ID="$BUILDS_R2_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="$BUILDS_R2_SECRET_ACCESS_KEY" AWS_DEFAULT_REGION=auto \
  aws s3 cp "$file" "s3://$BUILDS_R2_BUCKET/$env_name/$name" \
  --endpoint-url "https://$BUILDS_R2_ACCOUNT_ID.r2.cloudflarestorage.com" --only-show-errors
echo "Uploaded to the private bucket as $env_name/$name"
