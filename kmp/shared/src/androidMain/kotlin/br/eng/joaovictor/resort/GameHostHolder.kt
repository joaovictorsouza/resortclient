package br.eng.joaovictor.resort

import android.content.Context
import android.util.Log
import br.eng.joaovictor.bevykmp.NativeBridge
import br.eng.joaovictor.resort.game.GameHost
import br.eng.joaovictor.resort.game.HostPaths
import br.eng.joaovictor.resort.game.parseEnv
import java.io.File

/** Process-wide holder: the Bevy app outlives Activity recreation. */
object GameHostHolder {
    private var instance: GameHost? = null

    @Synchronized
    fun get(context: Context): GameHost {
        instance?.let { return it }
        NativeBridge.load("game")
        NativeBridge.initAndroidContext(context.applicationContext)
        val app = context.applicationContext
        val assets = File(app.filesDir, "assets")
        copyAssetsIfNeeded(app, assets)
        val env = runCatching {
            app.assets.open("dev.env").bufferedReader().use { parseEnv(it.readText()) }
        }.getOrDefault(emptyMap())
        val paths = HostPaths(app.filesDir.absolutePath, app.cacheDir.absolutePath, assets.absolutePath)
        return GameHost(paths, env, AndroidServices(app), AndroidGoogleAuth()).also { instance = it }
    }

    /** Bevy cannot read APK assets without an Activity, so they are copied once per app version. */
    private fun copyAssetsIfNeeded(context: Context, target: File) {
        val version = context.packageManager.getPackageInfo(context.packageName, 0).let {
            "${it.lastUpdateTime}"
        }
        val marker = File(target, ".copied")
        if (marker.exists() && marker.readText() == version) return
        target.deleteRecursively()
        copyDir(context, "", target)
        File(target, ".copied").writeText(version)
        Log.i("GameHostHolder", "copied bundled assets to $target")
    }

    private fun copyDir(context: Context, path: String, target: File) {
        target.mkdirs()
        val names = context.assets.list(path) ?: return
        for (name in names) {
            val child = if (path.isEmpty()) name else "$path/$name"
            val children = context.assets.list(child)
            if (children != null && children.isNotEmpty()) {
                copyDir(context, child, File(target, name))
            } else {
                // `dev.env` is runtime config read separately, not a game asset.
                if (child == "dev.env") continue
                context.assets.open(child).use { input ->
                    File(target, name).outputStream().use { input.copyTo(it) }
                }
            }
        }
    }
}

internal actual fun platformLog(level: String, target: String, text: String) {
    when (level) {
        "Error" -> Log.e("game", "$target: $text")
        "Warn" -> Log.w("game", "$target: $text")
        else -> Log.i("game", "$target: $text")
    }
}

internal actual fun writeFile(dir: String, name: String, bytes: ByteArray) {
    File(dir).mkdirs()
    File(dir, name).writeBytes(bytes)
}
