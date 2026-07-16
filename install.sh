#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CARGO_HOME_DIR=${CARGO_HOME:-"$HOME/.cargo"}
SHINE_BINARY="$CARGO_HOME_DIR/bin/shine"

echo "Installing Shine from $ROOT"
cargo install --path "$ROOT" --force

if [ ! -x "$SHINE_BINARY" ]; then
    echo "Error: Cargo did not create $SHINE_BINARY" >&2
    exit 1
fi

path_contains() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

INSTALL_DIR=${SHINE_INSTALL_DIR:-}
if [ -z "$INSTALL_DIR" ]; then
    for candidate in /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin"; do
        if [ -d "$candidate" ] && [ -w "$candidate" ] && path_contains "$candidate"; then
            INSTALL_DIR=$candidate
            break
        fi
    done
fi

if [ -z "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
fi
mkdir -p "$INSTALL_DIR"

ln -sf "$SHINE_BINARY" "$INSTALL_DIR/shine"

if ! path_contains "$INSTALL_DIR"; then
    SHELL_RC="$HOME/.profile"
    case "${SHELL:-}" in
        */zsh) SHELL_RC="$HOME/.zshrc" ;;
        */bash) SHELL_RC="$HOME/.bashrc" ;;
    esac
    PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
    if [ ! -f "$SHELL_RC" ] || ! grep -Fqx "$PATH_LINE" "$SHELL_RC"; then
        printf '\n# Shine programming language\n%s\n' "$PATH_LINE" >> "$SHELL_RC"
    fi
    echo "Added $INSTALL_DIR to PATH in $SHELL_RC"
    echo "Open a new terminal or run: source $SHELL_RC"
fi

echo "Shine installed as $INSTALL_DIR/shine"
"$INSTALL_DIR/shine" version
echo "You can now run 'shine new demo' from any directory."
