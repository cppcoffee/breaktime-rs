#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 <binary-path> <archive-path> <bundle-id> <version>" >&2
  exit 1
fi

binary_path=$1
archive_path=$2
bundle_id=$3
version=$4

if [[ ! -f "$binary_path" ]]; then
  echo "binary not found: $binary_path" >&2
  exit 1
fi

app_name="${APP_NAME:-BreakTime}"
binary_name=$(basename "$binary_path")
archive_dir=$(dirname "$archive_path")
archive_name=$(basename "$archive_path")
staging_dir=$(mktemp -d "${TMPDIR:-/tmp}/breaktime-app.XXXXXX")
app_bundle="$staging_dir/$app_name.app"
contents_dir="$app_bundle/Contents"
macos_dir="$contents_dir/MacOS"

cleanup() {
  rm -rf "$staging_dir"
}

trap cleanup EXIT

mkdir -p "$archive_dir" "$macos_dir"
archive_dir=$(cd "$archive_dir" && pwd)
archive_path="$archive_dir/$archive_name"
cp "$binary_path" "$macos_dir/$binary_name"
chmod +x "$macos_dir/$binary_name"

cat > "$contents_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>$app_name</string>
  <key>CFBundleExecutable</key>
  <string>$binary_name</string>
  <key>CFBundleIdentifier</key>
  <string>$bundle_id</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$app_name</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$version</string>
  <key>CFBundleVersion</key>
  <string>$version</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
EOF

rm -f "$archive_path"
(
  cd "$staging_dir"
  COPYFILE_DISABLE=1 /usr/bin/zip -q -r "$archive_path" "$app_name.app"
)
printf '%s\n' "$archive_path"
