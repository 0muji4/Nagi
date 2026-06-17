// UniFFI のバインディング生成 CLI。ビルド済みの .so から Kotlin/Swift コードを生成する。
// 例: cargo run -p nagi_ffi --bin uniffi-bindgen -- \
//       generate --library <target>/libnagi_ffi.so --language kotlin --out-dir <出力先>
fn main() {
    uniffi::uniffi_bindgen_main()
}
