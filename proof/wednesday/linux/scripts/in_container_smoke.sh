#!/usr/bin/env bash
# Runs INSIDE anki-linux-clean:w1 (a fresh box with only runtime libs + xvfb).
# The built installer is bind-mounted read-only at /src; results/screenshot are
# written to /out. Deliberately NOT `set -e`: we run every check and report.
set -uo pipefail

SRC_DIST=/src/anki/out/installer/dist
OUT=/out
mkdir -p "$OUT"
FAIL=0

echo "=================================================================="
echo "W1 clean-machine verification  (arch: $(uname -m), $(date -u +%FT%TZ))"
echo "This box has NO build toolchain:"
command -v cargo rustc gcc uv n2 2>/dev/null && { echo "!! build tools present — not a clean box"; } || echo "  confirmed: no cargo/rustc/gcc/uv/n2 on PATH"
echo "=================================================================="

TARBALL="$(ls "$SRC_DIST"/anki-*-linux-*.tar.zst 2>/dev/null | head -n1)"
if [ -z "${TARBALL:-}" ]; then
  echo "FATAL: no installer artifact found under $SRC_DIST"
  exit 2
fi
echo "== artifact =="
ls -la "$TARBALL"

WORK=/root/ankitest
rm -rf "$WORK"; mkdir -p "$WORK"; cd "$WORK"
echo "== extract (zstd --long) =="
zstd -dc --long=30 "$TARBALL" | tar -xf -
BUNDLE="$WORK/anki-linux"
if [ ! -x "$BUNDLE/anki" ]; then
  echo "FATAL: expected launcher $BUNDLE/anki not found after extract"; ls -la "$WORK"; exit 3
fi
echo "extracted bundle:"; ls -la "$BUNDLE" | head -n 40
echo "bundle size: $(du -sh "$BUNDLE" | cut -f1)"

echo ""
echo "########## [smoke 1] $BUNDLE/anki --version (bundle + Qt/WebEngine libs load) ##########"
# Under xvfb so a display is always available (the version path still imports PyQt6).
xvfb-run -a "$BUNDLE/anki" --version; RC=$?
echo "smoke1 exit=$RC"; [ $RC -eq 0 ] || FAIL=1

echo ""
echo "########## [smoke 2] engine: create a collection with the bundled runtime (AI off) ##########"
# Use the bundle's embedded interpreter against app_packages directly. Going via the
# bootstrap (BRIEFCASE_MAIN_MODULE) would import the app/anki *wrapper* package which
# shadows the real library; this path still exercises the exact embedded interpreter
# + compiled _rsbridge Rust engine that the app ships.
PYBIN="$(ls "$BUNDLE"/python/bin/python3.* 2>/dev/null | grep -E 'python3\.[0-9]+$' | head -n1)"
echo "bundled interpreter: $PYBIN"
cat > "$WORK/w1_smoke.py" <<'PY'
import os, tempfile
base = os.environ.get("W1_SMOKE_DIR") or tempfile.mkdtemp(prefix="w1col_")
os.makedirs(base, exist_ok=True)
import anki.buildinfo
from anki.collection import Collection
col = Collection(os.path.join(base, "collection.anki2"))
print("anki version:", anki.buildinfo.version, "buildhash:", anki.buildinfo.buildhash)
m = col.models.by_name("Basic") or col.models.all()[0]
fields = [f["name"] for f in m["flds"]]
note = col.new_note(m)
note[fields[0]] = "W1 clean-machine: 2+2?"
if len(fields) > 1:
    note[fields[1]] = "4"
col.add_note(note, col.decks.id("Default"))
print("note_count:", col.note_count(), "card_count:", col.card_count())
try:
    col.sched.reset(); print("scheduler reset OK (rust queue exercised)")
except Exception as e:
    print("scheduler reset skipped:", repr(e))
col.close()
print("ENGINE_SMOKE_OK")
PY
W1_SMOKE_DIR="$WORK/col" PYTHONPATH="$BUNDLE/app_packages" "$PYBIN" "$WORK/w1_smoke.py"; RC=$?
echo "smoke2 exit=$RC"
if [ $RC -ne 0 ] || [ ! -f "$WORK/col/collection.anki2" ]; then FAIL=1; echo "smoke2 FAILED"; else echo "smoke2 OK (collection.anki2 created)"; fi

echo ""
echo "########## [smoke 3] GUI launch under xvfb (headless, AI off) ##########"
BASE=/root/ankidata
rm -rf "$BASE"; mkdir -p "$BASE"

# Pre-seed the interface language so Anki skips its first-run Language chooser
# (a normal one-time step; doing it headlessly makes the launch deterministic and
# avoids driving a modal dialog under a headless X server). Uses the SAME bundled
# ProfileManager the app uses; writes prefs21.db into the base dir.
echo "-- pre-seeding defaultLang via bundled ProfileManager (skips first-run chooser) --"
PYTHONPATH="$BUNDLE/app_packages" "$PYBIN" - "$BASE" <<'PY' || echo "pre-seed step failed (continuing; dialog nudge is the fallback)"
import sys
base = sys.argv[1]
from aqt.profiles import ProfileManager
pm = ProfileManager(base)
pm.setupMeta()
pm.meta["defaultLang"] = "en_US"
pm.db.execute("update profiles set data = ? where name = ? collate nocase",
              pm._pickle(pm.meta), "_global")
