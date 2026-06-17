//! Nagi コア（規則の層）。
//!
//! 凪の刻の「規則」——いまどの状態か、いつリセットし、いつ完了するか——を
//! 端末に依存しない純粋な論理として持つ（DD §3 / Q1 の二層分離）。
//! 描画・音・触覚・前面背面の検知といった「感覚」は、この層には含めない。
//!
//! Phase 0 の目的は、この状態機械が正しく動くことを確かめること。外部クレートに
//! 依存せず std だけで書き、`cargo test` で検証できるようにしている。UniFFI による
//! 各プラットフォームへの公開（まず Android: Kotlin、後に iOS: Swift）と、アプリの
//! ビルド統合は、別途（Android SDK/NDK のある環境で）行う。

pub mod palette;

// 乱れゲージの初期値（PRD §4 の完走率を見ながら調整する仮値、DD Q4）。
// 上昇より下降を速くすることで、偶発的な短い接触は許し、意図的な操作の継続だけを
// 押し戻す——「穏やかさと強さの両立」を成立させる。

/// 触れている間の乱れの上昇速度（毎秒）。0.5/秒 → 約 2 秒の連続操作で閾値に達する。
const RISE_PER_SEC: f64 = 0.5;
/// 触れていない間の乱れの下降速度（毎秒）。1.0/秒 → 離せば約 1 秒で 0 に戻る。
const DECAY_PER_SEC: f64 = 1.0;
/// 乱れがこの値に達するとリセットする。
const RESET_THRESHOLD: f64 = 1.0;

/// 凪の刻の局面。落ち着き／乱れの別は連続量 `disturbance` で表すため、ここには持たない。
/// リセットは状態ではなく遷移（一過性）として扱う（`RenderState::ended`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// 未開始（長さを設定している段階）。最初の `start` までの状態。
    Idle,
    /// 進行中。落ち着き（disturbance≈0）と乱れ（disturbance>0）を含む。
    Running,
    /// 設定時間に到達して完了した。
    Completed,
}

/// 試行が中断した要因（PRD §4 の副指標＝リセット要因の分布の素地）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortReason {
    /// 画面操作による乱れが閾値に達した。
    Disturbance,
    /// 別アプリへ移った（背面化）。
    Left,
}

/// 一回の試行（起動から、完走または中断まで）の結末。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// 設定時間まで到達した（完走）。
    Completed,
    /// 途中で終わった（中断）。
    Aborted(AbortReason),
}

/// 一回の試行の記録（DD Q5）。完走率（PRD §4 主指標）とリセット要因の分布（副指標）の素地。
/// 開始時刻は壁時計を持たないコアでは決められないため、感覚の層が付与する。端末データベースへの
/// 保存は外部クレートを要するため、Phase 0 では構造のみ定義し、保存は感覚の層に委ねる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionRecord {
    /// 試行の開始時刻（Unix エポック秒）。感覚の層が壁時計から付与する。
    pub started_at_epoch_secs: i64,
    /// 設定した長さ（秒）。
    pub duration_set_secs: f64,
    /// 結末（完走 / 中断＋要因）。
    pub outcome: SessionEnd,
}

/// 感覚の層が毎フレーム描画に使う、コアの出力状態。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderState {
    pub phase: Phase,
    /// 残り時間（秒）。Running 以外では 0。
    pub remaining_secs: f64,
    /// 乱れゲージ 0.0..=1.0。波紋と警告音の強さをこの値に連動させる。
    pub disturbance: f64,
    /// このフレームで試行が終わったか（完走/中断＋要因）。終わっていなければ None。
    /// 感覚の層はこれを見て完了/リセットの合図を出し、セッション記録を残す。
    pub ended: Option<SessionEnd>,
}

/// 凪の刻の状態機械。
pub struct NagiTimer {
    duration_secs: f64,
    elapsed_secs: f64,
    disturbance: f64,
    phase: Phase,
    /// OS 割り込み（着信・緊急速報など）の最中か。割り込み中は時間を進めない（DD §7）。
    paused: bool,
}

impl Default for NagiTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl NagiTimer {
    #[must_use]
    pub fn new() -> Self {
        NagiTimer {
            duration_secs: 0.0,
            elapsed_secs: 0.0,
            disturbance: 0.0,
            phase: Phase::Idle,
            paused: false,
        }
    }

    /// 凪の刻を開始する。設定した長さで Running へ入る（PRD Step 1 の指を離した時点）。
    pub fn start(&mut self, duration_secs: f64) {
        self.duration_secs = duration_secs.max(0.0);
        self.elapsed_secs = 0.0;
        self.disturbance = 0.0;
        self.paused = false;
        self.phase = Phase::Running;
    }

