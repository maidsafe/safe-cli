properties([
    parameters([
        string(name: 'ARTIFACTS_BUCKET', defaultValue: 'safe-jenkins-build-artifacts'),
        string(name: 'DEPLOY_BUCKET', defaultValue: 'safe-cli')
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
        sh("git fetch --tags --force")
        retrieve_build_artifacts()
        if (version_change_commit()) {
            version = sh(
                returnStdout: true,
                script: "grep '^version' < Cargo.toml | head -n 1 | awk '{ print \$3 }' | sed 's/\"//g'").trim()
            package_artifacts_for_deploy(true)
            create_tag(version)
            create_github_release(version)
        } else {
            package_artifacts_for_deploy(false)
            upload_deploy_artifacts()
        }
    }
}

def version_change_commit() {
    short_commit_hash = sh(
        returnStdout: true,
        script: "git log -n 1 --pretty=format:'%h'").trim()
    message = sh(
        returnStdout: true,
        script: "git log --format=%B -n 1 ${short_commit_hash}").trim()
    return message.startsWith("Version change")
}

def package_artifacts_for_deploy(version_commit) {
    if (version_commit) {
        sh("make package-version-artifacts-for-deploy")
    } else {
        sh("make package-commit_hash-artifacts-for-deploy")
    }
}

def create_tag(version) {
    withCredentials([usernamePassword(
        credentialsId: "github_maidsafe_qa_user_credentials",
        usernameVariable: "GIT_USER",
        passwordVariable: "GIT_PASSWORD")]) {
        sh("git config --global user.name \$GIT_USER")
        sh("git config --global user.email qa@maidsafe.net")
        sh("git config credential.username \$GIT_USER")
        sh("git config credential.helper '!f() { echo password=\$GIT_PASSWORD; }; f'")
        sh("git tag -a ${version} -m 'Creating tag for ${version}'")
        sh("GIT_ASKPASS=true git push origin --tags")
    }
}

def create_github_release(version) {
    withCredentials([usernamePassword(
        credentialsId: "github_maidsafe_token_credentials",
        usernameVariable: "GITHUB_USER",
        passwordVariable: "GITHUB_TOKEN")]) {
        sh("make deploy-github-release")
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

def upload_deploy_artifacts() {
    withAWS(credentials: 'aws_jenkins_deploy_artifacts_user', region: 'eu-west-2') {
        def artifacts = sh(returnStdout: true, script: 'ls -1 deploy').trim().split("\\r?\\n")
        for (artifact in artifacts) {
            s3Upload(
                bucket: "${params.DEPLOY_BUCKET}",
                file: artifact,
                workingDir: "${env.WORKSPACE}/deploy",
                acl: 'PublicRead')
        }
    }
}
