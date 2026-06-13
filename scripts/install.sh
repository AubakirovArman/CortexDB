#!/usr/bin/env sh
# Install CortexDB binaries from a checked release tarball.

set -eu

usage() {
  cat <<'EOF'
Usage: scripts/install.sh <archive.tar.gz-or-url> [--sha256 <archive.tar.gz.sha256-or-url>] [--prefix <dir>] [--dry-run]

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

Examples:
  scripts/install.sh ./cortexdb-v0.2.0-beta.2-linux-x86_64.tar.gz
  scripts/install.sh https://example.com/cortexdb-v0.2.0-beta.2-linux-x86_64.tar.gz --prefix /usr/local
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

checksum_tool() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf 'sha256sum'
  elif command -v shasum >/dev/null 2>&1; then
    printf 'shasum'
  else
    fail "sha256sum or shasum is required"
  fi
}

is_url() {
  case "$1" in
    http://*|https://*|file://*) return 0 ;;
    *) return 1 ;;
  esac
}

url_basename() {
  without_query=${1%%\?*}
  base=${without_query##*/}
  [ -n "$base" ] || base="cortexdb-release.tar.gz"
  printf '%s' "$base"
}

download_file() {
  source="$1"
  destination="$2"
  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --silent --show-error --output "$destination" "$source"
  elif command -v wget >/dev/null 2>&1; then
    wget --quiet --output-document="$destination" "$source"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$source" "$destination" <<'PY'
import sys
import urllib.request

urllib.request.urlretrieve(sys.argv[1], sys.argv[2])
PY
  else
    fail "curl, wget, or python3 is required to download release artifacts"
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

[ -n "$archive" ] || { usage >&2; exit 2; }
[ -n "$prefix" ] || fail "prefix must not be empty"

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/cortexdb-install.XXXXXX")
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

if is_url "$archive"; then
  archive_name=$(url_basename "$archive")
  archive_url="$archive"
  archive="$tmp_dir/$archive_name"
  download_file "$archive_url" "$archive"
elif [ ! -f "$archive" ]; then
  fail "archive not found: $archive"
fi

[ -n "$sha256_file" ] || sha256_file="${archive_url:-$archive}.sha256"
if is_url "$sha256_file"; then
  checksum_name=$(url_basename "$sha256_file")
  sha256_url="$sha256_file"
  sha256_file="$tmp_dir/$checksum_name"
  download_file "$sha256_url" "$sha256_file"
elif [ ! -f "$sha256_file" ]; then
  fail "checksum file not found: $sha256_file"
fi

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
  printf 'next steps: rerun without --dry-run, then run %s/bin/cortexdb --version\n' "$prefix"
  exit 0
fi

install_dir="$prefix/bin"
mkdir -p "$install_dir"
install -m 0755 "$cortexdb" "$install_dir/cortexdb"
install -m 0755 "$server" "$install_dir/cortex-server"

printf 'installed cortexdb binaries to %s\n' "$install_dir"
printf 'next steps:\n'
printf '  1. Ensure %s is on PATH.\n' "$install_dir"
printf '  2. Run %s/cortexdb --version.\n' "$install_dir"
printf '  3. Run %s/cortexdb validate ./data before using an existing database.\n' "$install_dir"
printf '  4. Start the server with CORTEXDB_AUTH_TOKEN=change-me %s/cortex-server ./data 127.0.0.1:8181.\n' "$install_dir"
