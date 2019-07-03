SHELL := /bin/bash
SAFE_CLI_VERSION := $(shell grep "^version" < Cargo.toml | head -n 1 | awk '{ print $$3 }' | sed 's/\"//g')
USER_ID := $(shell id -u)
GROUP_ID := $(shell id -g)
UNAME_S := $(shell uname -s)
PWD := $(shell echo $$PWD)
UUID := $(shell uuidgen | sed 's/-//g')
S3_BUCKET := safe-jenkins-build-artifacts

build-container:
	rm -rf target/
	docker rmi -f maidsafe/safe-cli-build:${SAFE_CLI_VERSION}
	docker build -f Dockerfile.build -t maidsafe/safe-cli-build:${SAFE_CLI_VERSION} .

push-container:
	docker push maidsafe/safe-cli-build:${SAFE_CLI_VERSION}
