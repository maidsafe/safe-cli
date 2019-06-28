stage('build & test') {
    parallel linux: {
        node('docker') {
            checkout(scm)
            sh("make test")
        }
    },
    windows: {
        node('windows') {
            checkout(scm)
            sh("make test")
        }
    },
    macos: {
        node('osx') {
            checkout(scm)
            sh("make test")
        }
    }
}
