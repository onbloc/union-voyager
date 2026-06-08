# union-voyager Makefile
#
# Wrapper around nix build for the Voyager IBC relayer.
# Supported systems: x86_64-linux, aarch64-linux, aarch64-darwin (M-series Mac).
# Intel Mac (x86_64-darwin) is not supported by the upstream nix flake.

SHELL       := /bin/bash
.SHELLFLAGS := -o pipefail -c

# Make recipes can use `voyager` directly without the full path.
export PATH := $(CURDIR)/result/bin:$(PATH)

VOYAGER_LOG         := voyager.log
VOYAGER_MODULES_LOG := voyager-modules-plugins.log
VOYAGER_RUN_LOG     := voyager-run.log

.PHONY: help build run run-reset stop

help:
	@echo "Targets:"
	@echo "  build   — build voyager and voyager-modules-plugins via nix (parallel, logs to *.log)"
	@echo "  run     — start voyager in the background (nohup)"
	@echo "  run-reset — truncate the voyager queue then start voyager"
	@echo "  stop      — kill the running voyager process"
	@echo ""
	@echo "Logs:"
	@echo "  voyager build output            → $(VOYAGER_LOG)"
	@echo "  voyager-modules-plugins output  → $(VOYAGER_MODULES_LOG)"
	@echo "  voyager runtime output          → $(VOYAGER_RUN_LOG)"

stop:
	@pgrep voyager >/dev/null 2>&1 || { echo "voyager is not running"; exit 0; }
	@kill $$(pgrep voyager)
	@echo "ok: voyager stopped"

run-reset:
	@command -v voyager >/dev/null 2>&1 || { \
		echo "ERROR: voyager not found. Run 'make build' first."; exit 1; }
	@echo ">> truncating queue"
	@voyager queue truncate --queue --done --optimize --failed --config-file-path voyager/config.jsonc
	@echo "ok: queue truncated"
	@RUST_LOG=info nohup voyager --config-file-path voyager/config.jsonc start > $(VOYAGER_RUN_LOG) 2>&1 &
	@echo ">> voyager started (pid: $$!)"
	@echo "   tail -f $(VOYAGER_RUN_LOG)"

run:
	@command -v nix >/dev/null 2>&1 || { \
		echo "ERROR: 'nix' not found on PATH. Install Nix from https://nixos.org/download"; exit 1; }
	@command -v voyager >/dev/null 2>&1 || { \
		echo "ERROR: voyager not found. Run 'make build' first."; exit 1; }
	@RUST_LOG=info nohup voyager --config-file-path voyager/config.jsonc start > $(VOYAGER_RUN_LOG) 2>&1 &
	@echo ">> voyager started (pid: $$!)"
	@echo "   tail -f $(VOYAGER_RUN_LOG)"

build:
	@command -v nix >/dev/null 2>&1 || { \
		echo "ERROR: 'nix' not found on PATH. Install Nix from https://nixos.org/download"; exit 1; }
	@echo ">> building voyager and voyager-modules-plugins (parallel)"
	@nix build .#voyager -L > $(VOYAGER_LOG) 2>&1 & \
	 nix build .#voyager-modules-plugins -L > $(VOYAGER_MODULES_LOG) 2>&1 & \
	 wait
	@echo "ok: build complete  (see $(VOYAGER_LOG) and $(VOYAGER_MODULES_LOG))"
	@echo ">> symlinking voyager modules to target/debug/"
	@mkdir -p target/debug && \
	 for bin in /nix/store/*voyager*/bin/voyager-*; do ln -sf "$$bin" "target/debug/$$(basename $$bin)"; done
	@echo "ok: $$(ls target/debug/ | wc -l) symlinks created in target/debug/"
	@echo ""
	@echo "   to use voyager in your shell:"
	@echo "   export PATH=\$$PATH:$(CURDIR)/result/bin"