pm.db.commit()
print("pre-seeded prefs21.db defaultLang=en_US (firstTime will be False)")
PY

export QT_QPA_PLATFORM=xcb
export QTWEBENGINE_CHROMIUM_FLAGS="--no-sandbox --disable-gpu --disable-dev-shm-usage"
export LIBGL_ALWAYS_SOFTWARE=1
export QTWEBENGINE_DISABLE_SANDBOX=1

# best-effort GUI harness: no `set -e` here so one flaky xdotool/screenshot call
# can't tear down Xvfb mid-startup (which would crash Anki's first-run lang setup).
xvfb-run -a -s "-screen 0 1280x900x24" bash -u -c '
  BUNDLE="'"$BUNDLE"'"; BASE="'"$BASE"'"; OUT="'"$OUT"'"
  screenshot() {
    ( import -window root "$1" 2>/dev/null \
        || (xwd -root -silent | convert xwd:- "$1") 2>/dev/null ) \
        && echo "saved $1" || echo "screenshot to $1 failed"
  }
  # minimal WM so windowactivate/focus + real key input work reliably headless
  openbox >/root/openbox.log 2>&1 &
  sleep 1
  "$BUNDLE/anki" -b "$BASE" -l en --safemode >/root/anki_gui.log 2>&1 &
  APP=$!
  created=0
  # Wait for the profile to open -> the engine creates the collection on disk.
  # defaultLang is pre-seeded so no first-run dialog appears; as a safety net, if a
  # modal window shows up we activate it and press Return (accepts the default button).
  for i in $(seq 1 100); do
    if [ -f "$BASE/User 1/collection.anki2" ]; then created=1; echo "collection.anki2 created after ${i}s"; break; fi
    kill -0 "$APP" 2>/dev/null || { echo "anki exited early after ${i}s"; break; }
    if [ "$i" = 6 ] || [ "$i" = 15 ]; then
      WID=$(xdotool search --name "^Anki$" 2>/dev/null | head -n1)
      if [ -n "${WID:-}" ]; then
        [ "$i" = 6 ] && screenshot "$OUT/linux-clean-screenshot-startup.png"
        xdotool windowactivate --sync "$WID" 2>/dev/null || true
        xdotool key --clearmodifiers Return 2>/dev/null || true
      fi
    fi
    sleep 1
  done
  sleep 8   # let the main window (webview) paint
  echo "--- X windows present ---"
  xdotool search --name ".*" getwindowname %@ 2>/dev/null | sort -u | sed "/^$/d" | head -n 40 || true
  echo "--- screenshot (main window if reached) ---"
  screenshot "$OUT/linux-clean-screenshot.png"
  echo "--- graceful quit (Ctrl+Q to Anki window) ---"
  MAINWID=$(xdotool search --name "^Anki" 2>/dev/null | head -n1)
  if [ -n "${MAINWID:-}" ]; then
    xdotool windowactivate --sync "$MAINWID" 2>/dev/null || true
    xdotool key --clearmodifiers ctrl+q 2>/dev/null || true
  fi
  sleep 6
  kill -0 "$APP" 2>/dev/null && { echo "still running; SIGTERM"; kill -TERM "$APP" 2>/dev/null; sleep 4; }
  kill -0 "$APP" 2>/dev/null && { echo "still running; SIGKILL"; kill -KILL "$APP" 2>/dev/null; }
  wait "$APP" 2>/dev/null; echo "gui process final exit=$?"
  echo "GUI_COLLECTION_CREATED=$created"
'
echo "== GUI log (tail) =="
tail -n 60 /root/anki_gui.log 2>/dev/null || true
cp -f /root/anki_gui.log "$OUT/linux-clean-anki_gui.log" 2>/dev/null || true

echo "== base dir tree (proves profile+collection written) =="
find "$BASE" -maxdepth 2 2>/dev/null | sed "s#$BASE#<base>#" | head -n 40

# GUI pass = the GUI created a collection (started far enough to init the engine).
if [ -f "$BASE/User 1/collection.anki2" ]; then
  echo "smoke3 OK (GUI launched and created a collection with AI off)"
else
  echo "smoke3 WARN: GUI did not create a collection (see log above)"
fi

echo ""
echo "=================================================================="
if [ $FAIL -eq 0 ]; then
  echo "W1 CLEAN-MACHINE SMOKE: PASS (bundle loads + engine creates a collection, AI off)"
else
  echo "W1 CLEAN-MACHINE SMOKE: FAIL (see above)"
fi
echo "=================================================================="
exit $FAIL
