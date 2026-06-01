#!/usr/bin/env sh
# Install CortexDB binaries from a checked release tarball.

set -eu

usage() {
  cat <<'EOF'
Usage: scripts/install.sh <archive.tar.gz> [--sha256 <archive.tar.gz.sha256>] [--prefix <dir>] [--dry-run]

Verifies:
  1. the external <archive>.sha256 file;
  2. the package-internal SHA256SUMS file;
  3. bin/cortexdb and bin/cortex-server exist and are executable.

Installs to:
  <prefix>/bin/cortexdb
  <prefix>/bin/cortex-server

Defaults:
  --sha256 <archive>.sha256
  --prefix $HOME/.local
EOF
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

archive=""
sha256_file=""
prefix="${HOME:-}/.local"
dry_run="false"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --sha256)
      [ "$#" -ge 2 ] || fail "--sha256 requires a path"
      sha256_file="$2"
      shift 2
      ;;
    --prefix)
      [ "$#" -ge 2 ] || fail "--prefix requires a directory"
      prefix="$2"
      shift 2
      ;;
    --dry-run)
      dry_run="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      fail "unknown option: $1"
      ;;
    *)
      [ -z "$archive" ] || fail "archive was provided more than once"
      archive="$1"
      shift
      ;;
  esac
done

[ -n "$archive" ] || { usage >&2; exit 2; }
[ -f "$archive" ] || fail "archive not found: $archive"
[ -n "$sha256_file" ] || sha256_file="${archive}.sha256"
[ -f "$sha256_file" ] || fail "checksum file not found: $sha256_file"
[ -n "$prefix" ] || fail "prefix must not be empty"

checksum_tool() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf 'sha256sum'
  elif command -v shasum >/dev/null 2>&1; then
    printf 'shasum'
  else
    fail "sha256sum or shasum is required"
  fi
}

verify_checksum_file() {
  checksum="$1"
  checksum_dir=$(cd "$(dirname "$checksum")" && pwd)
  checksum_name=$(basename "$checksum")
  tool=$(checksum_tool)
  if [ "$tool" = "sha256sum" ]; then
    (cd "$checksum_dir" && sha256sum -c "$checksum_name")
  else
    (cd "$checksum_dir" && shasum -a 256 -c "$checksum_name")
  fi
}

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/cortexdb-install.XXXXXX")
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

verify_checksum_file "$sha256_file"
tar -xzf "$archive" -C "$tmp_dir"

root_count=$(find "$tmp_dir" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
[ "$root_count" = "1" ] || fail "archive must contain exactly one root directory"
package_root=$(find "$tmp_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)

[ -f "$package_root/SHA256SUMS" ] || fail "package missing SHA256SUMS"
verify_checksum_file "$package_root/SHA256SUMS"

cortexdb="$package_root/bin/cortexdb"
server="$package_root/bin/cortex-server"
[ -x "$cortexdb" ] || fail "package missing executable bin/cortexdb"
[ -x "$server" ] || fail "package missing executable bin/cortex-server"

if [ "$dry_run" = "true" ]; then
  printf 'verified %s for prefix %s\n' "$archive" "$prefix"
  exit 0
fi

install_dir="$prefix/bin"
mkdir -p "$install_dir"
install -m 0755 "$cortexdb" "$install_dir/cortexdb"
install -m 0755 "$server" "$install_dir/cortex-server"

printf 'installed cortexdb binaries to %s\n' "$install_dir"
