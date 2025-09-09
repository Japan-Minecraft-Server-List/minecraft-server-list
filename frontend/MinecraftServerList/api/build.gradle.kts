
plugins {
    kotlin("jvm") version "2.2.20-RC2"
    kotlin("plugin.serialization") version "2.2.20-RC2"
}

group = "org.lang.tyml"
version = "1.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
}
