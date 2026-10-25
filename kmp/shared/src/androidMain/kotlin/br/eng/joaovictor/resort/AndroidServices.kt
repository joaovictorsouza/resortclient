package br.eng.joaovictor.resort

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import android.util.Log
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Secret store backed by the Android Keystore: AES-GCM ciphertext in SharedPreferences. */
class AndroidServices(context: Context) : NativeServices {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    @Synchronized
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        if (store.containsAlias(ALIAS)) return store.getKey(ALIAS, null) as SecretKey
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    override fun securePut(key: String, value: String): Boolean = try {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val sealed = cipher.doFinal(value.toByteArray())
        val blob = cipher.iv + sealed
        prefs.edit().putString(key, Base64.encodeToString(blob, Base64.NO_WRAP)).commit()
    } catch (e: Exception) {
        Log.w(TAG, "put failed", e)
        false
    }

    override fun secureGet(key: String): String? {
        val encoded = prefs.getString(key, null) ?: return null
        return try {
            val blob = Base64.decode(encoded, Base64.NO_WRAP)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, blob, 0, 12))
            String(cipher.doFinal(blob, 12, blob.size - 12))
        } catch (e: Exception) {
            Log.w(TAG, "get failed, dropping the entry", e)
            prefs.edit().remove(key).commit()
            null
        }
    }

    override fun secureDelete(key: String) {
        prefs.edit().remove(key).commit()
    }

    private companion object {
        const val TAG = "AndroidServices"
        const val ALIAS = "resort_secure_store"
        const val PREFS = "resort_secure_store"
    }
}