    /// 時間を `dt_secs` 秒進める。`touching` はこのフレームで画面に触れているか。
    ///
    /// 触れていれば乱れが上昇し、離していれば下降する。乱れが閾値に達したら、時間を進めず
    /// 静かに最初へ戻す（中断＝Aborted(Disturbance)、同じ長さで新しい試行を始める）。そう
    /// でなければ時間を進め、設定時間に達したら完了にする（Completed）。
    pub fn tick(&mut self, dt_secs: f64, touching: bool) -> RenderState {
        // 進行中でない、または割り込み中は、状態を進めない。
        if self.phase != Phase::Running || self.paused {
            return self.snapshot();
        }

        // 乱れゲージの更新（触れていれば上昇、離していれば下降）。範囲は 0..=1。
        let delta = if touching {
            RISE_PER_SEC
        } else {
            -DECAY_PER_SEC
        };
        self.disturbance = (self.disturbance + delta * dt_secs).clamp(0.0, RESET_THRESHOLD);

        // 乱れが閾値に達したら中断。時間は進めず、同じ長さの凪を最初からやり直す。
        if self.disturbance >= RESET_THRESHOLD {
            self.reset_to_start();
            return self.snapshot_ended(SessionEnd::Aborted(AbortReason::Disturbance));
        }

        // 時間を進め、設定時間に達したら完了。
        self.elapsed_secs += dt_secs;
        if self.elapsed_secs >= self.duration_secs {
            self.elapsed_secs = self.duration_secs;
            self.phase = Phase::Completed;
            return self.snapshot_ended(SessionEnd::Completed);
        }
        self.snapshot()
    }

    /// 別アプリへ移った（背面化した）。進行中なら即中断する（Aborted(Left)、DD Q4）。
    /// 背面では何もしない時間を守れない以上、その時間は壊れたものとして扱う。
    pub fn leave(&mut self) -> RenderState {
        if self.phase == Phase::Running {
            self.reset_to_start();
            return self.snapshot_ended(SessionEnd::Aborted(AbortReason::Left));
        }
        self.snapshot()
    }

    /// OS 割り込みの開始（true）/終了（false）。割り込み中は時間を進めない。
    /// 着信や緊急速報はユーザーの操作ではないため、中断せず復帰時に再開する（DD §7）。
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// 現在の状態を、感覚の層向けの出力に変換する（試行は終わっていない）。
    #[must_use]
    pub fn snapshot(&self) -> RenderState {
        let remaining_secs = if self.phase == Phase::Running {
            (self.duration_secs - self.elapsed_secs).max(0.0)
        } else {
            0.0
        };
        RenderState {
            phase: self.phase,
            remaining_secs,
            disturbance: self.disturbance,
            ended: None,
        }
    }

    /// この呼び出しで試行が終わったことを添えたスナップショット。
    fn snapshot_ended(&self, end: SessionEnd) -> RenderState {
        RenderState {
            ended: Some(end),
            ..self.snapshot()
        }
    }

