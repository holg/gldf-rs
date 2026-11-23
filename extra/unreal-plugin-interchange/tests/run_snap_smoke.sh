#!/bin/bash
# Editor smoke test for the luminaire surface snapping (SnapToSurface /
# PlaceFromRay). Launches UnrealEditor-Cmd on the GldfArchTest host project,
# runs tests/snap_smoke.py in the Interior map, prints the PASS/FAIL table.
#
#   extra/unreal-plugin-interchange/tests/run_snap_smoke.sh
#
# Takes ~1 min (editor startup). Requires the plugin to be built for the host
# project (Build.sh UnrealEditor Mac Development -Project=...). A Metal
# toolchain must be installed for the Xcode in use
# (`xcodebuild -downloadComponent MetalToolchain`), or the editor exits at
# startup with "cannot execute tool 'metal'".
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GLDF_RS="$(cd "$ROOT/../.." && pwd)"
PROJECT="$GLDF_RS/tmp/GldfArchTest/GldfArchTest.uproject"
UE_CMD="/Users/Shared/Epic Games/UE_5.7/Engine/Binaries/Mac/UnrealEditor-Cmd"
export GLDF_RS_WORKSPACE="$GLDF_RS"
export GLDF_SEARCH_WORKSPACE="${GLDF_SEARCH_WORKSPACE:-$GLDF_RS/../gldf-search}"
export GLDF_SNAP_SMOKE_RESULT="$GLDF_RS/tmp/snap_smoke_result.txt"
LOG="$GLDF_RS/tmp/snap_smoke_editor.log"
rm -f "$GLDF_SNAP_SMOKE_RESULT"
"$UE_CMD" "$PROJECT" -ExecutePythonScript="$ROOT/tests/snap_smoke.py" \
  -unattended -nosplash -NoSound -stdout -FullStdOutLogOutput > "$LOG" 2>&1 || true
if [ -f "$GLDF_SNAP_SMOKE_RESULT" ]; then
  cat "$GLDF_SNAP_SMOKE_RESULT"
  grep -q FAIL "$GLDF_SNAP_SMOKE_RESULT" && { echo "SNAP SMOKE: FAILURES"; exit 1; }
  echo "SNAP SMOKE: all passed"
else
  echo "SNAP SMOKE: no result — see $LOG"; grep -n -i 'SNAPSMOKE\|LogPython: Error\|Message dialog' "$LOG" | tail -20; exit 1
fi
