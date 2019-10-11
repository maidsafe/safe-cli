SHELL := /bin/bash
SAFE_CLI_VERSION := $(shell grep "^version" < Cargo.toml | head -n 1 | awk '{ print $$3 }' | sed 's/\"//g')
USER_ID := $(shell id -u)
GROUP_ID := $(shell id -g)
UNAME_S := $(shell uname -s)
PWD := $(shell echo $$PWD)
UUID := $(shell uuidgen | sed 's/-//g')
S3_BUCKET := safe-jenkins-build-artifacts
SAFE_AUTH_DEFAULT_PORT := 41805
GITHUB_REPO_OWNER := maidsafe
GITHUB_REPO_NAME := safe-cli

build-clean-cli:
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:cli \
		bash -c "rm -rf /target/release && cargo build --release"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	rm -rf target
	cargo build --release --manifest-path=safe-cli/Cargo.toml
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-cli:
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:cli \
		cargo build --release
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	cargo build --release --manifest-path=safe-cli/Cargo.toml
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-clean-ffi:
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi \
		bash -c "rm -rf /target/release && cargo build --release"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	rm -rf target
	cargo build --release --manifest-path safe-ffi/Cargo.toml
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-ffi:
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi \
		cargo build --release --manifest-path safe-ffi/Cargo.toml
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	cargo build --release --manifest-path safe-ffi/Cargo.toml
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-ffi-android-x86_64:
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi-android-x86_64 \
		cargo build --release --manifest-path safe-ffi/Cargo.toml --target=x86_64-linux-android
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
	find target/x86_64-linux-android/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-ffi-android-armv7:
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi-android-armv7 \
		cargo build --release --manifest-path safe-ffi/Cargo.toml --target=armv7-linux-androideabi
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
	find target/armv7-linux-androideabi/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-clean-ffi-android-x86_64:
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi-android-x86_64 \
		bash -c "rm -rf /target && cargo build --release --manifest-path safe-ffi/Cargo.toml --target=x86_64-linux-android"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
	find target/x86_64-linux-android/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-clean-ffi-android-armv7:
	rm -rf target
	rm -rf artifacts
	mkdir artifacts
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:ffi-android-armv7 \
		bash -c "rm -rf /target && cargo build --release --manifest-path safe-ffi/Cargo.toml --target=armv7-linux-androideabi"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
	find target/armv7-linux-androideabi/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-ios-aarch64:
	rm -rf artifacts
	mkdir artifacts
	cargo build --release --manifest-path=safe-ffi/Cargo.toml --target=aarch64-apple-ios
	find target/aarch64-apple-ios/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

build-ios-x86_64:
	rm -rf artifacts
	mkdir artifacts
	cargo build --release --manifest-path=safe-ffi/Cargo.toml --target=x86_64-apple-ios
	find target/x86_64-apple-ios/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

retrieve-ios-build-artifacts:
ifndef SAFE_CLI_BUILD_BRANCH
	@echo "A branch or PR reference must be provided."
	@echo "Please set SAFE_CLI_BUILD_BRANCH to a valid branch or PR reference."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_NUMBER
	@echo "A valid build number must be supplied for the artifacts to be retrieved."
	@echo "Please set SAFE_CLI_BUILD_NUMBER to a valid build number."
	@exit 1
endif
	./scripts/retrieve-build-artifacts.sh "x86_64-apple-ios" "aarch64-apple-ios"

universal-ios-lib: retrieve-ios-build-artifacts
ifneq ($(UNAME_S),Darwin)
	@echo "This target can only be run on macOS"
	@exit 1
endif
ifndef SAFE_CLI_BUILD_BRANCH
	@echo "A branch or PR reference must be provided."
	@echo "Please set SAFE_CLI_BUILD_BRANCH to a valid branch or PR reference."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_NUMBER
	@echo "A valid build number must be supplied for the artifacts to be retrieved."
	@echo "Please set SAFE_CLI_BUILD_NUMBER to a valid build number."
	@exit 1
endif
	mkdir -p artifacts/real/universal
	mkdir -p artifacts/mock/universal
	lipo -create -output artifacts/real/universal/libsafe_ffi.a \
		artifacts/real/x86_64-apple-ios/release/libsafe_ffi.a \
		artifacts/real/aarch64-apple-ios/release/libsafe_ffi.a

