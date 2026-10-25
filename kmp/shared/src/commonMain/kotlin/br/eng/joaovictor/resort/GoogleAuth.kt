package br.eng.joaovictor.resort

/** Outcome of a native Google sign-in. Tokens are never logged. */
sealed interface GoogleResult {
    /** The Google ID token whose audience is the server client id (what Nakama expects). */
    data class Token(val idToken: String) : GoogleResult

    data object Cancelled : GoogleResult

    data object Unavailable : GoogleResult

    data class Failed(val reason: String) : GoogleResult
}

/** One interface for the whole app; the implementations are small and platform-specific. */
interface GoogleAuth {
    suspend fun idToken(serverClientId: String): GoogleResult
}

/** Used where no native sign-in exists. */
object NoGoogleAuth : GoogleAuth {
    override suspend fun idToken(serverClientId: String): GoogleResult = GoogleResult.Unavailable
}
