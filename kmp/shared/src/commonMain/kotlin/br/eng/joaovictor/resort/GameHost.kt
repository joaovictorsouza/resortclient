package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.ffi.BevyBackendKind
import br.eng.joaovictor.resort.ffi.BevyConfig
import br.eng.joaovictor.resort.ffi.BevyEvent
import br.eng.joaovictor.resort.ffi.BevyEventListener
import br.eng.joaovictor.resort.ffi.BevyHost
import br.eng.joaovictor.resort.ffi.BevyInsets as FfiInsets
import br.eng.joaovictor.resort.ffi.BevyPointerPhase
import br.eng.joaovictor.resort.ffi.bevyKmpRespondService
import br.eng.joaovictor.bevykmp.BevyBackend
import br.eng.joaovictor.bevykmp.BevyInsets
import br.eng.joaovictor.bevykmp.PointerPhase
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Adapts the UniFFI-generated [BevyHost] to bevy-kmp's hot-path [BevyBackend]. */
class UniffiBackend(private val host: BevyHost) : BevyBackend {
    override fun attachSurface(handle: Long, width: Int, height: Int, scale: Float) =
        host.attachSurface(handle.toULong(), width.toUInt(), height.toUInt(), scale)

    override fun detachSurface() = host.detachSurface()

    override fun resize(width: Int, height: Int, scale: Float) =
        host.resize(width.toUInt(), height.toUInt(), scale)

    override fun pause() = host.pause()

    override fun resume() = host.resume()

    override fun pointer(id: Long, phase: PointerPhase, x: Float, y: Float) = host.pointer(
        id.toULong(),
        when (phase) {
            PointerPhase.Down -> BevyPointerPhase.DOWN
            PointerPhase.Move -> BevyPointerPhase.MOVE
            PointerPhase.Up -> BevyPointerPhase.UP
            PointerPhase.Cancel -> BevyPointerPhase.CANCEL
        },
        x,
        y,
    )

    override fun setInsets(insets: BevyInsets) = host.setInsets(
        FfiInsets(insets.top, insets.left, insets.bottom, insets.right, insets.imeBottom),
    )

    override fun setFocus(focused: Boolean) = host.setFocus(focused)

    override fun lowMemory() = host.lowMemory()

    override fun key(key: String, pressed: Boolean, text: String?) = host.key(key, pressed, text)
}

/** Everything platform-specific the game asks the shell for (native services). */
interface NativeServices {
    /** Secure storage (Keychain / Keystore) used for the login's refresh token. */
    fun secureGet(key: String): String?

    fun securePut(key: String, value: String): Boolean

    fun secureDelete(key: String)
}

/**
 * Process-wide owner of the Bevy app, created once outside of the UI so the game survives screen
 * changes and Activity recreation.
 */