strip-artifacts:
ifeq ($(OS),Windows_NT)
	find artifacts -name "safe.exe" -exec strip -x '{}' \;
else ifeq ($(UNAME_S),Darwin)
	find artifacts -name "safe" -exec strip -x '{}' \;
else
	find artifacts -name "safe" -exec strip '{}' \;
endif

build-cli-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:cli
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:cli \
		--build-arg build_type="non-dev" \
		--build-arg build_component="safe-cli" .

push-cli-container:
	docker push maidsafe/safe-cli-build:cli

build-cli-dev-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:cli
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:cli-dev \
		--build-arg build_type="dev" \
		--build-arg build_component="safe-cli" .

push-cli-dev-container:
	docker push maidsafe/safe-cli-build:cli-dev

build-api-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:api
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:api \
		--build-arg build_type="non-dev" \
		--build-arg build_component="safe-api" .

push-api-container:
	docker push maidsafe/safe-cli-build:api

build-ffi-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:ffi
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:ffi \
		--build-arg build_type="non-dev" \
		--build-arg build_component="safe-ffi" .

push-ffi-container:
	docker push maidsafe/safe-cli-build:ffi

build-ffi-dev-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:ffi-dev
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:ffi-dev \
		--build-arg build_type="non-dev" \
		--build-arg build_component="safe-ffi" .

push-ffi-dev-container:
	docker push maidsafe/safe-cli-build:ffi-dev

build-ffi-android-armv7-container:
	rm -rf target/
	docker system prune --force
	docker rmi -f maidsafe/safe-cli-build:ffi-android-armv7
	docker build -f Dockerfile.android.armv7.build \
		-t maidsafe/safe-cli-build:ffi-android-armv7 \
		--build-arg build_type="non-dev" \
		--build-arg target="armv7-linux-androideabi" .

push-ffi-android-armv7-container:
	docker push maidsafe/safe-cli-build:ffi-android-armv7

build-ffi-android-armv7-dev-container:
	rm -rf target/
	docker system prune --force
	docker rmi -f maidsafe/safe-cli-build:ffi-android-armv7-dev
	docker build -f Dockerfile.android.armv7.build \
		-t maidsafe/safe-cli-build:ffi-android-armv7-dev \
		--build-arg build_type="dev" \
		--build-arg target="armv7-linux-androideabi" .

push-ffi-android-armv7-dev-container:
	docker push maidsafe/safe-cli-build:ffi-android-armv7-dev

build-ffi-android-x86_64-container:
	rm -rf target/
	docker system prune --force
	docker rmi -f maidsafe/safe-cli-build:ffi-android-x86_64
	docker build -f Dockerfile.android.x86_64.build \
		-t maidsafe/safe-cli-build:ffi-android-x86_64 \
		--build-arg build_type="non-dev" \
		--build-arg target="x86_64-linux-android" .

push-ffi-android-x86_64-container:
	docker push maidsafe/safe-cli-build:ffi-android-x86_64

build-ffi-android-x86_64-dev-container:
	rm -rf target/
	docker system prune --force
	docker rmi -f maidsafe/safe-cli-build:ffi-android-x86_64-dev
	docker build -f Dockerfile.android.x86_64.build \
		-t maidsafe/safe-cli-build:ffi-android-x86_64-dev \
		--build-arg build_type="dev" \
		--build-arg target="x86_64-linux-android" .

push-ffi-android-x86_64-dev-container:
	docker push maidsafe/safe-cli-build:ffi-android-x86_64-dev

clippy:
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:build \
		/bin/bash -c "cargo clippy --all-targets --all-features -- -D warnings"
else
	cargo clippy --all-targets --all-features -- -D warnings
endif

.ONESHELL:
test-cli:
ifndef SAFE_AUTH_PORT
	$(eval SAFE_AUTH_PORT := ${SAFE_AUTH_DEFAULT_PORT})
endif
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		-e RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} \
		-e SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
		maidsafe/safe-cli-build:cli-dev \
		bash -c "./resources/test-scripts/run-auth-daemon && ./resources/test-scripts/cli-tests"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	$(eval MOCK_VAULT_PATH := ~/safe_auth-${SAFE_AUTH_PORT})
	RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
	   ./resources/test-scripts/run-auth-daemon
	RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
	   ./resources/test-scripts/cli-tests
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

