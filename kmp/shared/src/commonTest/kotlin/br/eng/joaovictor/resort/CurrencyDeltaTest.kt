package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.ds.components.badgeCountText
import br.eng.joaovictor.resort.ds.components.currencyDelta
import br.eng.joaovictor.resort.ds.format.formatGroupedAmount
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class CurrencyDeltaTest {
    @Test
    fun noCelebrationOnTheFirstLoadOrUnknownBalance() {
        assertNull(currencyDelta(null, 500))
        assertNull(currencyDelta(500, null))
        assertNull(currencyDelta(null, null))
    }

    @Test
    fun onlyARiseIsCelebrated() {
        assertEquals(120L, currencyDelta(1_000, 1_120))
        assertNull(currencyDelta(1_000, 1_000))
        assertNull(currencyDelta(1_000, 400)) // a purchase just counts down
    }

    @Test
    fun amountsGroupThousandsLikeThePortugueseUi() {
        assertEquals("1.234.567", formatGroupedAmount(1_234_567))
        assertEquals("0", formatGroupedAmount(0))
        assertEquals("-1.000", formatGroupedAmount(-1000))
    }

    @Test
    fun badgeCountsCapAtNinePlus() {
        assertEquals("3", badgeCountText(3))
        assertEquals("9", badgeCountText(9))
        assertEquals("9+", badgeCountText(10))
    }
}