class GameHost(
    config: HostPaths,
    env: Map<String, String>,
    private val services: NativeServices,
    private val googleAuth: GoogleAuth = NoGoogleAuth,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private val _stage = MutableStateFlow(OnboardingStage())
    /** Onboarding state reported by the game (`onboarding_stage`). */
    val stage: StateFlow<OnboardingStage> = _stage.asStateFlow()

    private val _hud = MutableStateFlow(HudState())
    /** Snapshot for the Compose HUD test (`hud_state`). */
    val hud: StateFlow<HudState> = _hud.asStateFlow()

    /** `COMPOSE_HUD=1`: draw the Compose HUD test over the game. */
    val composeHud: Boolean = env["COMPOSE_HUD"] == "1"

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _fps = MutableStateFlow(0f)
    val fps: StateFlow<Float> = _fps.asStateFlow()

    @kotlin.concurrent.Volatile private var rssMb = 0L
    @kotlin.concurrent.Volatile private var gpuName = ""
    private val statusDir = config.dataDir

    /** Diagnostics for measurements and test scripts: `Documents/status.txt`, once per second. No secrets. */
    private fun writeStatus() {
        val text = "fps=${_fps.value.toInt()} rssMb=$rssMb stage=${_stage.value.stage} gpu=$gpuName\n"
        runCatching { writeFile(statusDir, "status.txt", text.encodeToByteArray()) }
    }

    private lateinit var hostRef: BevyHost

    private val listener = object : BevyEventListener {
        // Called from the Bevy render thread.
        override fun onEvent(event: BevyEvent) {
            when (event) {
                is BevyEvent.Fatal -> _error.value = event.message
                is BevyEvent.Fps -> {
                    _fps.value = event.value
                    writeStatus()
                }
                is BevyEvent.Memory -> rssMb = (event.rssBytes / 1048576u).toLong()
                is BevyEvent.Gpu -> gpuName = event.name
                is BevyEvent.App -> when (event.name) {
                    "onboarding_stage" -> _stage.value = parseStage(event.payload.decodeToString())
                    "hud_state" -> _hud.value = parseHud(event.payload.decodeToString())
                }
                is BevyEvent.ServiceRequest -> handleService(event)
                is BevyEvent.Log -> platformLog(event.level.name, event.target, event.text)
                else -> Unit
            }
        }
    }

    /** request(id, name, payload) -> respond(id, ok, payload). Never log payloads: they hold tokens. */
    private fun handleService(request: BevyEvent.ServiceRequest) {
        val text = request.payload.decodeToString()
        val (ok, reply) = when (request.name) {
            "secure_get" -> services.secureGet(text).let { (it != null) to (it ?: "") }
            "secure_put" -> services.securePut(text.substringBefore('\n'), text.substringAfter('\n')) to ""
            "secure_delete" -> {
                services.secureDelete(text)
                true to ""
            }
            "google_sign_in" -> {
                // Asynchronous (the account picker waits for the player): answered later.
                scope.launch {
                    val answer = when (val r = googleAuth.idToken(text)) {
                        is GoogleResult.Token -> true to r.idToken
                        GoogleResult.Cancelled -> false to "cancelled"
                        GoogleResult.Unavailable -> false to "unavailable"
                        is GoogleResult.Failed -> false to r.reason
                    }
                    respond(request.id, answer.first, answer.second.encodeToByteArray())
                }
                return
            }
            else -> false to "unknown service"
        }
        respond(request.id, ok, reply.encodeToByteArray())
    }

    /**
     * The game asks for services synchronously while it starts (the saved login is read from the
     * Keychain/Keystore), which can happen before the `BevyHost` constructor returned and
     * `hostRef` exists. The global answer function does not need the host object.
     */
    private fun respond(id: ULong, ok: Boolean, payload: ByteArray) {
        if (bevyKmpRespondService(id, ok, payload)) return
        hostRef.respondService(id, ok, payload)
    }

    val host: BevyHost = BevyHost(
        BevyConfig(
            dataDir = config.dataDir,
            cacheDir = config.cacheDir,
            assetDir = config.assetDir,
            env = env,
            backend = BevyBackendKind.AUTO,
            maxFrameDeltaMs = 100u,
        ),
        listener,
    )

    init {
        hostRef = host
    }

    val backend: BevyBackend = UniffiBackend(host)

    fun hudToggleNavigator() = host.sendMessage("hud_toggle_navigator", ByteArray(0))

    fun onboardingGoogle() = host.sendMessage("onboarding_google", ByteArray(0))

    /** Development tool: forget the session and return to the login screen. */
    fun onboardingReset() = host.sendMessage("onboarding_reset", ByteArray(0))

    fun onboardingDeviceLogin() = host.sendMessage("onboarding_device_login", ByteArray(0))

    fun onboardingNickname(nickname: String) = host.sendMessage(
        "onboarding_nickname",
        """{"nickname":"$nickname"}""".encodeToByteArray(),
    )
}

/** Directories the game uses. */
data class HostPaths(val dataDir: String, val cacheDir: String, val assetDir: String)

/** Parses `KEY=VALUE` lines (`#` comments and blanks ignored), like the game's own `parse_env_file`. */
fun parseEnv(content: String): Map<String, String> = content.lineSequence()
    .map { it.trim() }
    .filter { it.isNotEmpty() && !it.startsWith("#") && it.contains('=') }
    .associate { it.substringBefore('=').trim() to it.substringAfter('=').trim() }
    .filterKeys { it.isNotEmpty() }

internal expect fun writeFile(dir: String, name: String, bytes: ByteArray)

internal expect fun platformLog(level: String, target: String, text: String)
