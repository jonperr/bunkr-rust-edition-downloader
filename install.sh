#!/usr/bin/env bash
# Instala o Bunkr Downloader no Linux (CLI + GUI) a partir dos binários prontos.
#   ./install.sh                  baixa os binários da última release em ~/.local/bin
#   ./install.sh --source         compila do código (requer Rust 1.80+)
#   ./install.sh --prefix <DIR>   pasta de instalação (padrão: ~/.local/bin)
#   ./install.sh --no-gui         só a CLI
set -euo pipefail

PREFIX="${HOME}/.local/bin"
SOURCE=0
WITH_GUI=1

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) PREFIX="$2"; shift 2 ;;
        --source) SOURCE=1; shift ;;
        --no-gui) WITH_GUI=0; shift ;;
        -h|--help) sed -n '2,6p' "$0"; exit 0 ;;
        *) echo "Opção desconhecida: $1" >&2; exit 1 ;;
    esac
done

mkdir -p "$PREFIX"

if [ "$SOURCE" = 1 ]; then
    cd "$(dirname "$0")"
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Rust não encontrado. Use './install.sh' (sem --source) para baixar os binários." >&2
        exit 1
    fi
    echo "Compilando..."
    if [ "$WITH_GUI" = 1 ]; then
        cargo build --release --features gui
    else
        cargo build --release
    fi
    install -m 755 target/release/bunkr "$PREFIX/bunkr"
    if [ "$WITH_GUI" = 1 ] && [ -f target/release/bunkr-gui ]; then
        install -m 755 target/release/bunkr-gui "$PREFIX/bunkr-gui"
    fi
else
    # baixa os binários da última release do GitHub
    REPO="${BUNKR_REPO:-jonperr/bunkr-rust-edition-downloader}"
    API="https://api.github.com/repos/${REPO}/releases/latest"
    if ! command -v curl >/dev/null 2>&1; then
        echo "curl é necessário. Instale-o ou use './install.sh --source'." >&2
        exit 1
    fi
    TMP="$(mktemp -d)"
    trap 'rm -rf "$TMP"' EXIT
    download() { # $1 nome do asset, $2 destino
        local asset="$1" dest="$2"
        local url
        url="$(curl -fsSL "$API" | grep -oP '"browser_download_url":\s*"\K[^"]*(?=")' | grep "/$asset$" | head -1 || true)"
        if [ -z "$url" ]; then
            echo "Asset não encontrado: $asset" >&2
            return 1
        fi
        curl -fsSL -o "$TMP/$asset" "$url"
        install -m 755 "$TMP/$asset" "$dest"
    }
    echo "Baixando a última release..."
    download bunkr-linux-x86_64 "$PREFIX/bunkr"
    if [ "$WITH_GUI" = 1 ]; then
        download bunkr-gui-linux-x86_64 "$PREFIX/bunkr-gui"
    fi
fi

case ":$PATH:" in
    *":$PREFIX:"*) ;;
    *) echo "Atenção: $PREFIX não está no PATH. Adicione com:"
       echo "  export PATH=\"$PREFIX:\$PATH\"" ;;
esac

echo "Pronto! Teste com: bunkr --version"
