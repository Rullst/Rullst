
// Rullst application-owned release signing v1
// Secrets arrive through the environment, never generated files or CLI arguments.
// Keep Gradle configuration caches/build scans private; do not enable debug logs.
val rullstKeystore = System.getenv("RULLST_ANDROID_KEYSTORE")
val rullstKeyAlias = System.getenv("RULLST_ANDROID_KEY_ALIAS")
val rullstStorePassword = System.getenv("RULLST_ANDROID_STORE_PASSWORD")
val rullstKeyPassword = System.getenv("RULLST_ANDROID_KEY_PASSWORD")

android {
    signingConfigs {
        create("rullstRelease") {
            storeFile = rullstKeystore?.let { File(it) }
            keyAlias = rullstKeyAlias
            storePassword = rullstStorePassword
            keyPassword = rullstKeyPassword
        }
    }
    buildTypes.getByName("release") {
        signingConfig = signingConfigs.getByName("rullstRelease")
    }
}

// Fail closed even when the caller bypasses cargo rullst and runs Tauri/Gradle.
// Debug builds retain Android's development-only signing and need no release key.
val validateRullstReleaseSigning = tasks.register("validateRullstReleaseSigning") {
    doLast {
        val required = listOf(rullstKeystore, rullstKeyAlias, rullstStorePassword, rullstKeyPassword)
        if (required.any { it.isNullOrEmpty() }) {
            throw GradleException("Set RULLST_ANDROID_KEYSTORE, RULLST_ANDROID_KEY_ALIAS, RULLST_ANDROID_STORE_PASSWORD and RULLST_ANDROID_KEY_PASSWORD before a release build. Never distribute an unsigned APK.")
        }
        val key = File(rullstKeystore ?: "")
        if (!key.isAbsolute || !key.isFile) {
            throw GradleException("RULLST_ANDROID_KEYSTORE must be an existing absolute keystore path.")
        }
    }
}
tasks.matching { it.name == "preReleaseBuild" }.configureEach {
    dependsOn(validateRullstReleaseSigning)
}
