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

test:
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
