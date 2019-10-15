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

build-component:
ifndef SAFE_CLI_BUILD_COMPONENT
	@echo "A build component must be specified."
	@echo "Please set SAFE_CLI_BUILD_COMPONENT to 'safe-api', 'safe-ffi' or 'safe-cli'."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_TYPE
	@echo "A build type must be specified."
	@echo "Please set SAFE_CLI_BUILD_TYPE to 'dev' or 'non-dev'."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_TARGET
	@echo "A build target must be specified."
	@echo "Please set SAFE_CLI_BUILD_TARGET to a valid Rust 'target triple', e.g. 'x86_64-unknown-linux-gnu'."
	@exit 1
endif
ifndef SAFE_CLI_BUILD_CLEAN
	$(eval SAFE_CLI_BUILD_CLEAN := false)
endif
	./resources/build-component.sh \
		"${SAFE_CLI_BUILD_COMPONENT}" \
		"${SAFE_CLI_BUILD_TARGET}" \
		"${SAFE_CLI_BUILD_TYPE}" \
		"${SAFE_CLI_BUILD_CLEAN}"

build-all-containers:
	SAFE_CLI_CONTAINER_TARGET=x86_64-unknown-linux-gnu \
	SAFE_CLI_CONTAINER_TYPE=non-dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-cli \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-unknown-linux-gnu \
	SAFE_CLI_CONTAINER_TYPE=dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-cli \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-unknown-linux-gnu \
	SAFE_CLI_CONTAINER_TYPE=dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-api \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-unknown-linux-gnu \
	SAFE_CLI_CONTAINER_TYPE=dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-unknown-linux-gnu \
	SAFE_CLI_CONTAINER_TYPE=non-dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-linux-android \
	SAFE_CLI_CONTAINER_TYPE=dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=x86_64-linux-android \
	SAFE_CLI_CONTAINER_TYPE=non-dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=armv7-linux-androideabi \
	SAFE_CLI_CONTAINER_TYPE=dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container
	SAFE_CLI_CONTAINER_TARGET=armv7-linux-androideabi \
	SAFE_CLI_CONTAINER_TYPE=non-dev \
	SAFE_CLI_CONTAINER_COMPONENT=safe-ffi \
		make build-container

build-container:
ifndef SAFE_CLI_CONTAINER_COMPONENT
	@echo "A component to build must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_COMPONENT to 'safe-api', 'safe-ffi' or 'safe-cli'."
	@exit 1
endif
ifndef SAFE_CLI_CONTAINER_TYPE
	@echo "A container type must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_TYPE to 'dev' or 'non-dev'."
	@exit 1
endif
ifndef SAFE_CLI_CONTAINER_TARGET
	@echo "A build target must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_TARGET to a valid Rust 'target triple', e.g. 'x86_64-unknown-linux-gnu'."
	@exit 1
endif
	./resources/build-container.sh \
		"${SAFE_CLI_CONTAINER_COMPONENT}" \
		"${SAFE_CLI_CONTAINER_TARGET}" \
		"${SAFE_CLI_CONTAINER_TYPE}"

push-container:
ifndef SAFE_CLI_CONTAINER_COMPONENT
	@echo "A component to build must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_COMPONENT to 'safe-api', 'safe-ffi' or 'safe-cli'."
	@exit 1
endif
ifndef SAFE_CLI_CONTAINER_TYPE
	@echo "A container type must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_TYPE to 'dev' or 'non-dev'."
	@exit 1
endif
ifndef SAFE_CLI_CONTAINER_TARGET
	@echo "A build target must be specified."
	@echo "Please set SAFE_CLI_CONTAINER_TARGET to a valid Rust 'target triple', e.g. 'x86_64-unknown-linux-gnu'."
	@exit 1
endif
	$(eval COMPONENT_NAME := $(shell echo ${SAFE_CLI_CONTAINER_COMPONENT} | sed 's/safe-//g'))
ifeq ($(SAFE_CLI_CONTAINER_TYPE), dev)
	docker push \
		maidsafe/safe-cli-build:${COMPONENT_NAME}-${SAFE_CLI_CONTAINER_TARGET}-dev
else
	docker push \
		maidsafe/safe-cli-build:${COMPONENT_NAME}-${SAFE_CLI_CONTAINER_TARGET}
endif

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
	./resources/retrieve-build-artifacts.sh "x86_64-apple-ios" "aarch64-apple-ios"

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
	lipo -create -output artifacts/mock/universal/libsafe_ffi.a \
		artifacts/mock/x86_64-apple-ios/release/libsafe_ffi.a \
		artifacts/mock/aarch64-apple-ios/release/libsafe_ffi.a

strip-artifacts:
ifeq ($(OS),Windows_NT)
	find artifacts -name "safe.exe" -exec strip -x '{}' \;
else ifeq ($(UNAME_S),Darwin)
	find artifacts -name "safe" -exec strip -x '{}' \;
else
	find artifacts -name "safe" -exec strip '{}' \;
endif

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
	./resources/retrieve-build-artifacts.sh \
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
	( \
		cd artifacts; \
		tar -C mock/universal -zcvf \
			${SAFE_CLI_BUILD_BRANCH}-${SAFE_CLI_BUILD_NUMBER}-safe-ffi-apple-ios-dev.tar.gz .; \
	)
	rm -rf artifacts/real
	rm -rf artifacts/mock

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
