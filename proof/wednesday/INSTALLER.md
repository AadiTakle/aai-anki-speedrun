# F11 — Desktop installer (macOS)

Speedrun ships an installable desktop app that runs with **AI off** (AI is out of
scope for the MVP). On this machine (macOS, Apple Silicon) the full Briefcase
pipeline produces a signed app bundle **and** a distributable disk image.

## Reproduce (from `anki/`)

```bash
# 1. Build the pip-installable wheels (also a valid install path: `pip install` them)
just wheels
#    -> out/wheels/anki-26.5-cp310-abi3-macosx_12_0_arm64.whl
#    -> out/wheels/aqt-26.5-py3-none-any.whl

# 2. Build the .app bundle (Briefcase + the mac-template build dep)
./out/pyenv/bin/python qt/tools/build_installer.py --version 26.5 build \
  --aqt_wheel  "$(pwd)/out/wheels/aqt-26.5-py3-none-any.whl" \
  --anki_wheel "$(pwd)/out/wheels/anki-26.5-cp310-abi3-macosx_12_0_arm64.whl" \
  --skip_fcitx
#    -> out/installer/build/anki/macos/app/Anki.app  (ad-hoc signed, arm64)

# 3. Package the installer disk image
./out/pyenv/bin/python qt/tools/build_installer.py --version 26.5 package
#    -> out/installer/dist/anki-26.5-mac-apple.dmg  (~218 MB)
```

## Verified on this build

- `Anki.app` — 676 MB bundle, `codesign -dv` reports `Identifier=net.ankiweb.anki`,
  `arm64`, `flags=0x2(adhoc)`.
- `anki-26.5-mac-apple.dmg` — 218 MB, built via `briefcase package` with ad-hoc signing.

Artifacts live under `anki/out/` (git-ignored build output); regenerate with the
commands above.

## Notes / remaining

- The `qt/installer/mac-template` Briefcase template is a build dependency (cloned
  from `ankitects/briefcase-macOS-app-template@anki`, kept untracked like `ftl/*`).
- The Wednesday spec's headline target is a **Linux** installer verified on a clean
  machine; that path (`build_installer.py` on Linux → AppImage/zip, validated in a
  clean container) is not yet run here. The macOS `.app`/`.dmg` above is the
  installable-desktop proof produced on this host.
- Distribution beyond this machine would need real Developer ID signing +
  notarization (ad-hoc signing runs locally).
