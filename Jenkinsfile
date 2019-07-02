properties([
    parameters([
        string(name: 'ARTIFACTS_BUCKET', defaultValue: 'safe-jenkins-build-artifacts')
    ])
])

stage('build & test') {
    parallel linux: {
        node('docker') {
            checkout(scm)
            sh("make test")
            package_build_artifacts('linux')
            upload_build_artifacts()
        }
    },
    windows: {
        node('windows') {
            checkout(scm)
            sh("make test")
            package_build_artifacts('windows')
            upload_build_artifacts()
        }
    },
    macos: {
        node('osx') {
            checkout(scm)
            sh("make test")
            package_build_artifacts('macos')
            upload_build_artifacts()
        }
    }
}

stage('deploy') {
    node('docker') {
        checkout(scm)
        retrieve_build_artifacts()
        create_github_release()
    }
}

def retrieve_build_artifacts() {
    command = ""
    if (env.CHANGE_ID?.trim()) {
        command += "SAFE_CLI_BRANCH=${env.CHANGE_ID} "
    } else {
        command += "SAFE_CLI_BRANCH=${env.BRANCH_NAME} "
    }
    command += "SAFE_CLI_BUILD_NUMBER=${env.BUILD_NUMBER} "
    command += "make retrieve-all-build-artifacts"
    sh(command)
}

def create_github_release() {
    withCredentials([usernamePassword(
        credentialsId: "github_maidsafe_qa_user_credentials",
        usernameVariable: "GIT_USER",
        passwordVariable: "GIT_PASSWORD")]) {
        sh("git checkout -B ${BRANCH_NAME}")
        create_tag()
    }
}

def create_tag() {
    command = ""
    command += "GIT_USER=$GIT_USER "
    command += "GIT_PASSWORD=$GIT_PASSWORD "
    command += "make tag"
    sh(command)
}

def package_build_artifacts(os) {
    command = ""
    if (env.CHANGE_ID?.trim()) {
        command += "SAFE_CLI_BRANCH=${env.CHANGE_ID} "
    } else {
        command += "SAFE_CLI_BRANCH=${env.BRANCH_NAME} "
    }
    command += "SAFE_CLI_BUILD_NUMBER=${env.BUILD_NUMBER} "
    command += "SAFE_CLI_BUILD_OS=${os} "
    command += "make package-build-artifacts"
    sh(command)
}

def upload_build_artifacts() {
    withAWS(credentials: 'aws_jenkins_build_artifacts_user', region: 'eu-west-2') {
        def artifacts = sh(returnStdout: true, script: 'ls -1 artifacts').trim().split("\\r?\\n")
        for (artifact in artifacts) {
            s3Upload(
                bucket: "${params.ARTIFACTS_BUCKET}",
                file: artifact,
                workingDir: "${env.WORKSPACE}/artifacts",
                acl: 'PublicRead')
        }
    }
}
