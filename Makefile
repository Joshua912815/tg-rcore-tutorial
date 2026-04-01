SHELL := /bin/bash

.PHONY: extract run run-docker check-docker report

extract:
	@if [ ! -d tg-rcore-tutorial-ch8 ]; then \
		bash scripts/extract_submodules.sh; \
	else \
		echo "submodule crates already present, skip extraction"; \
	fi

run: extract
	bash scripts/t8-regression.sh all

run-docker: extract
	bash scripts/t8-docker-regression.sh rcore-docker all

check-docker:
	bash scripts/t8-docker-check.sh rcore-docker

report:
	cat report.md
