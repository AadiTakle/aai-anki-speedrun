#!/usr/bin/env bash
# Runs INSIDE anki-linux-build:w1 with the worktree bind-mounted at /work.
# Produces the Linux Anki installer under out/installer/dist/.
#
# Env:
#   RELEASE   (optional) "1" or "2" for an optimized/production build.
#             Default unset -> debug build (much faster; fine for the smoke test).
set -euo pipefail

cd /work/anki

export CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
export PATH=/opt/cargo/bin:$PATH
# Keep the whole build off the network only if OFFLINE_BUILD is set (default: online).

# Defensive: an *empty* RELEASE breaks the esbuild TS bundlers (`minify: "" && true`
# -> "" which esbuild rejects). Treat empty as unset so `minify` becomes undefined.
if [ -z "${RELEASE:-}" ]; then unset RELEASE; fi

echo "=================================================================="
echo "W1 Linux installer build"
echo "  arch:        $(uname -m)"
echo "  .version:    $(cat .version)"
echo "  RELEASE:     ${RELEASE:-<unset/debug>}"
echo "  nproc:       $(nproc)"
echo "  date:        $(date -u +%FT%TZ)"
echo "=================================================================="

echo "== [1/4] Build wheels + pyenv (compiles the Rust core; slow first time) =="
# Building pylib+qt guarantees out/pylib is importable (build_installer imports
# anki.lang) and that out/pyenv exists with briefcase; wheels produces the
# distributable anki/aqt wheels the installer bundles.
./ninja pylib qt wheels

echo "== wheels produced =="
ls -la out/wheels/*.whl

echo "== verify pyenv has briefcase + anki is importable =="
out/pyenv/bin/python -c "import briefcase; print('briefcase', briefcase.__version__)"
out/pyenv/bin/python -c "import sys; sys.path[:0]=['pylib','out/pylib']; from anki import lang; print('anki.lang import OK')"

AQT_WHEEL="$(ls "$(pwd)"/out/wheels/aqt-*.whl | head -n1)"
ANKI_WHEEL="$(ls "$(pwd)"/out/wheels/anki-*.whl | head -n1)"
# Derive the installer version from the actual aqt wheel (PEP440-normalized).
VER="$(basename "$AQT_WHEEL" | sed -E 's/^aqt-([^-]+)-.*/\1/')"
echo "  aqt wheel:  $AQT_WHEEL"
echo "  anki wheel: $ANKI_WHEEL"
echo "  version:    $VER"

# Briefcase unpacks the python-build-standalone support package, which contains a
# large tree of relative symlinks (python/share/terminfo/*). Python 3.13's tarfile
# data-filter runs os.path.realpath() on every member, which raises ELOOP on the
# colima virtiofs bind-mount. Redirect out/installer onto a real container-local
# fs (overlay/ext4) for the Briefcase build, then copy the artifact back to the mount.
INSTALLER_LOCAL="${INSTALLER_LOCAL:-/tmp/anki-installer}"
echo "== redirect out/installer -> $INSTALLER_LOCAL (real fs; avoids virtiofs symlink ELOOP) =="
rm -rf out/installer "$INSTALLER_LOCAL"
mkdir -p "$INSTALLER_LOCAL"
ln -sfn "$INSTALLER_LOCAL" out/installer

echo "== [2/4] build_installer.py build (Briefcase + linux-template) =="
out/pyenv/bin/python qt/tools/build_installer.py --version "$VER" build \
  --aqt_wheel "$AQT_WHEEL" --anki_wheel "$ANKI_WHEEL"

echo "== [3/4] build_installer.py package (tar.zst) =="
out/pyenv/bin/python qt/tools/build_installer.py --version "$VER" package

echo "== [4/4] copy artifact back onto the bind-mount =="
ART_LOCAL="$(ls "$INSTALLER_LOCAL"/dist/anki-*-linux-*.tar.zst | head -n1)"
rm -f out/installer                       # drop the symlink (keeps the built tree in $INSTALLER_LOCAL)
mkdir -p out/installer/dist out/installer/logs
cp -f "$ART_LOCAL" out/installer/dist/
cp -f "$INSTALLER_LOCAL"/logs/* out/installer/logs/ 2>/dev/null || true
ls -la out/installer/dist/
echo "ARTIFACT: $(ls "$(pwd)"/out/installer/dist/anki-*-linux-*.tar.zst)"
echo "== BUILD DONE =="
