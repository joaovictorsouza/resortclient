package br.eng.joaovictor.resort

import androidx.compose.ui.window.ComposeUIViewController
import br.eng.joaovictor.resort.game.GameHost
import br.eng.joaovictor.resort.game.HostPaths
import br.eng.joaovictor.resort.game.parseEnv
import platform.Foundation.NSBundle
import platform.Foundation.NSFileManager
import platform.Foundation.NSProcessInfo
import platform.Foundation.NSUserDefaults
import platform.Foundation.NSCachesDirectory
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSSearchPathDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSString
import platform.Foundation.NSUserDomainMask
import platform.Foundation.NSUTF8StringEncoding
import platform.Foundation.stringWithContentsOfFile
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.readBytes
import kotlinx.cinterop.usePinned
import androidx.compose.ui.graphics.toComposeImageBitmap
import platform.Foundation.dataWithContentsOfFile
import platform.Foundation.NSData
import platform.Foundation.create
import platform.Foundation.writeToFile
import platform.UIKit.UIViewController

private fun dir(kind: NSSearchPathDirectory): String =
    NSSearchPathForDirectoriesInDomains(kind, NSUserDomainMask, true).firstOrNull() as? String ?: ""

// Created once per process, outside of the UI.
/**
 * Keychain items outlive the app: after a reinstall the old refresh token would still be there and
 * the player would come back logged in. The first launch after an install (no marker in
 * NSUserDefaults, which is deleted with the app) clears them. `-ResetSession` as a launch argument
 * does the same, and also removes the profile file (development).
 */
private fun resetStaleSession(services: IosServices, documents: String) {
    val defaults = NSUserDefaults.standardUserDefaults
    val args = NSProcessInfo.processInfo.arguments.map { it.toString() }
    val reset = "-ResetSession" in args
    if (reset || defaults.stringForKey("install_marker") == null) {
        services.clearAll()
        if (reset) NSFileManager.defaultManager.removeItemAtPath("$documents/profile.json", null)
        defaults.setObject("1", forKey = "install_marker")
        println("[host] session reset (${if (reset) "launch argument" else "fresh install"})")
    }
}

private val game: GameHost by lazy {
    val assets = (NSBundle.mainBundle.resourcePath ?: "") + "/assets"
    val env = NSString.stringWithContentsOfFile("$assets/dev.env", NSUTF8StringEncoding, null)
        ?.let { parseEnv(it) } ?: emptyMap()
    val services = IosServices()
    resetStaleSession(services, dir(NSDocumentDirectory))
    GameHost(HostPaths(dir(NSDocumentDirectory), dir(NSCachesDirectory), assets), env, services, IosGoogleAuth())
}

fun MainViewController(): UIViewController = ComposeUIViewController { App(game) }

internal actual fun platformLog(level: String, target: String, text: String) {
    println("[$level] $target: $text")
}

@OptIn(ExperimentalForeignApi::class)
internal actual fun writeFile(dir: String, name: String, bytes: ByteArray) {
    bytes.usePinned { pinned ->
        NSData.create(bytes = pinned.addressOf(0), length = bytes.size.toULong())
            .writeToFile("$dir/$name", atomically = true)
    }
}

@OptIn(ExperimentalForeignApi::class)
internal actual fun loadImageFile(path: String): androidx.compose.ui.graphics.ImageBitmap? {
    val data = NSData.dataWithContentsOfFile(path) ?: return null
    val bytes = data.bytes?.readBytes(data.length.toInt()) ?: return null
    return runCatching { org.jetbrains.skia.Image.makeFromEncoded(bytes).toComposeImageBitmap() }.getOrNull()
}
