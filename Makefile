# Beep Makefile
#
# Usage:
#   make build                        Build all targets, all platforms
#   make build TARGET=cli             Build CLI only, all platforms
#   make build TARGET=gui             Build GUI only, all platforms
#   make build TARGET=tui             Build TUI only, all platforms
#   make build TARGET=gui PLATFORM=windows   GUI, Windows only
#   make build PLATFORM=linux         All targets, Linux only
#
#   make setup-gui                    Install GUI frontend deps (first time)
#   make run-gui                      Launch GUI dev server
#
#   make setup-cli                    Build CLI crate & deps
#   make run-cli                      Run CLI binary
#   make run-cli ARGS="--help"         Run with arguments

TARGET ?= all
PLATFORM ?= all

.PHONY: build run-gui setup-gui run-cli setup-cli

# ── Build ───────────────────────────────────────────────────────────────

build:
	chmod +x build.sh
	./build.sh $(TARGET) $(PLATFORM)

# ── Setup ───────────────────────────────────────────────────────────────

setup-gui:
	cd crates/beep-gui && npm install

# ── Run ─────────────────────────────────────────────────────────────────

run-gui:
	cd crates/beep-gui && npm run tauri dev

run-cli:
	cargo run -p beep -- $(ARGS)
