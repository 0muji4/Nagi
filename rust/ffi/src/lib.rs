//! Nagi の UniFFI 公開層（感覚の層への橋）。
//!
//! 純粋な規則コア（`nagi_core`、std のみ・FFI 非依存）を、各プラットフォームの
//! ネイティブ UI から呼べるように公開する（DD Q1 の規則と感覚の分離の「境界」）。
//! ここだけが `uniffi` に依存し、コアは依存ゼロのまま保つ。
//!
//! 公開の方針:
//! - データ型（`RenderState`, `Color`, `Palette`）と列挙（`Phase`, `SessionEnd`,
//!   `AbortReason`）は、コアの型に直接 UniFFI の印を付けず、UniFFI 用のミラー型を
//!   定義して `From` で変換する（コアを FFI 非依存に保つため）。
//! - `NagiTimer` は状態を持つため UniFFI の Object として公開する。UniFFI のメソッドは
//!   `&self` を取るので、内部を `Mutex` で包んで可変にする（`tick` 等は状態を書き換える）。
//!
//! このクレートは `uniffi`（crates.io）を要するため、ネットワークの無い環境では
//! ビルドできない。ビルドと Kotlin/Swift バインディング生成は CI かローカルマシンで行う。
//! バインディング生成（library モード）の例:
//!   cargo run -p nagi_ffi --bin uniffi-bindgen -- \
//!     generate --library <target>/libnagi_ffi.so --language kotlin --out-dir <出力先>

use std::sync::{Arc, Mutex};

uniffi::setup_scaffolding!();

/// 凪の刻の局面。`nagi_core::Phase` のミラー。
#[derive(uniffi::Enum)]
pub enum Phase {
    Idle,
    Running,
    Completed,
}

/// 試行が中断した要因。`nagi_core::AbortReason` のミラー。
#[derive(uniffi::Enum)]
pub enum AbortReason {
    Disturbance,
    Left,
}

/// 一回の試行の結果。`nagi_core::SessionEnd` のミラー。
#[derive(uniffi::Enum)]
pub enum SessionEnd {
    Completed,
    Aborted { reason: AbortReason },
}

/// 感覚の層が毎フレーム受け取る状態。`nagi_core::RenderState` のミラー。
#[derive(uniffi::Record)]
pub struct RenderState {
    pub phase: Phase,
    pub remaining_secs: f64,
    pub disturbance: f64,
    pub ended: Option<SessionEnd>,
}

/// グラデーションの一色（各成分 0.0..=1.0）。`nagi_core::palette::Color` のミラー。
#[derive(uniffi::Record)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// グラデーションの色の並び。`nagi_core::palette::Palette` のミラー。
#[derive(uniffi::Record)]
pub struct Palette {
    pub stops: Vec<Color>,
}

// --- コア型 → ミラー型の変換 ---

impl From<nagi_core::Phase> for Phase {
    fn from(p: nagi_core::Phase) -> Self {
        match p {
            nagi_core::Phase::Idle => Phase::Idle,
            nagi_core::Phase::Running => Phase::Running,
            nagi_core::Phase::Completed => Phase::Completed,
        }
    }
}

impl From<nagi_core::AbortReason> for AbortReason {
    fn from(r: nagi_core::AbortReason) -> Self {
        match r {
            nagi_core::AbortReason::Disturbance => AbortReason::Disturbance,
            nagi_core::AbortReason::Left => AbortReason::Left,
        }
    }
}

impl From<nagi_core::SessionEnd> for SessionEnd {
    fn from(e: nagi_core::SessionEnd) -> Self {
        match e {
            nagi_core::SessionEnd::Completed => SessionEnd::Completed,
            nagi_core::SessionEnd::Aborted(r) => SessionEnd::Aborted { reason: r.into() },
        }
    }
}

impl From<nagi_core::RenderState> for RenderState {
    fn from(s: nagi_core::RenderState) -> Self {
        RenderState {
            phase: s.phase.into(),
            remaining_secs: s.remaining_secs,
            disturbance: s.disturbance,
            ended: s.ended.map(Into::into),
        }
    }
}

impl From<nagi_core::palette::Color> for Color {
    fn from(c: nagi_core::palette::Color) -> Self {
        Color {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

impl From<nagi_core::palette::Palette> for Palette {
    fn from(p: nagi_core::palette::Palette) -> Self {
        Palette {
            stops: p.stops.into_iter().map(Into::into).collect(),
        }
    }
}

/// 凪の刻の状態機械。感覚の層から呼ぶ Object。
/// 純粋コアを `Mutex` で包み、`&self` のメソッドから安全に可変操作する。
#[derive(uniffi::Object)]
pub struct NagiTimer {
    inner: Mutex<nagi_core::NagiTimer>,
}

#[uniffi::export]
impl NagiTimer {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(nagi_core::NagiTimer::new()),
        })
    }

    /// 設定した長さ（秒）で凪の刻を開始する。
    pub fn start(&self, duration_secs: f64) {
        self.lock().start(duration_secs);
    }

    /// 時間を進める。返り値で画面を描き、`ended` で完走/中断（＋要因）を検知する。
    pub fn tick(&self, dt_secs: f64, touching: bool) -> RenderState {
        self.lock().tick(dt_secs, touching).into()
    }

    /// 別アプリへ移った（背面化）。進行中なら即リセット。
    pub fn leave(&self) -> RenderState {
        self.lock().leave().into()
    }

    /// OS 割り込み（着信等）の開始（true）/終了（false）。割り込み中は時間を進めない。
    pub fn set_paused(&self, paused: bool) {
        self.lock().set_paused(paused);
    }

    /// 現在の状態を返す（試行は進めない）。
    pub fn snapshot(&self) -> RenderState {
        self.lock().snapshot().into()
    }
}

impl NagiTimer {
    /// 内部コアをロックする。中の操作は panic しないため poison は起こらない前提。
    fn lock(&self) -> std::sync::MutexGuard<'_, nagi_core::NagiTimer> {
        self.inner.lock().expect("NagiTimer mutex poisoned")
    }
}

/// 時間帯（0.0=真夜中, 0.5=正午, 1.0 で一周）に応じた配色を返す。
#[uniffi::export]
pub fn palette_for(time_of_day: f64) -> Palette {
    nagi_core::palette::palette_for(time_of_day).into()
}
