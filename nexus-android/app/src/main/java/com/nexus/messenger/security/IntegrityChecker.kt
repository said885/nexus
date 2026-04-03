package com.nexus.messenger.security

import android.app.Activity
import android.content.Context
import android.os.Build
import android.os.Debug
import androidx.appcompat.app.AlertDialog
import java.io.File
import java.net.InetSocketAddress
import java.net.Socket

enum class ThreatLevel : Comparable<ThreatLevel> {
    NONE, LOW, MEDIUM, HIGH, CRITICAL
}

object IntegrityChecker {

    fun performFullCheck(context: Context): ThreatLevel {
        val checks = listOf(
            checkSuperUserBinaries(),
            checkDangerousPaths(),
            checkMagisk(),
            checkBuildTags(),
            checkSeLinux(),
            checkEmulator(),
            checkDebugger(),
            checkFridaPort(),
        )
        return checks.maxOrNull() ?: ThreatLevel.NONE
    }

    // -------------------------------------------------------------------------
    // Individual checks
    // -------------------------------------------------------------------------

    private fun checkSuperUserBinaries(): ThreatLevel {
        val dangerousPaths = listOf(
            "/system/bin/su",
            "/system/xbin/su",
            "/sbin/su",
            "/data/local/xbin/su",
            "/data/local/bin/su",
            "/system/sd/xbin/su",
            "/system/bin/failsafe/su",
            "/data/local/su",
            "/su/bin/su",
            "/magisk/.core/bin/su",
            "/system/app/Superuser.apk",
            "/system/etc/init.d/99SuperSUDaemon",
            "/dev/com.koushikdutta.superuser.daemon/",
        )
        return if (dangerousPaths.any { File(it).exists() }) ThreatLevel.HIGH else ThreatLevel.NONE
    }

    private fun checkDangerousPaths(): ThreatLevel {
        val paths = listOf(
            "/system/xbin/busybox",
            "/system/bin/busybox",
            "/sbin/busybox",
            "/data/local/bin/busybox",
            "/xbin/busybox",
        )
        return if (paths.any { File(it).exists() }) ThreatLevel.MEDIUM else ThreatLevel.NONE
    }

    private fun checkMagisk(): ThreatLevel {
        val magiskPaths = listOf(
            "/sbin/.magisk",
            "/sbin/.core/mirror",
            "/sbin/.core/img",
            "/data/adb/magisk",
            "/data/adb/ksu",
            "/system/addon.d/99-magisk.sh",
            "/cache/.disable_magisk",
        )
        if (magiskPaths.any { File(it).exists() }) return ThreatLevel.HIGH

        // Check Magisk-related packages via build props
        val magiskProps = listOf("ro.magisk.version", "ro.boot.verifiedbootstate")
        for (prop in magiskProps) {
            try {
                val process = Runtime.getRuntime().exec(arrayOf("getprop", prop))
                val result = process.inputStream.bufferedReader().readText().trim()
                if (result.isNotEmpty() && prop == "ro.magisk.version") return ThreatLevel.HIGH
            } catch (e: Exception) {
                // Ignored
            }
        }

        return ThreatLevel.NONE
    }

    private fun checkBuildTags(): ThreatLevel {
        val tags = Build.TAGS ?: ""
        return if (
            tags.contains("test-keys") ||
            Build.TYPE == "userdebug" ||
            Build.TYPE == "eng"
        ) ThreatLevel.HIGH else ThreatLevel.NONE
    }

    private fun checkSeLinux(): ThreatLevel {
        return try {
            val process = Runtime.getRuntime().exec(arrayOf("getenforce"))
            val result = process.inputStream.bufferedReader().readText().trim()
            if (result.equals("Permissive", ignoreCase = true)) ThreatLevel.HIGH
            else ThreatLevel.NONE
        } catch (e: Exception) {
            ThreatLevel.NONE
        }
    }

