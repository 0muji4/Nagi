//! 配色の計算（規則の層）。
//!
//! 時間帯から、隣り合う色相（アナロガス配色）でできた穏やかなグラデーションを返す
//! （DD Q3、PRD Step 2）。時刻の取得や実際の描画は感覚の層が行い、ここは時刻→配色の
//! 純粋な写像だけを持つ。`Date::now` 等には触れないため、現在時刻は引数で受け取る。

/// 0.0..=1.0 の RGB 成分を持つ色。感覚の層（Compose の Color 等）がそのまま使える。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// グラデーションの色の並び（順序付き）。手前=暗め → 奥=明るめ の順で持つ。
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub stops: Vec<Color>,
}

// 穏やかさのための定数。鮮やかさを抑え（彩度は中程度）、隣り合う色相だけを使う
// ことで、認知的なノイズを避ける（Background §3.1）。いずれも調整しうる初期値。

/// アナロガス配色の色相の振り幅（度）。中心色相 ± この値で隣り合う色相にする。
const ANALOGOUS_OFFSET_DEG: f64 = 28.0;
/// 彩度。鮮やかすぎるとノイズになるため中程度に抑える。
const SATURATION: f64 = 0.45;
/// グラデーションに奥行きを出すための明度（手前=暗め → 奥=明るめ）。3 ストップ。
const LIGHTNESS_STOPS: [f64; 3] = [0.28, 0.40, 0.52];

/// 時間帯（一日の位置）の代表点と、その時刻の中心色相（度）。
/// 夜=藍、朝=暖かい琥珀、昼=空の青、夕=夕焼けの橙。末尾は先頭と同値にして、
/// 日をまたぐ連続性（`time_of_day` の 0.0 と 1.0 が一致）を保証する。
const HUE_KEYFRAMES: [(f64, f64); 5] = [
    (0.00, 250.0), // 夜（藍）
    (0.25, 35.0),  // 朝（琥珀）
    (0.50, 205.0), // 昼（空の青）
    (0.75, 25.0),  // 夕（橙）
    (1.00, 250.0), // 夜（先頭へ戻る）
];

/// 時間帯（0.0=真夜中, 0.5=正午, 1.0 で一周）に応じた配色を返す。
///
/// 中心色相を時刻から求め、その左・中央・右（隣り合う色相）に奥行きのための明度を
/// 割り当てた 3 ストップのグラデーションにする。
#[must_use]
pub fn palette_for(time_of_day: f64) -> Palette {
    let t = time_of_day.rem_euclid(1.0);
    let base = base_hue(t);
    let hues = [
        base - ANALOGOUS_OFFSET_DEG,
        base,
        base + ANALOGOUS_OFFSET_DEG,
    ];
    let stops = hues
        .iter()
        .zip(LIGHTNESS_STOPS.iter())
        .map(|(&h, &l)| hsl_to_rgb(h, SATURATION, l))
        .collect();
    Palette { stops }
}

/// 時間帯から中心色相（度, 0..360）を求める。代表点の間を色相環上の最短弧で補間する。
fn base_hue(t: f64) -> f64 {
    for w in HUE_KEYFRAMES.windows(2) {
        let (t0, h0) = w[0];
        let (t1, h1) = w[1];
        if t >= t0 && t <= t1 {
            let span = t1 - t0;
            let local = if span > 0.0 { (t - t0) / span } else { 0.0 };
            return lerp_hue(h0, h1, local);
        }
    }
    HUE_KEYFRAMES[0].1 // 到達しないはずだが安全側
}

/// 色相を最短弧で補間する。a,b は度。戻り値は 0..360。
/// 例: 350° と 10° の間は 180° 側ではなく 0° をまたぐ 20° 側を通る。
fn lerp_hue(a: f64, b: f64, t: f64) -> f64 {
    // a→b の差を (-180, 180] に正規化し、最短の向きで回す。
    let diff = (((b - a) + 540.0).rem_euclid(360.0)) - 180.0;
    (a + diff * t).rem_euclid(360.0)
}

