#!/usr/bin/env bash
# Instala o Bunkr Downloader para o usuário atual, sem root.
#   ./setup.sh            compila (se necessário) e instala em ~/.local/bin
#   ./setup.sh --prefix /usr/local/bin
set -euo pipefail

PREFIX="${HOME}/.local/bin"
WITH_GUI=1

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) PREFIX="$2"; shift 2 ;;
        --no-gui) WITH_GUI=0; shift ;;
        -h|--help) sed -n '2,5p' "$0"; exit 0 ;;
        *) echo "Opção desconhecida: $1" >&2; exit 1 ;;
    esac
done

cd "$(dirname "$0")"
mkdir -p "$PREFIX"

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust não encontrado. Instale em https://rustup.rs e rode de novo." >&2
    exit 1
fi

echo "Compilando..."
if [ "$WITH_GUI" = 1 ]; then
    cargo build --release --features gui
else
    cargo build --release
fi

install -m 755 target/release/bunkr "$PREFIX/bunkr"
echo "CLI instalada em $PREFIX/bunkr"

if [ "$WITH_GUI" = 1 ] && [ -f target/release/bunkr-gui ]; then
    install -m 755 target/release/bunkr-gui "$PREFIX/bunkr-gui"
    echo "GUI instalada em $PREFIX/bunkr-gui"
fi

case ":$PATH:" in
    *":$PREFIX:"*) ;;
    *) echo "Atenção: $PREFIX não está no PATH. Adicione com:"
       echo "  export PATH=\"$PREFIX:\$PATH\"" ;;
esac

echo "Pronto! Teste com: bunkr --version"
