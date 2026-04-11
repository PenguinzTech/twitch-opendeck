.PHONY: build install uninstall clean dev

PLUGIN_DIR := $(HOME)/.config/opendeck/plugins/io.pngz.twitch.sdPlugin

build:
	. $$HOME/.cargo/env && cargo build --release

install: build
	mkdir -p $(PLUGIN_DIR)
	cp target/release/twitch-opendeck $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu.tmp
	mv $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu.tmp $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu
	cp -r plugin/* $(PLUGIN_DIR)/

uninstall:
	rm -rf $(PLUGIN_DIR)

clean:
	cargo clean

dev:
	source $$HOME/.cargo/env && cargo build
	mkdir -p $(PLUGIN_DIR)
	cp target/debug/twitch-opendeck $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu.tmp
	mv $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu.tmp $(PLUGIN_DIR)/twitch-opendeck-x86_64-unknown-linux-gnu
	cp -r plugin/* $(PLUGIN_DIR)/

