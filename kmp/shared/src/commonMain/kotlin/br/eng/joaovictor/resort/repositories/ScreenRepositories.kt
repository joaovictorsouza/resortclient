package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.ffi.BevyEvent
import br.eng.joaovictor.resort.game.GameGateway
import br.eng.joaovictor.resort.models.CreatorState
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.models.NavigatorState
import br.eng.joaovictor.resort.models.ShopState
import br.eng.joaovictor.resort.models.parseCreator
import br.eng.joaovictor.resort.models.parseInventory
import br.eng.joaovictor.resort.models.RoomUiState
import br.eng.joaovictor.resort.models.parseNavigator
import br.eng.joaovictor.resort.models.parseRoomUi
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

    /**
     * A drag that left the inventory window: [phase] is `move`, `end` (released: place it) or `cancel`.
     * ([x], [y]) is the finger in physical pixels of the game surface.
     */
    fun drag(catalogId: String, x: Float, y: Float, phase: String) =
        game.command("inventory_drag", "catalog_id" to catalogId, "x" to x, "y" to y, "phase" to phase)
}

/** HUD buttons that open bevy_hui windows with no Compose screen yet. */
class HudRepository(private val game: GameGateway) {
    fun toggleAvatarMenu() = game.send("hud_avatar_menu", "{}")
    fun toggleRoomSettings() = game.send("hud_room_settings", "{}")
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

/** The in-room popups (`room_ui_state`): avatar menu, room settings (owner) and the doorbell (owner). */
class RoomUiRepository(private val game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<RoomUiState> = game.snapshots("room_ui_state", RoomUiState(), scope, ::parseRoomUi)

    fun closeAvatarMenu() = game.send("avatar_menu_close", "{}")
    fun visuals() = game.send("avatar_visuals", "{}")
    fun signOut() = game.send("avatar_sign_out", "{}")

    fun settingsName(text: String) = game.command("settings_name", "text" to text)

    /** `public|hidden|private` */
    fun settingsVisibility(visibility: String) = game.command("settings_visibility", "visibility" to visibility)
    fun settingsSave() = game.send("settings_save", "{}")
    fun settingsAskDelete() = game.send("settings_delete_ask", "{}")
    fun settingsCancelDelete() = game.send("settings_delete_cancel", "{}")
    fun settingsConfirmDelete() = game.send("settings_delete_confirm", "{}")
    fun settingsClose() = game.send("settings_close", "{}")
    fun answerDoorbell(requestId: String, allowed: Boolean) =
        game.command("doorbell_answer", "request_id" to requestId, "allowed" to allowed)
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

    /** Paints the active category's colour group with a `#RRGGBB` swatch. */
    fun color(hex: String) = game.command("creator_color", "hex" to hex)
    fun randomize() = game.send("creator_random", "{}")

    /** Turns the 3D preview by [delta] radians. */
    fun rotate(delta: Float) = game.command("creator_rotate", "delta" to delta)
    fun save() = game.send("creator_save", "{}")
    fun cancel() = game.send("creator_cancel", "{}")
}
