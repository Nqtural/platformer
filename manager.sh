#!/bin/sh

APP_NAME="platformer"
BIN_NAME="client"
LOCAL_BIN="$HOME/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/$APP_NAME"
CONFIG_FILE="config.toml"
ASSETS_DIR="assets"

mkdir -p "$LOCAL_BIN"
mkdir -p "$DATA_DIR"

warn_path() {
    printf "%s" "$PATH" | grep -q "$LOCAL_BIN"
    if [ $? -ne 0 ]; then
        printf "Warning: %s is not in your PATH!\n" "$LOCAL_BIN"
    fi
}

copy_assets_and_config() {
    printf "Copying assets...\n"
    mkdir -p "$DATA_DIR"
    # Copy assets without overwriting existing files
    for file in "$ASSETS_DIR"/*; do
        target="$DATA_DIR/$(basename "$file")"
        if [ ! -e "$target" ]; then
            cp -r "$file" "$target"
        fi
    done
    # Copy config if missing
    if [ ! -f "$DATA_DIR/$CONFIG_FILE" ]; then
        cp "$CONFIG_FILE" "$DATA_DIR/$CONFIG_FILE"
    fi
}

build_binary() {
    printf "Building %s...\n" "$APP_NAME"
    cargo build --release
    cp "target/release/$BIN_NAME" "$LOCAL_BIN/$APP_NAME"
}

build_debug() {
	printf "Building debug %s...\n" "$APP_NAME"
	cargo build
}

install() {
    printf "Installing %s...\n" "$APP_NAME"
    build_binary
    copy_assets_and_config
    warn_path
    printf "Installation complete.\n"
}

update() {
    printf "Updating %s...\n" "$APP_NAME"
	git stash -u -m "manager auto-stash" >/dev/null 2>&1
    git switch master
    git pull
	git stash pop || printf "Warning: merge conflict after stash pop. Please resolve manually.\n"
    build_binary
    copy_assets_and_config
    warn_path
    printf "Update complete.\n"
}

playtest() {
	git stash -u -m "manager auto-stash" >/dev/null 2>&1
    printf "Switching to playtest...\n"
    git switch playtest
    git pull
	git stash pop || printf "Warning: merge conflict after stash pop. Please resolve manually.\n"
    build_debug
    printf "Playtest branch ready.\n"
}

case "$1" in
    install)
        install
        ;;
    update)
        update
        ;;
    playtest)
        playtest
        ;;
    *)
        printf "Usage: %s {install|update|playtest}\n" "$0"
        exit 1
        ;;
esac
