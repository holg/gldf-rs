#!/bin/bash
# Generic cross-build driver for macOS hosts.
#
# ALL project specifics (which packages/bins to build, profile,
# RUSTFLAGS, per-target toolchains, dist dir) live in
# scripts/cross-build.toml — edit that, not this script. The script only:
#   1. reads the config,
#   2. installs the rustup target if missing,
#   3. exports the per-target CC/AR/LINKER (+ optional env.*) from the
#      config, with sccache/incremental disabled for cross builds,
#   4. runs `cargo build` per configured "<package>:<bin>",
#   5. collects the binaries into <dist_dir>/<target-triple>/.
#
# Usage:
#   scripts/cross_build_on_mac.sh                       # list targets
#   scripts/cross_build_on_mac.sh <target-triple>       # build one target
#   scripts/cross_build_on_mac.sh all                   # build every target
#   scripts/cross_build_on_mac.sh <triple> --only <package:bin>
#                                                       # build a subset
#   CROSS_BUILD_CONFIG=path/to.toml scripts/cross_build_on_mac.sh …
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG="${CROSS_BUILD_CONFIG:-$REPO_ROOT/scripts/cross-build.toml}"
cd "$REPO_ROOT"

if [[ ! -f "$CONFIG" ]]; then
  echo "Error: config not found: $CONFIG" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Minimal TOML subset reader (see the header comment in cross-build.toml).
# ---------------------------------------------------------------------------