    /// 同じ長さの凪を最初からやり直す。経過と乱れを 0 に戻し、Running を保つ。
    /// 【仮】「別アプリ離脱は Idle（設定画面）へ戻す」案もありうる。本実装は両ケースを
    /// 同じ挙動（最初から再開）に統一している（DD Q4 で要再検討）。
    fn reset_to_start(&mut self) {
        self.elapsed_secs = 0.0;
        self.disturbance = 0.0;
        self.phase = Phase::Running;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn starts_in_idle() {
        let t = NagiTimer::new();
        assert_eq!(t.snapshot().phase, Phase::Idle);
    }

    #[test]
    fn start_enters_running_with_full_remaining() {
        let mut t = NagiTimer::new();
        t.start(60.0);
        let s = t.snapshot();
        assert_eq!(s.phase, Phase::Running);
        assert!(approx(s.remaining_secs, 60.0));
        assert!(approx(s.disturbance, 0.0));
    }

    #[test]
    fn advances_and_completes_without_touch() {
        let mut t = NagiTimer::new();
        t.start(3.0);
        let s1 = t.tick(1.0, false);
        assert_eq!(s1.phase, Phase::Running);
        assert!(approx(s1.remaining_secs, 2.0));
        assert!(s1.ended.is_none());
        t.tick(1.0, false);
        let s3 = t.tick(1.0, false);
        assert_eq!(s3.phase, Phase::Completed);
        assert!(approx(s3.remaining_secs, 0.0));
    }

    #[test]
    fn completion_reports_completed_outcome() {
        let mut t = NagiTimer::new();
        t.start(1.0);
        let s = t.tick(1.0, false);
        assert_eq!(s.phase, Phase::Completed);
        assert_eq!(s.ended, Some(SessionEnd::Completed));
    }

    #[test]
    fn sustained_touch_resets_with_disturbance_reason() {
        let mut t = NagiTimer::new();
        t.start(60.0);
        // 0.5/秒で上昇 → 2 秒未満ではリセットしない。
        let s = t.tick(1.9, true);
        assert_eq!(s.phase, Phase::Running);
        assert!(s.ended.is_none());
        assert!(s.disturbance > 0.9 && s.disturbance < 1.0);
        // 2 秒に達すると中断（操作による乱れ）し、最初へ戻る。
        let s = t.tick(0.2, true);
        assert_eq!(s.ended, Some(SessionEnd::Aborted(AbortReason::Disturbance)));
        assert_eq!(s.phase, Phase::Running);
        assert!(approx(s.remaining_secs, 60.0));
        assert!(approx(s.disturbance, 0.0));
    }

    #[test]
    fn brief_touch_then_release_does_not_reset() {
        let mut t = NagiTimer::new();
        t.start(60.0);
        // 0.3 秒の接触で乱れは 0.15。
        let s = t.tick(0.3, true);
        assert!(approx(s.disturbance, 0.15));
        assert!(s.ended.is_none());
        // 離すと 1.0/秒で下降。0.3 秒で 0 に戻る（クランプ）。偶発接触は許される。
        let s = t.tick(0.3, false);
        assert!(approx(s.disturbance, 0.0));
        assert_eq!(s.phase, Phase::Running);
        // 進捗（経過時間）は保たれている：合計 0.6 秒経過。
        assert!(approx(s.remaining_secs, 60.0 - 0.6));
    }

    #[test]
    fn repeated_short_touches_outpacing_decay_eventually_reset() {
        let mut t = NagiTimer::new();
        t.start(60.0);
        // 0.4 秒触れ（+0.2）→ 0.1 秒離す（-0.1）を繰り返すと、正味 +0.1/回 で蓄積する。
        let mut reason = None;
        for _ in 0..20 {
            let s = t.tick(0.4, true);
            if let Some(SessionEnd::Aborted(r)) = s.ended {
                reason = Some(r);
                break;
            }
            t.tick(0.1, false);
        }
        assert_eq!(
            reason,
            Some(AbortReason::Disturbance),
            "繰り返しの操作はいずれ中断に至るはず"
        );
    }

    #[test]
    fn aborted_attempt_restarts_and_can_complete() {
        let mut t = NagiTimer::new();
        t.start(2.0);
        t.tick(1.0, false); // 残り 1 秒まで進める
        let aborted = t.tick(2.0, true); // 連続操作で中断
        assert_eq!(
            aborted.ended,
            Some(SessionEnd::Aborted(AbortReason::Disturbance))
        );
        // 新しい試行として最初からやり直し、触れずに進めれば完走できる。
        let s = t.tick(2.0, false);
        assert_eq!(s.phase, Phase::Completed);
        assert_eq!(s.ended, Some(SessionEnd::Completed));
    }

    #[test]
    fn leave_reports_left_reason_and_restarts() {
        let mut t = NagiTimer::new();
        t.start(60.0);
        t.tick(10.0, false);
        assert!(approx(t.snapshot().remaining_secs, 50.0));
        let s = t.leave();
        assert_eq!(s.ended, Some(SessionEnd::Aborted(AbortReason::Left)));
        assert_eq!(s.phase, Phase::Running);
        assert!(approx(s.remaining_secs, 60.0)); // 最初へ戻った
    }

    #[test]
    fn leave_is_noop_when_not_running() {
        let mut t = NagiTimer::new();
        let s = t.leave(); // Idle で離脱しても何も起きない
        assert_eq!(s.phase, Phase::Idle);
        assert!(s.ended.is_none());
    }

    #[test]
    fn paused_does_not_advance_time_then_resumes() {
        let mut t = NagiTimer::new();
        t.start(10.0);
        t.tick(2.0, false);
        // OS 割り込み中は時間が進まない。
        t.set_paused(true);
        let s = t.tick(5.0, false);
        assert!(approx(s.remaining_secs, 8.0));
        // 割り込みが終われば再開する。
        t.set_paused(false);
        let s = t.tick(3.0, false);
        assert!(approx(s.remaining_secs, 5.0));
    }

    #[test]
    fn disturbance_is_clamped_to_unit_interval() {
        let mut t = NagiTimer::new();
        t.start(600.0);
        // 離し続けても 0 未満にならない。
        let s = t.tick(5.0, false);
        assert!(approx(s.disturbance, 0.0));
    }

    #[test]
    fn ticks_after_completion_are_noop() {
        let mut t = NagiTimer::new();
        t.start(1.0);
        let s = t.tick(1.0, false);
        assert_eq!(s.phase, Phase::Completed);
        // 完了後の tick は状態を変えず、試行終了も再報告しない。
        let s = t.tick(1.0, true);
        assert_eq!(s.phase, Phase::Completed);
        assert!(s.ended.is_none());
    }

    #[test]
    fn session_record_assembles_from_outcome() {
        // 感覚の層が、終了したフレームの outcome に壁時計と設定長を添えて記録する想定。
        let ended = SessionEnd::Aborted(AbortReason::Left);
        let record = SessionRecord {
            started_at_epoch_secs: 1_700_000_000,
            duration_set_secs: 300.0,
            outcome: ended,
        };
        assert_eq!(record.outcome, SessionEnd::Aborted(AbortReason::Left));
        assert!(approx(record.duration_set_secs, 300.0));
    }
}
