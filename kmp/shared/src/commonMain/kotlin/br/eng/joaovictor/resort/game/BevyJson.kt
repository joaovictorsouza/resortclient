package br.eng.joaovictor.resort.game

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNamingStrategy
import kotlinx.serialization.encodeToString
import kotlinx.serialization.decodeFromString

/**
 * The one JSON configuration for every message exchanged with the game. Unknown keys are ignored,
 * and an unknown or null value for a field that has a default falls back to that default, so a newer
 * game can talk to an older shell. A payload that is not valid JSON never throws: [decodeOrNull] gives `null`.
 */
object BevyJson {
    @OptIn(ExperimentalSerializationApi::class)
    val json = Json {
        // The game speaks snake_case (`selected_category`); Kotlin properties stay camelCase.
        namingStrategy = JsonNamingStrategy.SnakeCase
        ignoreUnknownKeys = true
        isLenient = false
        coerceInputValues = true
    }

    inline fun <reified T> decodeOrNull(text: String): T? =
        try {
            json.decodeFromString<T>(text)
        } catch (_: SerializationException) {
            null
        } catch (_: IllegalArgumentException) {
            null
        }

    inline fun <reified T> encode(value: T): String = json.encodeToString(value)
}
