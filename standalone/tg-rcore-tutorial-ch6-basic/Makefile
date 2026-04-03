PACKAGE_NAME := tg-rcore-tutorial-ch6-basic

.PHONY: setup run test-base test-exercise check-exercise clean-vendor

setup:
	./scripts/ensure-vendor.sh

run: setup
	cargo run

test-base: setup
	./test.sh base

test-exercise: setup
	./test.sh exercise

check-exercise: setup
	cargo check --features exercise

clean-vendor:
	rm -rf vendor
