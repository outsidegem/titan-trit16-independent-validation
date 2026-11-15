plugins {
    java
}

group = "com.titan.trit16"
version = "0.1.0"

java {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
}

tasks.withType<JavaCompile> {
    options.encoding = "UTF-8"
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Werror"))
}

val nativeLibDir = file("${projectDir}/../ffi/target/release")
val jniArm64Dir = file("${projectDir}/src/main/jniLibs/arm64-v8a")

tasks.register<Exec>("runJniTests") {
    dependsOn("testClasses")
    val cp = sourceSets["main"].output.classesDirs.asPath +
        File.pathSeparator +
        sourceSets["test"].output.classesDirs.asPath
    val soFile = file("${jniArm64Dir}/libtitan_trit16_ffi.so")
    commandLine(
        "java",
        "-ea",
        "-classpath",
        cp,
        "com.titan.trit16.Trit16JniTest",
        soFile.absolutePath
    )
}

tasks.register<Zip>("assembleAar") {
    dependsOn("jar")
    archiveFileName.set("titan-trit16-${project.version}.aar")
    destinationDirectory.set(file("${layout.buildDirectory.get()}/outputs/aar"))

    from("${projectDir}/src/main/AndroidManifest.xml")
    from(tasks.named<Jar>("jar").get().archiveFile) {
        rename { "classes.jar" }
    }
    from("${projectDir}/src/main/jniLibs") {
        into("jni")
    }
}
