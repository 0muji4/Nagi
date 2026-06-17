#!/bin/sh
# Rust の UniFFI 公開層(nagi_ffi)を Android 向けにビルドし、生成物をアプリへ配置する。
#   1) ホスト向けに一度ビルド → uniffi-bindgen で Kotlin バインディングを生成
#   2) cargo-ndk で各 ABI の .so をビルドし jniLibs へ配置
#
# 前提（初回のみセットアップ。詳細は android/ の README / CONTRIBUTING）:
#   - Android NDK を導入し、ANDROID_NDK_HOME（または ANDROID_SDK_ROOT/ndk）を設定
#   - rustup target add aarch64-linux-android x86_64-linux-android
#   - cargo install cargo-ndk
#
# Android Studio で android/ の雛形（applicationId = io.github.muji4.nagi）を作った後に実行する。
set -eu

repo=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
rust_dir="$repo/rust"
app_dir="$repo/android/app"
abis="arm64-v8a x86_64"          # Apple Silicon のエミュレータ＋実機をカバー
kotlin_out="$app_dir/build/generated/uniffi"
jni_out="$app_dir/src/main/jniLibs"

error() { printf '\033[1;31mError: %s\033[0m\n' "$1" >&2; }

[ -d "$app_dir" ] || { error "$app_dir が無い。先に Android Studio で android/ の雛形を作ること。"; exit 1; }

cd "$rust_dir"

# 1) ホストビルド（uniffi-bindgen がメタデータを読むため）→ Kotlin 生成
echo "==> host build + Kotlin バインディング生成"
cargo build -p nagi_ffi --release
host_lib=$(ls target/release/libnagi_ffi.dylib target/release/libnagi_ffi.so 2>/dev/null | head -n1)
[ -n "$host_lib" ] || { error "ホストの libnagi_ffi が見つからない。"; exit 1; }
rm -rf "$kotlin_out"
mkdir -p "$kotlin_out"
cargo run -q -p nagi_ffi --bin uniffi-bindgen -- \
  generate --library "$host_lib" --language kotlin --out-dir "$kotlin_out"

# 2) 各 ABI の .so をビルドして jniLibs へ配置（cargo-ndk が <jniLibs>/<abi>/ に置く）
echo "==> cargo-ndk で .so をビルド ($abis)"
ndk_targets=""
for abi in $abis; do ndk_targets="$ndk_targets -t $abi"; done
# shellcheck disable=SC2086
cargo ndk $ndk_targets -o "$jni_out" build --release -p nagi_ffi

echo "Done:"
echo "  .so     -> $jni_out/<abi>/libnagi_ffi.so"
echo "  Kotlin  -> $kotlin_out/uniffi/nagi_ffi/nagi_ffi.kt"
