SHELL := /bin/bash
.SHELLFLAGS := -e -o pipefail -c

.PHONY: all help format format-check clippy test build check image

BUILD_VERSION ?= $(shell ./scripts/version.sh)
export BUILD_VERSION

all: check
all: ## Format-check, lint, test, and build the project.

help: ## Show this help dialog.
	@IFS=$$'\n' ; \
	help_lines=(`fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sed -e 's/\\$$//'`); \
	for help_line in $${help_lines[@]}; do \
		IFS=$$'#' ; \
		help_split=($$help_line) ; \
		help_command=`echo $${help_split[0]} | sed -e 's/^ *//' -e 's/ *$$//'` ; \
		help_info=`echo $${help_split[2]} | sed -e 's/^ *//' -e 's/ *$$//'` ; \
		printf "%-30s %s\n" $$help_command $$help_info ; \
	done

format: ## Format Rust sources.
	cargo fmt --all

format-check: ## Check Rust formatting.
	cargo fmt --all -- --check

clippy: ## Run Clippy with warnings denied.
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test: ## Run all tests.
	cargo test --workspace --all-features

build: ## Build the release binary.
	cargo build --workspace --release

check: format-check clippy test build
check: ## Run every local verification step.

image: ## Build the local container image.
	docker build --build-arg BUILD_VERSION=$(BUILD_VERSION) --tag backup-normalizer:local .
