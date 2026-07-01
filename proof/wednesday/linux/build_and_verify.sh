#!/usr/bin/env bash
# W1 — Build the Linux Anki installer in a container, then verify it on a clean box.
#
# Usage (run from anywhere; needs a working `docker`, e.g. colima on macOS):
#   proof/wednesday/linux/build_and_verify.sh images   # build the two images
#   proof/wednesday/linux/build_and_verify.sh build     # build the installer (slow)
#   proof/wednesday/linux/build_and_verify.sh verify    # clean-machine smoke test
#   proof/wednesday/linux/build_and_verify.sh all       # images + build + verify (default)
#
# Env:
#   RELEASE=2   optimized/production build (default: unset -> debug, much faster).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"

BUILD_IMG=anki-linux-build:w1
CLEAN_IMG=anki-linux-clean:w1
RELEASE="${RELEASE:-}"

log() { echo -e "\n\033[1;36m== $* ==\033[0m"; }

docker_version() {
  log "docker version"
  docker version
}

build_images() {
  log "Building build image ($BUILD_IMG)"
  docker build -f "$SCRIPT_DIR/Dockerfile.build" -t "$BUILD_IMG" "$SCRIPT_DIR" 2>&1 | tee "$LOG_DIR/image-build.log"
  log "Building clean image ($CLEAN_IMG)"
  docker build -f "$SCRIPT_DIR/Dockerfile.clean" -t "$CLEAN_IMG" "$SCRIPT_DIR" 2>&1 | tee "$LOG_DIR/image-clean.log"
}

build_installer() {
  log "Building the Linux installer inside $BUILD_IMG (this compiles the Rust core; slow)"
  # Only forward RELEASE when non-empty. An *empty* RELEASE would leak into the
  # esbuild TS bundlers as `minify: "" && true` -> "" which esbuild rejects
  # ("minify must be a boolean"); an unset RELEASE is `undefined` which is fine.
  local release_args=()
  [ -n "$RELEASE" ] && release_args=(-e "RELEASE=$RELEASE")
  # ${arr[@]+"${arr[@]}"} = bash-3.2-safe expansion (macOS) that is empty-safe under set -u.
  docker run --rm \
    -v "$REPO_ROOT":/work \
    -v w1-rustup:/opt/rustup \
    -v w1-cargo-registry:/opt/cargo/registry \
    -v w1-uv-cache:/root/.cache/uv \
    ${release_args[@]+"${release_args[@]}"} \
    -w /work/anki \
    "$BUILD_IMG" \
    bash /work/proof/wednesday/linux/scripts/in_container_build.sh 2>&1 | tee "$LOG_DIR/build.log"
}

verify_clean() {
  log "Verifying the installer on a clean box ($CLEAN_IMG)"
  docker run --rm \
    -v "$REPO_ROOT":/src:ro \
    -v "$LOG_DIR":/out \
    "$CLEAN_IMG" \
    bash /src/proof/wednesday/linux/scripts/in_container_smoke.sh 2>&1 | tee "$LOG_DIR/clean-smoke.log"
}

cmd="${1:-all}"
docker_version 2>&1 | tee "$LOG_DIR/docker-version.log" >/dev/null || true
case "$cmd" in
  images)  build_images ;;
  build)   build_installer ;;
  verify)  verify_clean ;;
  all)     build_images; build_installer; verify_clean ;;
  *) echo "unknown command: $cmd (use images|build|verify|all)"; exit 1 ;;
esac
log "done: $cmd  (logs in $LOG_DIR)"