.ONESHELL:
test-api:
ifndef SAFE_AUTH_PORT
	$(eval SAFE_AUTH_PORT := ${SAFE_AUTH_DEFAULT_PORT})
endif
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		-e RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} \
		-e SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
		maidsafe/safe-cli-build:api \
		bash -c "./resources/test-scripts/run-auth-daemon && ./resources/test-scripts/api-tests"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	$(eval MOCK_VAULT_PATH := ~/safe_auth-${SAFE_AUTH_PORT})
	RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
		./resources/test-scripts/run-auth-daemon
	RANDOM_PORT_NUMBER=${SAFE_AUTH_PORT} SAFE_MOCK_VAULT_PATH=${MOCK_VAULT_PATH} \
		./resources/test-scripts/api-tests
endif
	find target/release -maxdepth 1 -type f -exec cp '{}' artifacts \;

package-build-artifacts:
ifndef SAFE_CLI_BRANCH
	@echo "A branch or PR reference must be provided."
	@echo "Please set SAFE_CLI_BRANCH to a valid branch or PR reference."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_NUMBER
	@echo "A build number must be supplied for build artifact packaging."
	@echo "Please set SAFE_CLI_BUILD_NUMBER to a valid build number."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_TYPE
	@echo "A value must be supplied for SAFE_CLI_BUILD_TYPE."
	@echo "Valid values are 'dev' or 'non-dev'."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_COMPONENT
	@echo "A value must be supplied for SAFE_CLI_BUILD_COMPONENT."
	@echo "Valid values are 'safe-cli', 'safe-api' or 'safe-ffi'."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_TARGET
	@echo "A value must be supplied for SAFE_CLI_BUILD_TARGET."
	@exit 1
endif
ifeq ($(SAFE_CLI_BUILD_TYPE),dev)
	$(eval ARCHIVE_NAME := ${SAFE_CLI_BRANCH}-${SAFE_CLI_BUILD_NUMBER}-${SAFE_CLI_BUILD_COMPONENT}-${SAFE_CLI_BUILD_TARGET}-dev.tar.gz)
else
	$(eval ARCHIVE_NAME := ${SAFE_CLI_BRANCH}-${SAFE_CLI_BUILD_NUMBER}-${SAFE_CLI_BUILD_COMPONENT}-${SAFE_CLI_BUILD_TARGET}.tar.gz)