# toml_get <section> <key> → value ("" when absent)
toml_get() {
  awk -v section="[$1]" -v key="$2" '
    $0 == section { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section {
      line = $0
      sub(/^[ \t]+/, "", line)
      if (index(line, key " ") == 1 || index(line, key "=") == 1) {
        sub(/^[^=]*=[ \t]*/, "", line)
        gsub(/^"|"[ \t]*$/, "", line)
        print line
        exit
      }
    }
  ' "$CONFIG"
}

# toml_list <section> <key> → one array item per line
toml_list() {
  awk -v section="[$1]" -v key="$2" '
    $0 == section { in_section = 1; next }
    /^\[/ { in_section = 0; in_array = 0 }
    in_section && !in_array {
      line = $0
      sub(/^[ \t]+/, "", line)
      if (index(line, key " ") == 1 || index(line, key "=") == 1) {
        in_array = 1
        next
      }
    }
    in_array {
      if ($0 ~ /\]/) { in_array = 0; next }
      line = $0
      gsub(/[ \t",]/, "", line)
      if (line != "") print line
    }
  ' "$CONFIG"
}

# toml_env <target-triple> → "KEY=value" per line from env.* keys
toml_env() {
  awk -v section="[target.$1]" '
    $0 == section { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section {
      line = $0
      sub(/^[ \t]+/, "", line)
      if (index(line, "env.") == 1) {
        sub(/^env\./, "", line)
        key = line
        sub(/[ \t]*=.*$/, "", key)
        sub(/^[^=]*=[ \t]*/, "", line)
        gsub(/^"|"[ \t]*$/, "", line)
        print key "=" line
      }
    }
  ' "$CONFIG"
}

# all [target.*] triples in the config
toml_targets() {
  awk '/^\[target\./ { gsub(/^\[target\.|\]$/, ""); print }' "$CONFIG"
}

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

TARGET_ARG="${1:-}"
ONLY=""
if [[ "${2:-}" == "--only" ]]; then
  ONLY="${3:?--only needs a <package:bin> argument}"
fi

if [[ -z "$TARGET_ARG" ]]; then
  echo "Configured targets in $CONFIG:"
  toml_targets | sed 's/^/  - /'
  echo "Usage: $0 <target-triple|all> [--only <package:bin>]"
  exit 0
fi

if [[ "$TARGET_ARG" == "all" ]]; then
  TARGETS="$(toml_targets)"
else
  TARGETS="$TARGET_ARG"
fi

PROJECT_NAME="$(toml_get project name)"
PROFILE="$(toml_get project profile)"
DEFAULT_RUSTFLAGS="$(toml_get project rustflags)"
DIST_DIR="$(toml_get project dist_dir)"
BUILDS="$(toml_list project builds)"

if [[ -z "$BUILDS" ]]; then
  echo "Error: no builds configured under [project] builds in $CONFIG" >&2
  exit 1
fi
if [[ -n "$ONLY" ]]; then
  BUILDS="$(printf '%s\n' "$BUILDS" | grep -Fx "$ONLY" || true)"
  if [[ -z "$BUILDS" ]]; then
    echo "Error: --only $ONLY does not match any configured build" >&2
    exit 1
  fi
fi

check_rustup_target() {
  local target="$1"
  if ! rustup target list --installed | grep -qx "$target"; then
    echo "Installing rustup target $target …"
    rustup target add "$target"
  fi
}

# ---------------------------------------------------------------------------
# Per-target build
# ---------------------------------------------------------------------------

build_target() {
  local triple="$1"

  if ! toml_targets | grep -qx "$triple"; then
    echo "Error: target $triple is not configured in $CONFIG" >&2
    echo "Configured targets:" >&2
    toml_targets | sed 's/^/  - /' >&2
    return 1
  fi

  local section="target.$triple"
  local cc ar linker hint target_rustflags
  cc="$(toml_get "$section" cc)"
  ar="$(toml_get "$section" ar)"
  linker="$(toml_get "$section" linker)"
  hint="$(toml_get "$section" install_hint)"
  target_rustflags="$(toml_get "$section" rustflags)"

  # Toolchain sanity check before touching cargo.
  for tool in $cc $ar $linker; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      echo "Error: $tool not found in PATH (needed for $triple)" >&2
      [[ -n "$hint" ]] && echo "Install with: $hint" >&2
      return 1
    fi
  done

  check_rustup_target "$triple"

  echo ""
  echo "=== $PROJECT_NAME → $triple ==="

  # Build in a subshell so per-target env never leaks into the next one.
  (
    # Env-var name mangling: cc-rs wants lowercase-with-underscores
    # suffixes, cargo wants SCREAMING_SNAKE triples.
    local lower upper
    lower="${triple//-/_}"
    upper="$(echo "$lower" | tr '[:lower:]' '[:upper:]')"

    # Cross builds: no sccache/wrapper, no incremental, no macOS flags
    # leaking into foreign compilers.
    export SCCACHE_DISABLE=1
    export RUSTC_WRAPPER=""
    export CARGO_INCREMENTAL=0
    if [[ "$triple" != *apple* ]]; then
      unset MACOSX_DEPLOYMENT_TARGET
      export "CFLAGS_${lower}="
      export "CXXFLAGS_${lower}="
    fi

    if [[ -n "$cc" ]]; then
      export "CC_${lower}=$cc"
      unset CC
    fi
    if [[ -n "$ar" ]]; then
      export "AR_${lower}=$ar"
      unset AR
    fi
    if [[ -n "$linker" ]]; then
      export "CARGO_TARGET_${upper}_LINKER=$linker"
    fi

    local rustflags="${target_rustflags:-$DEFAULT_RUSTFLAGS}"
    if [[ -n "$rustflags" ]]; then
      export RUSTFLAGS="$rustflags"
    fi

    # Extra per-target env from the config (env.KEY = "value").
    while IFS= read -r pair; do
      [[ -z "$pair" ]] && continue
      export "${pair?}"
      echo "  env: $pair"
    done < <(toml_env "$triple")

    local profile_args=()
    local profile_dir="$PROFILE"
    if [[ "$PROFILE" == "release" ]]; then
      profile_args=(--release)
    elif [[ -n "$PROFILE" ]]; then
      profile_args=(--profile "$PROFILE")
    else
      profile_dir="debug"
    fi

    local exe_suffix=""
    [[ "$triple" == *windows* ]] && exe_suffix=".exe"

    local out_dir="$DIST_DIR/$triple"
    mkdir -p "$out_dir"

    while IFS= read -r build; do
      [[ -z "$build" ]] && continue
      local package="${build%%:*}"
      local bin="${build##*:}"
      # Optional per-bin cargo args ([build.<bin>] cargo_args in config).
      local extra_args
      extra_args="$(toml_get "build.$bin" cargo_args)"
      # shellcheck disable=SC2086  # word-splitting is the point here
      echo ""
      echo "--- cargo build -p $package --bin $bin --target $triple ${profile_args[*]} $extra_args ---"
      cargo build -p "$package" --bin "$bin" --target "$triple" "${profile_args[@]}" $extra_args

      local built="target/$triple/$profile_dir/$bin$exe_suffix"
      if [[ -f "$built" ]]; then
        cp "$built" "$out_dir/"
        echo "  → $out_dir/$bin$exe_suffix ($(du -h "$built" | cut -f1))"
      else
        echo "Error: expected binary not found: $built" >&2
        exit 1
      fi
    done <<< "$BUILDS"
  )
}

FAILED=""
for triple in $TARGETS; do
  if ! build_target "$triple"; then
    FAILED="$FAILED $triple"
  fi
done

echo ""
if [[ -n "$FAILED" ]]; then
  echo "Done with failures:$FAILED" >&2
  exit 1
fi
echo "All builds completed. Artifacts under $DIST_DIR/<target>/"
