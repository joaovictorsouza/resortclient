package br.eng.joaovictor.resort;

import android.app.Activity;
import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import android.util.Log;

import java.nio.charset.StandardCharsets;
import java.security.KeyStore;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/**
 * Small secret store (the login's refresh token) for Rust, see crates/game/src/secure_store.rs.
 * Values are encrypted with an AES-GCM key that lives in the Android Keystore and never leaves
 * it; only the ciphertext is written to SharedPreferences.
 */
public final class SecureStore {
    private static final String TAG = "SecureStore";
    private static final String KEY_ALIAS = "resort_secure_store";
    private static final String PREFS = "resort_secure_store";
    private static final int GCM_TAG_BITS = 128;

    private SecureStore() {}

    private static synchronized SecretKey key() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        if (store.containsAlias(KEY_ALIAS)) {
            return (SecretKey) store.getKey(KEY_ALIAS, null);
        }
        KeyGenerator generator =
                KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        generator.init(new KeyGenParameterSpec.Builder(
                KEY_ALIAS, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build());
        return generator.generateKey();
    }

    private static SharedPreferences prefs(Activity activity) {
        return activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    /** Stores {@code value} under {@code name}; returns false if it could not be encrypted. */
    public static boolean put(Activity activity, String name, String value) {
        try {
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, key());
            byte[] iv = cipher.getIV();
            byte[] sealed = cipher.doFinal(value.getBytes(StandardCharsets.UTF_8));
            byte[] blob = new byte[iv.length + sealed.length];
            System.arraycopy(iv, 0, blob, 0, iv.length);
            System.arraycopy(sealed, 0, blob, iv.length, sealed.length);
            return prefs(activity).edit()
                    .putString(name, Base64.encodeToString(blob, Base64.NO_WRAP))
                    .commit();
        } catch (Exception e) {
            Log.w(TAG, "put failed", e);
            return false;
        }
    }

    /** Returns the stored value, or null when absent or no longer decryptable (e.g. restored backup). */
    public static String get(Activity activity, String name) {
        String encoded = prefs(activity).getString(name, null);
        if (encoded == null) {
            return null;
        }
        try {
            byte[] blob = Base64.decode(encoded, Base64.NO_WRAP);
            // AES-GCM from the Keystore uses a 12-byte IV.
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, key(), new GCMParameterSpec(GCM_TAG_BITS, blob, 0, 12));
            byte[] plain = cipher.doFinal(blob, 12, blob.length - 12);
            return new String(plain, StandardCharsets.UTF_8);
        } catch (Exception e) {
            Log.w(TAG, "get failed, dropping the entry", e);
            prefs(activity).edit().remove(name).commit();
            return null;
        }
    }

    public static void delete(Activity activity, String name) {
        prefs(activity).edit().remove(name).commit();
    }
}
