plugins {
    id("com.android.application")
}

android {
    namespace = "com.titan.trit16.consumer"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.titan.trit16.consumer"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner =
            "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters += setOf("arm64-v8a", "x86_64")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    testOptions {
        animationsDisabled = true
    }
}

dependencies {
    implementation(files("libs/titan-trit16-0.1.0.aar"))

    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
}