    private fun checkEmulator(): ThreatLevel {
        var score = 0

        // Check QEMU properties
        val qemuProps = listOf(
            "ro.kernel.qemu",
            "ro.product.model",
            "ro.hardware",
        )
        val emulatorIndicators = listOf(
            "goldfish", "ranchu", "generic", "unknown", "vbox"
        )

        for (prop in qemuProps) {
            try {
                val process = Runtime.getRuntime().exec(arrayOf("getprop", prop))
                val result = process.inputStream.bufferedReader().readText().trim().lowercase()
                if (emulatorIndicators.any { result.contains(it) }) score++
            } catch (e: Exception) {
                // Ignored
            }
        }

        // Check build model
        val model = Build.MODEL.lowercase()
        if (model.contains("sdk") || model.contains("emulator") ||
            model.contains("android sdk built for") || model.contains("genymotion")) {
            score += 2
        }

        // Check manufacturer
        val manufacturer = Build.MANUFACTURER.lowercase()
        if (manufacturer == "unknown" || manufacturer.contains("genymotion")) score++

        // Check hardware
        val hardware = Build.HARDWARE.lowercase()
        if (hardware.contains("goldfish") || hardware.contains("ranchu") || hardware.contains("vbox")) score += 2

        // Check fingerprint
        val fingerprint = Build.FINGERPRINT.lowercase()
        if (fingerprint.contains("generic") || fingerprint.contains("unknown") ||
            fingerprint.contains("emulator") || fingerprint.startsWith("google/sdk")) {
            score++
        }

        return when {
            score >= 4 -> ThreatLevel.HIGH
            score >= 2 -> ThreatLevel.MEDIUM
            score >= 1 -> ThreatLevel.LOW
            else -> ThreatLevel.NONE
        }
    }

    private fun checkDebugger(): ThreatLevel {
        return if (Debug.isDebuggerConnected() || Debug.waitingForDebugger()) {
            ThreatLevel.CRITICAL
        } else {
            ThreatLevel.NONE
        }
    }

    private fun checkFridaPort(): ThreatLevel {
        // Attempt connection to Frida default port (27042) and secondary (27043)
        val fridaPorts = listOf(27042, 27043)
        for (port in fridaPorts) {
            try {
                Socket().use { socket ->
                    socket.soTimeout = 300
                    socket.connect(InetSocketAddress("127.0.0.1", port), 300)
                    // Connection succeeded — Frida is likely running
                    return ThreatLevel.HIGH
                }
            } catch (e: Exception) {
                // Expected: connection refused means Frida not present
            }
        }
        return ThreatLevel.NONE
    }

    // -------------------------------------------------------------------------
    // Threat response
    // -------------------------------------------------------------------------

    fun handleThreat(level: ThreatLevel, activity: Activity) {
        when (level) {
            ThreatLevel.NONE -> Unit
            ThreatLevel.LOW -> showSecurityWarning(activity)
            ThreatLevel.MEDIUM -> enableRestrictedMode(activity)
            ThreatLevel.HIGH -> requireReauthentication(activity)
            ThreatLevel.CRITICAL -> triggerSecureWipe(activity)
        }
    }

    private fun showSecurityWarning(activity: Activity) {
        if (activity.isFinishing || activity.isDestroyed) return
        activity.runOnUiThread {
            AlertDialog.Builder(activity)
                .setTitle("Security Warning")
                .setMessage("A potential security concern was detected on this device. Your messages remain encrypted, but be cautious.")
                .setPositiveButton("Understood") { d, _ -> d.dismiss() }
                .setCancelable(false)
                .show()
        }
    }

    private fun enableRestrictedMode(activity: Activity) {
        if (activity.isFinishing || activity.isDestroyed) return
        activity.runOnUiThread {
            AlertDialog.Builder(activity)
                .setTitle("Restricted Mode")
                .setMessage("Security threats detected. Nexus is running in restricted mode. Some features are limited to protect your data.")
                .setPositiveButton("Continue") { d, _ -> d.dismiss() }
                .setCancelable(false)
                .show()
        }
    }

    private fun requireReauthentication(activity: Activity) {
        if (activity.isFinishing || activity.isDestroyed) return
        activity.runOnUiThread {
            AlertDialog.Builder(activity)
                .setTitle("Security Alert")
                .setMessage("High-severity security threats detected on this device (rooted, test keys, or analysis tools detected). Access is restricted.")
                .setPositiveButton("Lock App") { _, _ ->
                    activity.finish()
                }
                .setCancelable(false)
                .show()
        }
    }

    fun triggerSecureWipe(context: Context) {
        // 1. Clear all app shared preferences
        val prefsDir = File(context.applicationInfo.dataDir, "shared_prefs")
        prefsDir.listFiles()?.forEach { it.delete() }

        // 2. Delete all app databases
        context.databaseList().forEach { dbName ->
            context.deleteDatabase(dbName)
        }

        // 3. Clear cache and files
        context.cacheDir.deleteRecursively()
        context.filesDir.deleteRecursively()
        context.getExternalFilesDir(null)?.deleteRecursively()

        // 4. If this is an Activity, finish it
        if (context is Activity) {
            context.finishAffinity()
        }
    }
}
