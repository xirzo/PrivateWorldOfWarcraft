#!/usr/bin/env bash
set -euo pipefail

# Build a localization patch zip for one locale, compute its SHA-256 and
# write the result (URL + checksum) into installer/locales.json.
#
# Usage:
#   scripts/build-locale-patch.sh <locale> <source_root> [output_dir] [url]
#
# <source_root> must contain Data/<locale>/ — the folder to package. The
# output zip contains Data/<locale>/... entries, matching the layout the
# installer merges into the client. [url] overrides the download URL written
# into locales.json (use a Google Drive share link, or omit it to default to
# the `patches` GitHub Release). After building, upload the zip to that URL.
#
# Example:
#   ./installer/scripts/build-locale-patch.sh ruRU ~/wow_client_ruRU dist \
#     "https://drive.google.com/file/d/AbC123xyz/view?usp=sharing"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
locales_json="$repo_root/installer/locales.json"

locale="${1:-}"
source_root="${2:-}"
output_dir="${3:-$repo_root/dist}"
url_override="${4:-}"

if [[ -z "$locale" || -z "$source_root" ]]; then
  echo "usage: $0 <locale> <source_root> [output_dir] [url]" >&2
  exit 2
fi

locale_dir="$source_root/Data/$locale"
if [[ ! -d "$locale_dir" ]]; then
  echo "error: no such directory: $locale_dir" >&2
  exit 1
fi

mkdir -p "$output_dir"
release_tag="patches"
zip_path="$output_dir/wow-$locale-3.3.5a.zip"
rm -f "$zip_path"

(
  cd "$source_root"
  zip -qr "$zip_path" "Data/$locale"
)

sha256="$(sha256sum "$zip_path" | awk '{print $1}')"
if [[ -n "$url_override" ]]; then
  url="$url_override"
else
  url="https://github.com/xirzo/PrivateWorlfOfWarcraft/releases/download/$release_tag/$(basename "$zip_path")"
fi

python3 - "$locales_json" "$locale" "$url" "$sha256" <<'PY'
import json
import sys

path, locale, url, sha256 = sys.argv[1:]
with open(path, encoding="utf-8") as f:
    reg = json.load(f)
if locale not in reg:
    print(f"error: locale {locale!r} not in {path}", file=sys.stderr)
    sys.exit(1)
reg[locale]["url"] = url
reg[locale]["sha256"] = sha256
with open(path, "w", encoding="utf-8") as f:
    json.dump(reg, f, indent=2, ensure_ascii=False)
    f.write("\n")
PY

echo "Patch:      $zip_path"
echo "SHA-256:    $sha256"
echo "URL:        $url"
echo
if [[ -n "$url_override" ]]; then
  echo "Upload '$zip_path' to the URL above and keep the link in locales.json in sync."
else
  echo "Upload it to the $release_tag GitHub Release:"
  echo "  gh release upload '$release_tag' '$zip_path'"
fi
