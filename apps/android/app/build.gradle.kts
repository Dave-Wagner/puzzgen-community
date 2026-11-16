import java.net.URI
import groovy.json.JsonSlurper

plugins { id("com.android.application") }

dependencies {
    implementation(platform("com.google.firebase:firebase-bom:35.2.0"))
    implementation("com.google.firebase:firebase-auth")
}

fun firebaseConfig(environment: String): Map<String, String> {
    val config=file("../firebase/$environment.json")
    if (!config.isFile) return mapOf("projectId" to "", "appId" to "", "apiKey" to "")
    @Suppress("UNCHECKED_CAST")
    val data=JsonSlurper().parse(config) as Map<String, String>
    require(data.keys.containsAll(listOf("projectId","appId","apiKey"))) { "Incomplete Firebase $environment config" }
    require(data.values.all { it.matches(Regex("[A-Za-z0-9:_-]+")) }) { "Invalid public Firebase configuration" }
    return data.toMap()
}
val devFirebase=firebaseConfig("dev")
val prodFirebase=firebaseConfig("production")
val internalFirebase=firebaseConfig("internal")
require(devFirebase["projectId"].isNullOrEmpty() || prodFirebase["projectId"].isNullOrEmpty() || devFirebase["projectId"] != prodFirebase["projectId"]) { "Dev and production must use separate Firebase projects" }

