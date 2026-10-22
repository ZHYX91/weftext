plugins { id("com.android.library"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "org.weftext.android.host"
    compileSdk = 35
    buildToolsVersion = "35.0.0"
    defaultConfig { minSdk = 30; testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner" }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    lint {
        abortOnError = true; warningsAsErrors = true
        // Version availability is not correctness: this build deliberately targets the installed SDK 35.
        disable += "GradleDependency"
    }
}
kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }
dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
}
