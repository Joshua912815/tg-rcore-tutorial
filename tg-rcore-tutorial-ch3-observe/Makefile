IMAGE ?= rcore-docker

.PHONY: run run-base run-crash run-docker gdb

run:
	bash test.sh all

run-base:
	bash test.sh base

run-crash:
	bash test.sh crash

run-docker:
	bash scripts/t21-docker-regression.sh "$(IMAGE)" all

gdb:
	bash scripts/launch-qemu-gdb.sh