val debugOrigin = providers.gradleProperty("puzzgenBaseUrl").getOrElse("https://puzzlegen.davewagner.org")
val productionOrigin = providers.gradleProperty("puzzgenProductionBaseUrl").getOrElse("https://puzzlegen.davewagner.org")
val internalOrigin = providers.gradleProperty("puzzgenInternalBaseUrl").getOrElse("https://puzzlegen.davewagner.org")
val versionCodeProperty = providers.gradleProperty("puzzgenVersionCode")
val requestedVersionCode = versionCodeProperty.orNull?.toIntOrNull() ?: 2
val explicitVersionCode = versionCodeProperty.isPresent
val explicitVersionName = providers.gradleProperty("puzzgenVersion").isPresent
val explicitProductionOrigin = providers.gradleProperty("puzzgenProductionBaseUrl").isPresent
require(!versionCodeProperty.isPresent || (versionCodeProperty.get().toIntOrNull() != null && requestedVersionCode in 1..2100000000)) { "Supply a positive Play version code" }
val productionUri = URI(productionOrigin)
require(productionUri.scheme == "https" && productionUri.host != null && productionUri.userInfo == null && productionUri.query == null && productionUri.fragment == null && productionUri.path in listOf("", "/")) { "puzzgenProductionBaseUrl must be a credential-free HTTPS origin" }
val origin = URI(debugOrigin)
val nativeAbi = providers.gradleProperty("puzzgenAbi").getOrElse("arm64-v8a")
require(nativeAbi in listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")) { "Unsupported puzzgenAbi" }
require(origin.scheme in listOf("http", "https") && origin.host != null && origin.userInfo == null && origin.query == null && origin.fragment == null && origin.path in listOf("", "/")) { "puzzgenBaseUrl must be an HTTP(S) origin without credentials or a path" }
require(origin.scheme == "https" || origin.host == "127.0.0.1") { "Cleartext debug traffic is allowed only on ADB-forwarded loopback" }

val legacyMigration = providers.gradleProperty("puzzgenLegacyMigration" ).getOrElse("false" ).toBoolean()
require(!legacyMigration || gradle.startParameter.taskNames.none { it.contains("release", ignoreCase = true) || it.contains("internal", ignoreCase = true) }) { "Legacy identity is debug migration only" }

android {
    namespace = "org.davewagner.puzzgen"
    compileSdk = 36
    defaultConfig {
        ndk { abiFilters += nativeAbi }
        applicationId = if (legacyMigration) "org.davewagner.puzzgen" else "org.davewagner.puzzlegen.beaconandbough"
        minSdk = 26
        targetSdk = 36
        versionCode = requestedVersionCode
        versionName = providers.gradleProperty("puzzgenVersion").getOrElse("0.2.0-native-dev")
    }
    buildFeatures { buildConfig = true; resValues = true }
    sourceSets {
        getByName("main").assets.directories.add(file("../game/public").absolutePath)
        getByName("main").jniLibs.directories.add(file("../native/jniLibs").absolutePath)
    }
    packaging { jniLibs { useLegacyPackaging = false } }
    buildTypes {
        debug {
            applicationIdSuffix = ".dev"
            versionNameSuffix = "-local"
            buildConfigField("String", "BASE_URL", "\"${debugOrigin.trimEnd('/')}\"")
            resValue("string", "app_name", "Beacon & Bough Dev")
            for ((key,value) in devFirebase) buildConfigField("String", "FIREBASE_"+key.replace(Regex("([a-z])([A-Z])"),"$1_$2").uppercase(), "\"$value\"")
        }
        release {
            isMinifyEnabled = false
            buildConfigField("String", "BASE_URL", "\"${productionOrigin.trimEnd('/')}\"")
            resValue("string", "app_name", "Beacon & Bough")
            for ((key,value) in prodFirebase) buildConfigField("String", "FIREBASE_"+key.replace(Regex("([a-z])([A-Z])"),"$1_$2").uppercase(), "\"$value\"")
        }
        create("internal") {
            initWith(getByName("release"))
            matchingFallbacks += listOf("release")
            isDebuggable = false
            signingConfig = null
            versionNameSuffix = "-internal"
            buildConfigField("String", "BASE_URL", "\"${internalOrigin.trimEnd('/')}\"")
            resValue("string", "app_name", "Beacon & Bough Preview")
            for ((key,value) in internalFirebase) buildConfigField("String", "FIREBASE_"+key.replace(Regex("([a-z])([A-Z])"),"$1_$2").uppercase(), "\"$value\"")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// These checks run only when producing distributable artifacts. Compilation/lint
// remain available before environment registration and signing are reviewed.
tasks.register("validateInternalDistribution")
tasks.register("validateProductionDistribution")
tasks.configureEach {
    val distributionEnvironment = when(name) {
        "packageInternal", "packageInternalBundle", "bundleInternal", "validateInternalDistribution" -> "internal"
        "packageRelease", "packageReleaseBundle", "bundleRelease", "validateProductionDistribution" -> "production"
        else -> null
    }
    if(distributionEnvironment != null && !name.startsWith("validate")) {
        dependsOn(if(distributionEnvironment == "internal") "validateInternalDistribution" else "validateProductionDistribution")
    }
    if(distributionEnvironment != null) doFirst {
        require(!legacyMigration) { "Legacy identity is debug migration only" }
        require(explicitVersionCode && requestedVersionCode > 2) { "Distribution requires explicit -PpuzzgenVersionCode greater than the existing local code 2; also check Play's highest used code" }
        require(explicitVersionName) { "Distribution requires explicit -PpuzzgenVersion" }
        val config=if(distributionEnvironment == "internal") internalFirebase else prodFirebase
        require(config.values.all { it.isNotEmpty() }) { "Register and provide Firebase $distributionEnvironment configuration first" }
        if(distributionEnvironment == "internal") {
            val uri=URI(internalOrigin)
            require(uri.scheme == "https" && uri.host != null && uri.userInfo == null && uri.query == null && uri.fragment == null && uri.path in listOf("", "/")) { "Internal testing requires a credential-free HTTPS origin" }
            require(config["projectId"] == devFirebase["projectId"] && config["projectId"] != prodFirebase["projectId"]) { "Internal testing must use the testing Firebase project" }
            require(config["appId"] != devFirebase["appId"] && config["appId"] != prodFirebase["appId"]) { "Internal testing needs its own Firebase Android registration for the final application ID" }
        } else {
            require(explicitProductionOrigin && productionOrigin.trimEnd('/') != debugOrigin.trimEnd('/')) { "Production distribution requires an explicit production origin separate from development" }
            require(config["projectId"] != devFirebase["projectId"]) { "Production must not trust the testing project" }
        }
    }
}

val nativeLibrary = layout.projectDirectory.file("../native/jniLibs/$nativeAbi/libpuzzgen_native.so")
tasks.named("preBuild") {
    inputs.file(nativeLibrary)
    val libraryFile = nativeLibrary.asFile
    doFirst {
        require(libraryFile.isFile) { "Build native Rust first with scripts/Build-Android.ps1." }
    }
}