/// HSL（色相°, 彩度 0..1, 明度 0..1）を RGB（各 0..1）へ変換する。標準的な式。
// h/s/l/c/x/m は HSL→RGB の標準的な式の記号そのもの。説明的な名前にするとかえって
// 式との対応が見えにくくなるため、慣例の一文字名を保つ。
#[allow(clippy::many_single_char_names)]
fn hsl_to_rgb(h_deg: f64, s: f64, l: f64) -> Color {
    let h = h_deg.rem_euclid(360.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color {
        r: r1 + m,
        g: g1 + m,
        b: b1 + m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn hsl_to_rgb_primaries() {
        let red = hsl_to_rgb(0.0, 1.0, 0.5);
        assert!(approx(red.r, 1.0) && approx(red.g, 0.0) && approx(red.b, 0.0));
        let green = hsl_to_rgb(120.0, 1.0, 0.5);
        assert!(approx(green.r, 0.0) && approx(green.g, 1.0) && approx(green.b, 0.0));
        let blue = hsl_to_rgb(240.0, 1.0, 0.5);
        assert!(approx(blue.r, 0.0) && approx(blue.g, 0.0) && approx(blue.b, 1.0));
    }

    #[test]
    fn hsl_zero_saturation_is_gray() {
        let gray = hsl_to_rgb(123.0, 0.0, 0.5);
        assert!(approx(gray.r, 0.5) && approx(gray.g, 0.5) && approx(gray.b, 0.5));
    }

    #[test]
    fn lerp_hue_takes_shortest_arc_across_zero() {
        // 350 → 10 は 0 をまたぐ短い側（+20）を通り、中点は 0（=360）になる。
        let mid = lerp_hue(350.0, 10.0, 0.5);
        assert!(approx(mid, 0.0) || approx(mid, 360.0));
    }

    #[test]
    fn base_hue_matches_keyframes() {
        assert!(approx(base_hue(0.0), 250.0));
        assert!(approx(base_hue(0.25), 35.0));
        assert!(approx(base_hue(0.5), 205.0));
        assert!(approx(base_hue(0.75), 25.0));
        assert!(approx(base_hue(1.0), 250.0));
    }

    #[test]
    fn base_hue_is_continuous_across_midnight() {
        // 1.0 直前と 0.0 が滑らかに繋がる（夜の藍へ収束）。
        let before_midnight = base_hue(0.999);
        assert!((before_midnight - 250.0).abs() < 2.0);
    }

    #[test]
    fn palette_has_three_stops_in_unit_range() {
        for i in 0..=100 {
            let p = palette_for(f64::from(i) / 100.0);
            assert_eq!(p.stops.len(), 3);
            for c in &p.stops {
                assert!(c.r >= 0.0 && c.r <= 1.0, "r 範囲外: {}", c.r);
                assert!(c.g >= 0.0 && c.g <= 1.0, "g 範囲外: {}", c.g);
                assert!(c.b >= 0.0 && c.b <= 1.0, "b 範囲外: {}", c.b);
            }
        }
    }

    #[test]
    fn morning_is_warm_midday_is_cool() {
        // 朝（琥珀）は赤が青より強く、昼（空の青）は青が赤より強い。
        let morning = palette_for(0.25).stops[1];
        assert!(morning.r > morning.b, "朝は暖色のはず: {morning:?}");
        let midday = palette_for(0.5).stops[1];
        assert!(midday.b > midday.r, "昼は寒色のはず: {midday:?}");
    }

    #[test]
    fn palette_is_cyclic_over_one_day() {
        assert_eq!(palette_for(0.0), palette_for(1.0));
    }

    #[test]
    fn stops_are_ordered_dark_to_light() {
        // 明度が手前→奥で上がる（奥行き）。緑成分の代わりに、明度の代理として
        // 各ストップの明るさ（成分の和）が単調増加することを確かめる。
        let p = palette_for(0.5);
        let lum = |c: &Color| c.r + c.g + c.b;
        assert!(lum(&p.stops[0]) < lum(&p.stops[1]));
        assert!(lum(&p.stops[1]) < lum(&p.stops[2]));
    }
}
