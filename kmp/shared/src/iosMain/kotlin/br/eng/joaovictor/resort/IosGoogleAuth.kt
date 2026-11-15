package br.eng.joaovictor.resort

import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * Implemented in Swift (GoogleSignIn-iOS) and registered in [IosGoogleRegistry] before the first
 * screen. Exactly one of [callback]'s arguments is set: the ID token, or an error (`cancelled`
 * for a dismissed sheet).
 */
interface GoogleSignInProvider {
    fun signIn(serverClientId: String, callback: (idToken: String?, error: String?) -> Unit)
}

object IosGoogleRegistry {
    var provider: GoogleSignInProvider? = null
}

class IosGoogleAuth : GoogleAuth {
    override suspend fun idToken(serverClientId: String): GoogleResult {
        val provider = IosGoogleRegistry.provider ?: return GoogleResult.Unavailable
        return suspendCancellableCoroutine { continuation ->
            provider.signIn(serverClientId) { token, error ->
                val result = when {
                    token != null -> GoogleResult.Token(token)
                    error == "cancelled" -> GoogleResult.Cancelled
                    else -> GoogleResult.Failed(error ?: "no token")
                }
                if (continuation.isActive) continuation.resume(result)
            }
        }
    }
}
