plugins {
    id("com.android.library") version "8.13.2" apply false
    id("org.jetbrains.kotlin.android") version "2.3.21" apply false
}
val ownedBuildRoot = providers.gradleProperty("weftextBuildRoot").orNull
    ?: error("Use scripts/android.py with an explicitly owned state directory")
allprojects { layout.buildDirectory.set(file(ownedBuildRoot).resolve(project.name)) }
