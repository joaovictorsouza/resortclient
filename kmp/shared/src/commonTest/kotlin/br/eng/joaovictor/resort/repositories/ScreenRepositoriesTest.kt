package br.eng.joaovictor.resort.repositories

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ScreenRepositoriesTest {
    @Test
    fun shopSnapshotParsesSnakeCaseKeys() = runTest {
        val gateway = FakeGateway()
        val repo = ShopRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        gateway.emitApp(
            "shop_state",
            """{"open":true,"loading":false,"categories":[],"offers":[{"catalog_id":"chair","category_id":"c","name":"Chair","width":1,"depth":1,"walkable":false,"price_silver":5,"price_gold":0}],"selected_category":"c","query":"","pending":null,"status":{"kind":"error","text":"x"}}""",
        )
        val s = repo.state.value
        assertTrue(s.open)
        assertEquals("chair", s.offers.single().catalogId)
        assertEquals(5, s.offers.single().priceSilver)
        assertEquals("c", s.selectedCategory)
        assertEquals("error", s.status?.kind)
    }

    @Test
    fun commandsAreSentAsSnakeCaseJson() = runTest {
        val gateway = FakeGateway()
        val scope = CoroutineScope(UnconfinedTestDispatcher(testScheduler))
        ShopRepository(gateway, scope).purchase("chair")
        NavigatorRepository(gateway, scope).create("Casa", null)
        NavigatorRepository(gateway, scope).join("r1")
        assertEquals("shop_purchase" to """{"catalog_id":"chair"}""", gateway.sent[0])
        assertEquals("navigator_create" to """{"name":"Casa"}""", gateway.sent[1])
        assertEquals("navigator_join" to """{"room_id":"r1"}""", gateway.sent[2])
    }

    @Test
    fun creatorItemsCarryAThumbnailTheShellCanResolve() = runTest {
        val gateway = FakeGateway()
        val repo = CreatorRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        gateway.emitApp(
            "creator_state",
            """{"active":true,"category":"hair","appearance":{"body_type":"male","face_id":"f1","hair_id":"h1","hat_id":"","glasses_id":"","top_id":"","bottom_id":"","outfit_id":"","shoes_id":""},"categories":[{"key":"hair","items":[{"id":"h1","name":"Hair 1","thumbnail":"ui/thumbs/characters/h1.png","selected":true}]}]}""",
        )
        val item = repo.state.value.section("hair")!!.items.single()
        assertEquals("male", repo.state.value.appearance.bodyType)
        assertEquals("https://cdn.test/ui/thumbs/characters/h1.png", item.thumbnailUrl("https://cdn.test/"))
    }

    @Test
    fun inventoryAndNavigatorStartEmptyAndIgnoreGarbage() = runTest {
        val gateway = FakeGateway()
        val scope = CoroutineScope(UnconfinedTestDispatcher(testScheduler))
        val inv = InventoryRepository(gateway, scope)
        val nav = NavigatorRepository(gateway, scope)
        gateway.emitApp("inventory_state", "not json")
        gateway.emitApp("navigator_state", """{"open":true,"tab":"mine","mine":[{"room_id":"a","name":"A","visibility":"private","user_count":1,"max_users":10}]}""")
        assertTrue(inv.state.value.items.isEmpty())
        assertEquals("mine", nav.state.value.tab)
        assertEquals("a", nav.state.value.mine.single().roomId)
    }
}
