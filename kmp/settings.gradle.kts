rootProject.name = "resort-kmp"

pluginManagement {
    repositories { google(); mavenCentral(); gradlePluginPortal() }
}
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        mavenLocal()
        // bevy-kmp's Kotlin library (br.eng.joaovictor:bevy-kmp) from the GitLab Maven registry of the
        // bevy_kmp project (id 88044695). The registry is private: set GITLAB_TOKEN (read access) and,
        // for personal/project access tokens, GITLAB_TOKEN_HEADER=Private-Token (Deploy-Token is the
        // default for deploy tokens). The token is read from the environment only.
        System.getenv("GITLAB_TOKEN")?.takeIf { it.isNotBlank() }?.let { token ->
            maven {
                url = uri("https://gitlab.com/api/v4/projects/88044695/packages/maven")
                credentials(HttpHeaderCredentials::class) {
                    name = System.getenv("GITLAB_TOKEN_HEADER") ?: "Deploy-Token"
                    value = token
                }
                authentication { create<HttpHeaderAuthentication>("header") }
                content { includeGroup("br.eng.joaovictor") }
            }
        }
    }
}

// With the bevy_kmp checkout next to this repository the library is built from source (included
// build, same coordinates); otherwise it is resolved from mavenLocal / the registry above.
file("../../bevy_kmp/kmp/bevy-kmp").takeIf { it.exists() }?.let { includeBuild(it) }

include(":shared", ":androidApp")
