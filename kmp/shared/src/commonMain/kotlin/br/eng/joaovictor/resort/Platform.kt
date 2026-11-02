package br.eng.joaovictor.resort

internal expect fun writeFile(dir: String, name: String, bytes: ByteArray)

internal expect fun platformLog(level: String, target: String, text: String)

/** Decodes an image file (PNG) from the game's cache; `null` when it is missing or not an image. */
internal expect fun loadImageFile(path: String): androidx.compose.ui.graphics.ImageBitmap?
