.PHONY: lint build install fmt clippy check-openapi update-openapi test qa bump-homebrew

CARGO ?= cargo
HOMEBREW_TAP ?= $(abspath ../homebrew-tap)
VERSION ?= $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
TAG ?= v$(VERSION)

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

# Requires the GitHub tag to exist. Updates sibling homebrew-tap.
bump-homebrew:
	@test -n "$(VERSION)" || (echo "could not read version from Cargo.toml" >&2; exit 2)
	$(HOMEBREW_TAP)/scripts/bump-formula.sh \
		--formula timely-cli \
		--tag "$(TAG)" \
		--repository amkisko/timely-cli.rs \
		--commit
