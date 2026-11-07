#!/bin/sh
# Builds speci-local.app (one universal binary for Apple Silicon and Intel Macs)
# and zips it as speci-local-macos-universal.zip. Run on macOS from the repository root.
set -eu

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
app=target/speci-local.app

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/speci-local" \
    target/aarch64-apple-darwin/release/speci-local \
    target/x86_64-apple-darwin/release/speci-local

# icon: an .iconset with the sizes we can make from the 256 px PNG
set_dir=target/speci-local.iconset
rm -rf "$set_dir" && mkdir -p "$set_dir"
for s in 16 32 128 256; do
    sips -z $s $s assets/icon.png --out "$set_dir/icon_${s}x${s}.png" >/dev/null
done
for s in 16 128; do
    d=$((s * 2))
    sips -z $d $d assets/icon.png --out "$set_dir/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$set_dir" -o "$app/Contents/Resources/speci-local.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>speci-local</string>
    <key>CFBundleDisplayName</key><string>speci-local</string>
    <key>CFBundleIdentifier</key><string>io.github.fcsab.specilocal</string>
    <key>CFBundleExecutable</key><string>speci-local</string>
    <key>CFBundleIconFile</key><string>speci-local</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
</dict>
</plist>
PLIST

# ad-hoc signature (no Apple developer account): Apple Silicon refuses unsigned binaries
codesign --force --deep --sign - "$app"

rm -f speci-local-macos-universal.zip
ditto -c -k --keepParent "$app" speci-local-macos-universal.zip
echo "Built speci-local-macos-universal.zip ($version)"
