@file:OptIn(ExperimentalForeignApi::class, ExperimentalUnsignedTypes::class)

package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.game.NativeServices
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.readBytes
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import platform.CoreFoundation.CFDataCreate
import platform.CoreFoundation.CFDataGetBytePtr
import platform.CoreFoundation.CFDataGetLength
import platform.CoreFoundation.CFDataRef
import platform.CoreFoundation.CFDictionaryAddValue
import platform.CoreFoundation.CFDictionaryCreateMutable
import platform.CoreFoundation.CFRelease
import platform.CoreFoundation.CFStringCreateWithCString
import platform.CoreFoundation.CFTypeRefVar
import platform.CoreFoundation.kCFBooleanTrue
import platform.CoreFoundation.kCFStringEncodingUTF8
import platform.CoreFoundation.kCFTypeDictionaryKeyCallBacks
import platform.CoreFoundation.kCFTypeDictionaryValueCallBacks
import platform.Security.SecItemAdd
import platform.Security.SecItemCopyMatching
import platform.Security.SecItemDelete
import platform.Security.errSecSuccess
import platform.Security.kSecAttrAccount
import platform.Security.kSecAttrService
import platform.Security.kSecClass
import platform.Security.kSecClassGenericPassword
import platform.Security.kSecMatchLimit
import platform.Security.kSecMatchLimitOne
import platform.Security.kSecReturnData
import platform.Security.kSecValueData

/** Secret store backed by the iOS Keychain (generic passwords, service = bundle id). */
class IosServices : NativeServices {
    private fun identity(key: String) = CFDictionaryCreateMutable(
        null, 0, kCFTypeDictionaryKeyCallBacks.ptr, kCFTypeDictionaryValueCallBacks.ptr,
    ).also {
        val service = CFStringCreateWithCString(null, SERVICE, kCFStringEncodingUTF8)
        val account = CFStringCreateWithCString(null, key, kCFStringEncodingUTF8)
        CFDictionaryAddValue(it, kSecClass, kSecClassGenericPassword)
        CFDictionaryAddValue(it, kSecAttrService, service)
        CFDictionaryAddValue(it, kSecAttrAccount, account)
        CFRelease(service)
        CFRelease(account)
    }

    override fun securePut(key: String, value: String): Boolean {
        secureDelete(key)
        val query = identity(key)
        val bytes = value.encodeToByteArray()
        val data = bytes.asUByteArray().usePinned { CFDataCreate(null, it.addressOf(0), bytes.size.toLong()) }
        CFDictionaryAddValue(query, kSecValueData, data)
        val status = SecItemAdd(query, null)
        CFRelease(data)
        CFRelease(query)
        return status == errSecSuccess
    }

    override fun secureGet(key: String): String? = memScoped {
        val query = identity(key)
        CFDictionaryAddValue(query, kSecReturnData, kCFBooleanTrue)
        CFDictionaryAddValue(query, kSecMatchLimit, kSecMatchLimitOne)
        val result = alloc<CFTypeRefVar>()
        val status = SecItemCopyMatching(query, result.ptr)
        CFRelease(query)
        if (status != errSecSuccess) return null
        val data = result.value as? CFDataRef ?: return null
        val text = CFDataGetBytePtr(data)?.readBytes(CFDataGetLength(data).toInt())?.decodeToString()
        CFRelease(data)
        text
    }

    /** Removes every item this app stored in the Keychain (generic passwords of this service). */
    fun clearAll() {
        val query = CFDictionaryCreateMutable(
            null, 0, kCFTypeDictionaryKeyCallBacks.ptr, kCFTypeDictionaryValueCallBacks.ptr,
        )
        val service = CFStringCreateWithCString(null, SERVICE, kCFStringEncodingUTF8)
        CFDictionaryAddValue(query, kSecClass, kSecClassGenericPassword)
        CFDictionaryAddValue(query, kSecAttrService, service)
        SecItemDelete(query)
        CFRelease(service)
        CFRelease(query)
    }

    override fun secureDelete(key: String) {
        val query = identity(key)
        SecItemDelete(query)
        CFRelease(query)
    }

    private companion object {
        const val SERVICE = "br.eng.joaovictor.resort"
    }
}
