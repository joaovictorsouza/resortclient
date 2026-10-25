package br.eng.joaovictor.resort

import android.app.Activity
import android.util.Log
import androidx.credentials.CredentialManager
import androidx.credentials.CustomCredential
import androidx.credentials.GetCredentialRequest
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import java.lang.ref.WeakReference

/** The Activity that can present the account picker; set by the app's Activity. Never retained strongly. */
object ActivityHolder {
    private var ref: WeakReference<Activity>? = null

    var current: Activity?
        get() = ref?.get()
        set(value) {
            ref = value?.let { WeakReference(it) }
        }
}

/** Google sign-in through Credential Manager; the result is the ID token for [serverClientId]. */
class AndroidGoogleAuth : GoogleAuth {
    override suspend fun idToken(serverClientId: String): GoogleResult {
        val activity = ActivityHolder.current ?: return GoogleResult.Failed("no activity to present the sign-in")
        return try {
            val option = GetGoogleIdOption.Builder()
                .setFilterByAuthorizedAccounts(false)
                .setServerClientId(serverClientId)
                .build()
            val request = GetCredentialRequest.Builder().addCredentialOption(option).build()
            val credential = CredentialManager.create(activity).getCredential(activity, request).credential
            if (credential is CustomCredential &&
                credential.type == GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL
            ) {
                GoogleResult.Token(GoogleIdTokenCredential.createFrom(credential.data).idToken)
            } else {
                GoogleResult.Failed("unexpected credential type ${credential.type}")
            }
        } catch (e: GetCredentialCancellationException) {
            GoogleResult.Cancelled
        } catch (e: GetCredentialException) {
            Log.w("GoogleAuth", "Credential Manager failed: ${e.type}")
            GoogleResult.Failed("${e.type}: ${e.message}")
        }
    }
}
