package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.models.parseWallet
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertNull
import kotlin.test.assertEquals

@OptIn(ExperimentalCoroutinesApi::class)
class WalletRepositoryTest {
    @Test
    fun startsUnloadedAtZero() = runTest {
        val repo = WalletRepository(FakeGateway(), CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        assertEquals(Wallet(silver = 0, gold = 0, loaded = false), repo.state.value)
    }

    @Test
    fun walletStateUpdatesTheBalance() = runTest {
        val gateway = FakeGateway()
        val repo = WalletRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        gateway.emitApp("wallet_state", """{"silver":12,"gold":3,"loaded":true}""")
        assertEquals(Wallet(12, 3, true), repo.state.value)
        gateway.emitApp("wallet_state", """{"silver":10,"gold":3,"loaded":true}""")
        assertEquals(10, repo.state.value.silver)
    }

    @Test
    fun otherEventsAreIgnored() = runTest {
        val gateway = FakeGateway()
        val repo = WalletRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        gateway.emitApp("hud_state", """{"silver":99,"gold":99}""")
        assertEquals(Wallet(), repo.state.value)
    }

    @Test
    fun aMalformedPayloadKeepsTheLastGoodBalance() = runTest {
        val gateway = FakeGateway()
        val repo = WalletRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        gateway.emitApp("wallet_state", """{"silver":7,"gold":1,"loaded":true}""")
        gateway.emitApp("wallet_state", "garbage")
        gateway.emitApp("wallet_state", """{"silver":8,"gold":1,"loaded":true}""")
        assertEquals(Wallet(8, 1, true), repo.state.value) // the flow survived the bad payload
    }

    @Test
    fun parsingIsTolerant() {
        assertEquals(Wallet(1234567, 0, true), parseWallet("""{"silver":1234567,"gold":0,"loaded":true}"""))
        assertNull(parseWallet("not json"))
        assertNull(parseWallet(""))
        assertEquals(Wallet(silver = 5), parseWallet("""{"silver":5}"""))
        assertEquals(Wallet(1, 2, true), parseWallet("""{"silver":1,"gold":2,"loaded":true,"future_field":"x"}"""))
    }
}
