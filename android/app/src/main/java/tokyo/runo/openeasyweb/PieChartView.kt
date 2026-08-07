package tokyo.runo.openeasyweb

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.View

/**
 * 使用率(0.0〜1.0)を受け取り、使用中/空きの2色で円グラフ(ドーナツ状の
 * 円弧)を描画する汎用カスタムView(2026-08-06新規実装)。
 *
 * 外部グラフライブラリには依存せず、`android.graphics.Canvas`と`Paint`のみで
 * 描画する(`server/src/shell.rs`側のWASM UIがSVGの`stroke-dasharray`で
 * 同様の円グラフを描いているのと同じ考え方——依存を増やさず標準APIのみで
 * 実現する既存方針をAndroidネイティブ側でも踏襲)。
 *
 * 使い方: `pieChartView.setUsage(0.42f)`で使用率42%を反映。
 * `usedColor`/`freeColor`プロパティで色を変更できる(既定は赤系/緑系)。
 */
class PieChartView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    /** 使用率(0.0〜1.0)。範囲外の値はコンストラクタ/setter側でクランプする。 */
    private var usedRatio: Float = 0f

    /** 使用中を表す弧の色(既定: 赤系)。 */
    var usedColor: Int = 0xFFE53935.toInt()
        set(value) {
            field = value
            invalidate()
        }

    /** 空きを表す弧の色(既定: 緑系)。 */
    var freeColor: Int = 0xFF43A047.toInt()
        set(value) {
            field = value
            invalidate()
        }

    /** 円弧の太さ(ドーナツ状にする場合、dp単位)。0にすると塗りつぶしの円になる。 */
    var strokeWidthDp: Float = 18f
        set(value) {
            field = value
            invalidate()
        }

    private val arcPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.BUTT
    }

    private val bounds = RectF()

    /** 使用率(0.0〜1.0)を設定して再描画する。範囲外の値は0.0〜1.0へクランプする。 */
    fun setUsage(usedRatio: Float) {
        this.usedRatio = usedRatio.coerceIn(0f, 1f)
        invalidate()
    }

    fun getUsage(): Float = usedRatio

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)

        val strokeWidthPx = strokeWidthDp * resources.displayMetrics.density
        arcPaint.strokeWidth = strokeWidthPx

        val inset = strokeWidthPx / 2f + 2f
        bounds.set(inset, inset, width - inset, height - inset)
        if (bounds.width() <= 0f || bounds.height() <= 0f) return

        val usedSweep = 360f * usedRatio
        val freeSweep = 360f - usedSweep

        // 12時方向(-90度)から時計回りに「使用中」→「空き」の順で描く。
        arcPaint.color = usedColor
        canvas.drawArc(bounds, -90f, usedSweep, false, arcPaint)

        arcPaint.color = freeColor
        canvas.drawArc(bounds, -90f + usedSweep, freeSweep, false, arcPaint)
    }
}
