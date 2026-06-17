package io.github.muji4.nagi

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.style.TextAlign
import uniffi.nagi_ffi.NagiTimer
import uniffi.nagi_ffi.paletteFor

// ③a 最小疎通（walking skeleton）。
// 目的は「Kotlin → Rust(FFI) が実行時に通ること」の確認であって、体験の作り込みではない。
// 流体グラデーション・環境音・触覚・水滴UI などは ③b で乗せる。
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { MaterialTheme { NagiSkeleton() } }
    }
}

@androidx.compose.runtime.Composable
fun NagiSkeleton() {
    // FFI 越しに Rust の状態機械を生成（.so のロードと疎通の確認）。
    val timer = remember { NagiTimer() }
    DisposableEffect(Unit) { onDispose { timer.close() } }

    var state by remember { mutableStateOf(timer.snapshot()) }
    var touching by remember { mutableStateOf(false) }

    // 60 秒の凪を開始し、毎フレーム tick する（押している間は touching=true で乱れが上がる）。
    LaunchedEffect(Unit) {
        timer.start(60.0)
        var last = 0L
        while (true) {
            val now = withFrameNanos { it }
            if (last != 0L) {
                state = timer.tick((now - last) / 1_000_000_000.0, touching)
            }
            last = now
        }
    }

    // 時間帯に応じた配色（デモは正午 = 0.5）。中央のストップを背景に使う。
    val c = remember { paletteFor(0.5).stops[1] }
    val background = Color(c.r.toFloat(), c.g.toFloat(), c.b.toFloat())

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(background)
            .pointerInput(Unit) {
                detectTapGestures(
                    onPress = {
                        touching = true
                        tryAwaitRelease()
                        touching = false
                    },
                )
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = "phase = ${state.phase}\n" +
                "remaining = ${"%.1f".format(state.remainingSecs)} s\n" +
                "disturbance = ${"%.2f".format(state.disturbance)}\n" +
                "ended = ${state.ended}",
            textAlign = TextAlign.Center,
        )
    }
}
