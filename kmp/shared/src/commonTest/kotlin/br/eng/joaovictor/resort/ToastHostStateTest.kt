package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.ds.components.ToastHostState
import br.eng.joaovictor.resort.ds.components.ToastKind
import br.eng.joaovictor.resort.ds.components.ToastMessage
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ToastHostStateTest {
    private fun toast(id: Long) = ToastMessage(id, ToastKind.Info, "t$id")

    @Test
    fun showsOneAtATimeAndQueuesTheRest() {
        val host = ToastHostState()
        host.show(toast(1))
        host.show(toast(2))
        assertEquals(1L, host.current.value?.id)
        host.dismiss(1)
        assertEquals(2L, host.current.value?.id)
        host.dismiss(2)
        assertNull(host.current.value)
    }

    @Test
    fun dismissingAQueuedOrUnknownToastDoesNotDisturbTheCurrentOne() {
        val host = ToastHostState()
        host.show(toast(1))
        host.show(toast(2))
        host.dismiss(2)
        host.dismiss(99)
        assertEquals(1L, host.current.value?.id)
        host.dismiss(1)
        assertNull(host.current.value)
    }
}
