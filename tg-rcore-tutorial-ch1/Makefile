.PHONY: build run test package publish-dry-run

build:
	cargo build

run:
	cargo run

test:
	bash test.sh

package:
	cargo package --list --allow-dirty

publish-dry-run:
	cargo publish --dry-run --allow-dirty
