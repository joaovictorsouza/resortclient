package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.ffi.BevyEvent
import br.eng.joaovictor.resort.game.GameGateway
import br.eng.joaovictor.resort.models.CreatorState
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.models.NavigatorState
import br.eng.joaovictor.resort.models.ShopState
import br.eng.joaovictor.resort.models.parseCreator
import br.eng.joaovictor.resort.models.parseInventory
import br.eng.joaovictor.resort.models.parseNavigator
import br.eng.joaovictor.resort.models.parseShop
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.mapNotNull
import kotlinx.coroutines.flow.stateIn
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * Latest snapshot of one game screen. The game publishes the whole state each time it changes (and on
 * resume), so a malformed payload just keeps the last good value. Create the repositories once, right
 * after the `GameHost`, so no snapshot is missed.
 */
private fun <T> GameGateway.snapshots(
    event: String,
    initial: T,
    scope: CoroutineScope,
    parse: (String) -> T?,
): StateFlow<T> = events
    .filterIsInstance<BevyEvent.App>()
    .filter { it.name == event }
    .mapNotNull { parse(it.payload.decodeToString()) }
    .stateIn(scope, SharingStarted.Eagerly, initial)

private fun GameGateway.command(name: String, vararg fields: Pair<String, Any?>) {
    val body = JsonObject(
        fields.filter { it.second != null }.associate { (k, v) ->
            k to when (v) {
                is Boolean -> JsonPrimitive(v)
                is Number -> JsonPrimitive(v)
                else -> JsonPrimitive(v.toString())
            }
        },
    )
    send(name, body.toString())
}

/** The shop window (`shop_state`). Purchases are validated again by the server. */
class ShopRepository(private val game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<ShopState> = game.snapshots("shop_state", ShopState(), scope, ::parseShop)
    fun setOpen(open: Boolean) = game.command("shop_open", "open" to open)
    fun selectCategory(id: String) = game.command("shop_category", "id" to id)
    fun search(query: String) = game.command("shop_search", "query" to query)
    fun purchase(catalogId: String) = game.command("shop_purchase", "catalog_id" to catalogId)
}

/** The furniture inventory (`inventory_state`). */
class InventoryRepository(private val game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<InventoryState> = game.snapshots("inventory_state", InventoryState(), scope, ::parseInventory)
    fun setOpen(open: Boolean) = game.command("inventory_open", "open" to open)
    fun select(catalogId: String) = game.command("inventory_select", "catalog_id" to catalogId)
}

/** The room navigator (`navigator_state`): lists, search, joining and creating rooms. */
class NavigatorRepository(private val game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<NavigatorState> = game.snapshots("navigator_state", NavigatorState(), scope, ::parseNavigator)
    fun setOpen(open: Boolean) = game.command("navigator_open", "open" to open)

    /** `public|popular|mine|search|create`; the game fetches what the tab needs. */
    fun selectTab(tab: String) = game.command("navigator_tab", "tab" to tab)
    fun setQuery(query: String) = game.command("navigator_query", "query" to query)
    fun search() = game.send("navigator_search", "{}")
    fun join(roomId: String) = game.command("navigator_join", "room_id" to roomId)
    fun create(name: String, layoutId: String? = null) =
        game.command("navigator_create", "name" to name, "layout_id" to layoutId)
}

/**
 * The character creator (`creator_state`). Every item has a `thumbnail` (a picture, not the asset name);
 * resolve it with `CreatorItem.thumbnailUrl(cdnBase)`.
 */
class CreatorRepository(private val game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<CreatorState> = game.snapshots("creator_state", CreatorState(), scope, ::parseCreator)

    /** `body|face|skin|hair|top|bottom|outfit|shoes|accessories` */
    fun selectCategory(category: String) = game.command("creator_category", "category" to category)

    /** Picks (or, for slots that can be empty, removes) an item of the active category. */
    fun pick(id: String) = game.command("creator_pick", "id" to id)
    fun setBody(bodyType: String) = game.command("creator_body", "body_type" to bodyType)
    fun save() = game.send("creator_save", "{}")
    fun cancel() = game.send("creator_cancel", "{}")
}
