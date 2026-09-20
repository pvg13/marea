import org.gradle.api.tasks.bundling.AbstractArchiveTask

// Bundled into consuming apps by `#[manganis::ffi("src/android")]` in
// `notify.rs`, the same mechanism WaveSyncDB uses to ship its FCM service.
// dx reads it out of the compiled binary's symbol table, so an app that turns
// the `local-notify` feature on gets the receivers and the manifest entries
// without copying a file.
plugins {
    id("com.android.library") version "8.7.0"
    kotlin("android") version "2.0.20"
}

android {
    namespace = "dev.marea.notify"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        targetSdk = 34
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        getByName("release") { isMinifyEnabled = false }
        getByName("debug") { isMinifyEnabled = false }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    // NotificationCompat and NotificationManagerCompat only. Deliberately no
    // WorkManager and no Firebase: a local reminder needs neither, and a
    // framework crate should not drag either into an app that says nothing
    // about push.
    implementation("androidx.core:core-ktx:1.13.1")
}

tasks.withType<AbstractArchiveTask>().configureEach {
    archiveBaseName.set("marea-notify")
}
