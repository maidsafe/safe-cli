SHELL := /bin/bash
SAFE_CLI_VERSION := $(shell grep "^version" < Cargo.toml | head -n 1 | awk '{ print $$3 }' | sed 's/\"//g')
USER_ID := $(shell id -u)
GROUP_ID := $(shell id -g)
UNAME_S := $(shell uname -s)
PWD := $(shell echo $$PWD)
UUID := $(shell uuidgen | sed 's/-//g')

build-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:${SAFE_CLI_VERSION}
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:${SAFE_CLI_VERSION} .

push-container:
	docker push maidsafe/safe-cli-build:${SAFE_CLI_VERSION}

test:
	rm -rf artifacts
	mkdir artifacts
ifeq ($(UNAME_S),Linux)
	docker run --name "safe-cli-build-${UUID}" -v "${PWD}":/usr/src/safe-cli:Z \
		-u ${USER_ID}:${GROUP_ID} \
		maidsafe/safe-cli-build:${SAFE_CLI_VERSION} \
		/bin/bash -c "cargo test --release --features=scl-mock -- --test-threads=1"
	docker cp "safe-cli-build-${UUID}":/target .
	docker rm "safe-cli-build-${UUID}"
else
	cargo check --release
	cargo test --release --features=scl-mock -- --test-threads=1
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
ifndef SAFE_CLI_BUILD_OS
	@echo "A value must be supplied for SAFE_CLI_BUILD_OS."
	@echo "Valid values are 'linux' or 'windows' or 'macos'."
	@exit 1
endif
	$(eval ARCHIVE_NAME := ${SAFE_CLI_BRANCH}-${SAFE_CLI_BUILD_NUMBER}-safe_cli-${SAFE_CLI_BUILD_OS}-x86_64.tar.gz)
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
	rm -rf artifacts
	mkdir -p artifacts/linux/release
	mkdir -p artifacts/win/release
	mkdir -p artifacts/osx/release
	aws s3 cp --no-sign-request --region eu-west-2 s3://${S3_BUCKET}/${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-linux-x86_64.tar.gz .
	aws s3 cp --no-sign-request --region eu-west-2 s3://${S3_BUCKET}/${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-windows-x86_64.tar.gz .
	aws s3 cp --no-sign-request --region eu-west-2 s3://${S3_BUCKET}/${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-osx-x86_64.tar.gz .
	tar -C artifacts/linux/release -xvf ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-linux-x86_64.tar.gz
	tar -C artifacts/win/release -xvf ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-windows-x86_64.tar.gz
	tar -C artifacts/osx/release -xvf ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-osx-x86_64.tar.gz
	rm ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-linux-x86_64.tar.gz
	rm ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-windows-x86_64.tar.gz
	rm ${SCL_BRANCH}-${SCL_BUILD_NUMBER}-safe_cli-osx-x86_64.tar.gz
