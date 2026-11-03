package br.eng.joaovictor.resort.models

import br.eng.joaovictor.resort.game.BevyJson
import kotlinx.serialization.Serializable

/** Feedback line of a screen (`kind` is `success` or `error`). */
@Serializable
data class StatusLine(val kind: String = "", val text: String = "")

// ---- Shop (`shop_state`) ----

@Serializable
data class ShopCategory(val id: String = "", val name: String = "", val icon: String = "", val parent: String = "")

@Serializable
data class ShopOffer(
    val catalogId: String = "",
    val categoryId: String = "",
    val name: String = "",
    val width: Int = 1,
    val depth: Int = 1,
    val walkable: Boolean = false,
    val priceSilver: Long = 0,
    val priceGold: Long = 0,
    /** Local file of the thumbnail in the game's CDN cache; empty until it is downloaded. */
    val thumbnail: String = "",
)

@Serializable
data class ShopState(
    val open: Boolean = false,
    val loading: Boolean = false,
    val categories: List<ShopCategory> = emptyList(),
    val offers: List<ShopOffer> = emptyList(),
    val selectedCategory: String? = null,
    val query: String = "",
    val pending: String? = null,
    val status: StatusLine? = null,
)

// ---- Inventory (`inventory_state`) ----

@Serializable
data class InventoryItem(
    val id: String = "",
    val catalogId: String = "",
    val name: String = "",
    val category: String = "",
    val quantity: Int = 0,
    val width: Int = 1,
    val depth: Int = 1,
    val walkable: Boolean = false,
    val color: String = "",
    val assetId: String = "",
    /** Local file of the thumbnail in the game's CDN cache; empty until it is downloaded. */
    val thumbnail: String = "",
)

@Serializable
data class InventoryState(
    val open: Boolean = false,
    val loading: Boolean = false,
    val items: List<InventoryItem> = emptyList(),
    val selectedCatalogId: String? = null,
    /** Only the owner of the room can place items. */
    val canPlace: Boolean = false,
)

// ---- Room navigator (`navigator_state`) ----

@Serializable
data class RoomSummary(
    val roomId: String = "",
    val name: String = "",
    val ownerId: String? = null,
    val ownerName: String? = null,
    val visibility: String = "public",
    val userCount: Int = 0,
    val maxUsers: Int = 0,
    val matchId: String? = null,
)

@Serializable
data class RoomLayout(
    val id: String = "",
    val name: String = "",
    val width: Int = 0,
    val height: Int = 0,
    val maxUsers: Int = 0,
    val imageAsset: String = "",
)

/** A built-in public space (lobby, cafe), fixed in the game. */
@Serializable
data class PublicSpace(val roomId: String = "", val name: String = "", val description: String = "", val maxUsers: Int = 0)

@Serializable
data class NavigatorState(
    val open: Boolean = false,
    /** `public|popular|mine|search|create` */
    val tab: String = "public",
    val public: List<PublicSpace> = emptyList(),
    val popular: List<RoomSummary> = emptyList(),
    val mine: List<RoomSummary> = emptyList(),
    val search: List<RoomSummary> = emptyList(),
    val layouts: List<RoomLayout> = emptyList(),
    val query: String = "",
    val searching: Boolean = false,
    val hasSearched: Boolean = false,
    val loadingPopular: Boolean = false,
    val status: String? = null,
)

// ---- In-room popups (`room_ui_state`) ----

@Serializable
data class SettingsInfo(val name: String = "", val visibility: String = "public", val confirmingDelete: Boolean = false)

@Serializable
data class DoorbellInfo(val requestId: String = "", val requesterName: String = "")

/** The small popups the shell draws inside a room: avatar menu, room settings and the doorbell. */
@Serializable
data class RoomUiState(
    val avatarMenu: Boolean = false,
    val settings: SettingsInfo? = null,
    val doorbell: DoorbellInfo? = null,
)

// ---- Character creator (`creator_state`) ----

@Serializable
data class CreatorItem(
    val id: String = "",
    val name: String = "",
    /** Asset path (`ui/thumbs/characters/<id>.png`); resolve it against the CDN base with [thumbnailUrl]. */
    val thumbnail: String = "",
    val selected: Boolean = false,
    /** Local file of the thumbnail in the game's CDN cache; empty until it is downloaded. */
    val thumbnailFile: String = "",
) {
    fun thumbnailUrl(cdnBase: String): String = "${cdnBase.trimEnd('/')}/$thumbnail"
}

@Serializable
data class CreatorSection(
    val key: String = "",
    val label: String = "",
    val items: List<CreatorItem> = emptyList(),
    /** `#RRGGBB` swatches of the category; empty when it has none. */
    val palette: List<String> = emptyList(),
    /** The category's current main colour. */
    val color: String? = null,
)

@Serializable
data class CreatorAppearance(
    val bodyType: String = "female",
    val faceId: String = "",
    val hairId: String = "",
    val hatId: String = "",
    val glassesId: String = "",
    val topId: String = "",
    val bottomId: String = "",
    val outfitId: String = "",
    val shoesId: String = "",
)

@Serializable
data class CreatorState(
    val active: Boolean = false,
    /** Editing an existing avatar (can close without saving) rather than creating the first one. */
    val editing: Boolean = false,
    val confirmLabel: String = "",
    val category: String = "face",
    val appearance: CreatorAppearance = CreatorAppearance(),
    val categories: List<CreatorSection> = emptyList(),
) {
    fun section(key: String): CreatorSection? = categories.firstOrNull { it.key == key }
}

fun parseShop(json: String): ShopState? = BevyJson.decodeOrNull<ShopState>(json)
fun parseInventory(json: String): InventoryState? = BevyJson.decodeOrNull<InventoryState>(json)
fun parseNavigator(json: String): NavigatorState? = BevyJson.decodeOrNull<NavigatorState>(json)
fun parseRoomUi(json: String): RoomUiState? = BevyJson.decodeOrNull<RoomUiState>(json)
fun parseCreator(json: String): CreatorState? = BevyJson.decodeOrNull<CreatorState>(json)