endif
	tar -C artifacts -zcvf ${ARCHIVE_NAME} .
	rm artifacts/**
	mv ${ARCHIVE_NAME} artifacts

retrieve-all-build-artifacts:
ifndef SAFE_CLI_BRANCH
	@echo "A branch or PR reference must be provided."
	@echo "Please set SAFE_CLI_BRANCH to a valid branch or PR reference."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_NUMBER
	@echo "A build number must be supplied for build artifact packaging."
	@echo "Please set SAFE_CLI_BUILD_NUMBER to a valid build number."
	@exit 1
endif
	./scripts/retrieve-build-artifacts \
		"x86_64-unknown-linux-gnu" "x86_64-pc-windows-gnu" "x86_64-apple-darwin" \
		"armv7-linux-androideabi" "x86_64-linux-android" "x86_64-apple-ios" \
		"aarch64-apple-ios" "apple-ios"

package-universal-ios-lib:
ifndef SAFE_CLI_BUILD_BRANCH
	@echo "A branch or PR reference must be provided."
	@echo "Please set SAFE_CLI_BUILD_BRANCH to a valid branch or PR reference."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_NUMBER
	@echo "A valid build number must be supplied for the artifacts to be retrieved."
	@echo "Please set SAFE_CLI_BUILD_NUMBER to a valid build number."
	@exit 1
endif
	( \
		cd artifacts; \
		tar -C real/universal -zcvf \
			${SAFE_CLI_BUILD_BRANCH}-${SAFE_CLI_BUILD_NUMBER}-safe-ffi-apple-ios.tar.gz .; \
	)
	rm -rf artifacts/real

clean:
ifndef SAFE_AUTH_PORT
	$(eval SAFE_AUTH_PORT := ${SAFE_AUTH_DEFAULT_PORT})
endif
ifeq ($(OS),Windows_NT)
	powershell.exe -File resources/test-scripts/cleanup.ps1 -port ${SAFE_AUTH_PORT}
else ifeq ($(UNAME_S),Darwin)
	lsof -t -i tcp:${SAFE_AUTH_PORT} | xargs -n 1 -x kill
endif
	$(eval MOCK_VAULT_PATH := ~/safe_auth-${SAFE_AUTH_PORT})
	rm -rf ${MOCK_VAULT_PATH}

package-commit_hash-artifacts-for-deploy:
	rm -f *.zip
	rm -rf deploy
	mkdir -p deploy/dev
	mkdir -p deploy/release
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-unknown-linux-gnu.zip artifacts/linux/release/safe
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-pc-windows-gnu.zip artifacts/win/release/safe.exe
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-apple-darwin.zip artifacts/macos/release/safe
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-unknown-linux-gnu-dev.zip artifacts/linux/dev/safe
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-pc-windows-gnu-dev.zip artifacts/win/dev/safe.exe
	zip safe-cli-$$(git rev-parse --short HEAD)-x86_64-apple-darwin-dev.zip artifacts/macos/dev/safe
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-unknown-linux-gnu.zip deploy/release
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-pc-windows-gnu.zip deploy/release
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-apple-darwin.zip deploy/release
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-unknown-linux-gnu-dev.zip deploy/dev
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-pc-windows-gnu-dev.zip deploy/dev
	mv safe-cli-$$(git rev-parse --short HEAD)-x86_64-apple-darwin-dev.zip deploy/dev

package-version-artifacts-for-deploy:
	rm -rf deploy
	mkdir -p deploy/dev
	mkdir -p deploy/release
	( \
		cd deploy/release; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.zip \
			../../artifacts/linux/release/safe; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.zip \
			../../artifacts/win/release/safe.exe; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.zip \
			../../artifacts/macos/release/safe; \
		tar -C ../../artifacts/linux/release \
			-zcvf safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz safe; \
		tar -C ../../artifacts/win/release \
			-zcvf safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.tar.gz safe.exe; \
		tar -C ../../artifacts/macos/release \
			-zcvf safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.tar.gz safe; \
	)
	( \
		cd deploy/dev; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu-dev.zip \
			../../artifacts/linux/dev/safe; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu-dev.zip \
			../../artifacts/win/dev/safe.exe; \
		zip -j safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin-dev.zip \
			../../artifacts/macos/dev/safe; \
	)

deploy-github-release:
ifndef GITHUB_TOKEN
	@echo "Please set GITHUB_TOKEN to the API token for a user who can create releases."
	@exit 1
endif
	github-release release \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli" \
		--description "$$(./resources/get_release_description.sh ${SAFE_CLI_VERSION})";
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.zip" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.zip;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.zip" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.zip;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.zip" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.zip;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.tar.gz" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-pc-windows-gnu.tar.gz;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.tar.gz" \
		--file deploy/release/safe-cli-${SAFE_CLI_VERSION}-x86_64-apple-darwin.tar.gz;
	github-release upload \
		--user ${GITHUB_REPO_OWNER} \
		--repo ${GITHUB_REPO_NAME} \
		--tag ${SAFE_CLI_VERSION} \
		--name "safe_completion.sh" \
		--file resources/safe_completion.sh

retrieve-cache:
ifndef SAFE_CLI_BRANCH
	@echo "A branch reference must be provided."
	@echo "Please set SAFE_CLI_BRANCH to a valid branch reference."
	@exit 1
endif
ifndef SAFE_CLI_OS
	@echo "The OS for the cache must be specified."
	@echo "Please set SAFE_CLI_OS to either 'macos' or 'windows'."
	@exit 1
endif
	aws s3 cp \
		--no-sign-request \
		--region eu-west-2 \
		s3://${S3_BUCKET}/safe_cli-${SAFE_CLI_BRANCH}-${SAFE_CLI_OS}-cache.tar.gz .
	mkdir target
	tar -C target -xvf safe_cli-${SAFE_CLI_BRANCH}-${SAFE_CLI_OS}-cache.tar.gz
	rm safe_cli-${SAFE_CLI_BRANCH}-${SAFE_CLI_OS}-cache.tar.gz
