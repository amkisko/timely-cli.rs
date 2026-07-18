.PHONY: lint build install fmt clippy check-openapi update-openapi test qa

CARGO ?= cargo

lint: fmt clippy check-openapi

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check-openapi:
	ruby -c scripts/update-openapi.rb

update-openapi:
	ruby scripts/update-openapi.rb

build:
	$(CARGO) build --workspace

test:
	$(CARGO) test --workspace

qa: lint test

install:
	$(CARGO) install --path timely --locked
