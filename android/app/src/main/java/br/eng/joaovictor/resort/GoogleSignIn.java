package br.eng.joaovictor.resort;

import android.app.Activity;
import android.os.CancellationSignal;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.credentials.Credential;
import androidx.credentials.CredentialManager;
import androidx.credentials.CredentialManagerCallback;
import androidx.credentials.CustomCredential;
import androidx.credentials.GetCredentialRequest;
import androidx.credentials.GetCredentialResponse;
import androidx.credentials.exceptions.GetCredentialCancellationException;
import androidx.credentials.exceptions.GetCredentialException;

import com.google.android.libraries.identity.googleid.GetGoogleIdOption;
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential;

import java.util.concurrent.Executor;

/**
 * Google sign-in through Credential Manager. Rust calls {@link #start} (see
 * crates/game/src/android_google.rs) and receives the outcome through {@link #nativeOnResult}.
 */
public final class GoogleSignIn {
    private static final String TAG = "GoogleSignIn";

    static final int STATUS_TOKEN = 0;
    static final int STATUS_CANCELLED = 1;
    static final int STATUS_FAILED = 2;

    private GoogleSignIn() {}

    /** Reports the outcome to Rust: the id_token for {@code STATUS_TOKEN}, else a message. */
    private static native void nativeOnResult(int status, String value);

    public static void start(final Activity activity, final String serverClientId) {
        activity.runOnUiThread(() -> request(activity, serverClientId));
    }

    private static void request(Activity activity, String serverClientId) {
        try {
            GetGoogleIdOption option = new GetGoogleIdOption.Builder()
                    .setFilterByAuthorizedAccounts(false)
                    .setServerClientId(serverClientId)
                    .build();
            GetCredentialRequest request = new GetCredentialRequest.Builder()
                    .addCredentialOption(option)
                    .build();
            Executor executor = activity.getMainExecutor();
            CredentialManager.create(activity).getCredentialAsync(
                    activity,
                    request,
                    new CancellationSignal(),
                    executor,
                    new CredentialManagerCallback<GetCredentialResponse, GetCredentialException>() {
                        @Override
                        public void onResult(GetCredentialResponse response) {
                            handle(response.getCredential());
                        }

                        @Override
                        public void onError(@NonNull GetCredentialException e) {
                            Log.w(TAG, "Credential Manager failed", e);
                            if (e instanceof GetCredentialCancellationException) {
                                nativeOnResult(STATUS_CANCELLED, "cancelled");
                            } else {
                                nativeOnResult(STATUS_FAILED, e.getType() + ": " + e.getMessage());
                            }
                        }
                    });
        } catch (RuntimeException e) {
            Log.e(TAG, "Could not start Google sign-in", e);
            nativeOnResult(STATUS_FAILED, String.valueOf(e));
        }
    }

    private static void handle(Credential credential) {
        if (credential instanceof CustomCredential
                && GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL.equals(credential.getType())) {
            GoogleIdTokenCredential google =
                    GoogleIdTokenCredential.createFrom(((CustomCredential) credential).getData());
            nativeOnResult(STATUS_TOKEN, google.getIdToken());
        } else {
            nativeOnResult(STATUS_FAILED, "unexpected credential type " + credential.getType());
        }
    }
}
