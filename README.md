<div align="center">

<img src="assets/bunkr-banner.png" alt="Bunkr Downloader" width="100%">

# Bunkr Downloader — Rust Edition

**Baixe álbuns e arquivos do Bunkr em alta velocidade — CLI e GUI nativas em Rust.**
**Download Bunkr albums and files at high speed — native Rust CLI and GUI.**

<a href="#en"><img src="https://img.shields.io/badge/🇺🇸_English-2ea043?style=for-the-badge" alt="English" height="40"></a>
&nbsp;
<a href="#pt"><img src="https://img.shields.io/badge/🇧🇷_Português-6c47ff?style=for-the-badge" alt="Português" height="40"></a>

</div>

---

<div id="en"></div>

## 🇺🇸 English

*Switch to [🇧🇷 Português](#-português) below.*

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust&logoColor=white)](https://rustup.rs)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue)](#license)
[![Platform](https://img.shields.io/badge/Windows%20%7C%20Linux%20%7C%20macOS-supported-success)](#download)

<br>

<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe">
  <img src="https://img.shields.io/badge/⬇_Download_now-Windows_installer-2ea043?style=for-the-badge&logo=windows&logoColor=white" alt="Download for Windows" height="50">
</a>

</div>

### Download

<div align="center">

<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe">
  <img src="https://img.shields.io/badge/⬇_Download_for_Windows-Installer_.exe-2ea043?style=for-the-badge&logo=windows&logoColor=white" alt="Download for Windows" height="46">
</a>
&nbsp;
<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-linux-x86_64">
  <img src="https://img.shields.io/badge/⬇_Linux_x64-CLI_binary-2ea043?style=for-the-badge&logo=linux&logoColor=white" alt="Download for Linux" height="46">
</a>
&nbsp;
<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest">
  <img src="https://img.shields.io/badge/All_versions-Releases-6c47ff?style=for-the-badge&logo=github&logoColor=white" alt="All releases" height="46">
</a>

*Latest version: [v2.0.0](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest) • Windows 10/11 and Linux x64*

</div>

#### ✅ Recommended: Windows installer

Download **[`bunkr-setup-2.0.0-windows-x64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe)**,
double-click it and follow the wizard. It installs the **CLI and GUI** ready to run, creates Desktop
and Start Menu shortcuts, adds `bunkr` to your `PATH`, and includes an uninstaller.
**No compiling, no Rust required.**

#### 🐧 Linux — one-command install

```bash
# downloads install.sh and installs CLI + GUI into ~/.local/bin
curl -fsSL https://raw.githubusercontent.com/jonperr/bunkr-rust-edition-downloader/main/install.sh | bash
```

Or straight from the repository:

```bash
git clone https://github.com/jonperr/bunkr-rust-edition-downloader.git
cd bunkr-rust-edition-downloader
./install.sh            # downloads the prebuilt binaries from the release
./install.sh --source   # or build from source (requires Rust 1.80+)
./install.sh --no-gui   # install the CLI only
```

#### Standalone binaries

| Platform | CLI | GUI |
|---|---|---|
| **Windows x64** | [`bunkr-cli-windows-x86_64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-cli-windows-x86_64.exe) | [`bunkr-gui-windows-x86_64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-gui-windows-x86_64.exe) |
| **Linux x64** | [`bunkr-linux-x86_64`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-linux-x86_64) | [`bunkr-gui-linux-x86_64`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-gui-linux-x86_64) |

On Linux, make it executable: `chmod +x bunkr-linux-x86_64 && ./bunkr-linux-x86_64 --version`

> The binaries are self-contained. On Linux the GUI uses the system's OpenGL/Wayland/X11.
> On macOS and other architectures, build from source (see [Build](#build)).

### Quick start

```bash
# a whole album
bunkr https://bunkr.cr/a/xAsSbMzU

# a single file
bunkr https://bunkr.cr/f/XXXXXXX

# no arguments: reads URLs from URLs.txt (one per line)
bunkr

# list what would be downloaded, with names and sizes, without downloading
bunkr --dry-run https://bunkr.cr/a/xAsSbMzU
```

Or open the **GUI** (`bunkr-gui`) and paste your links — same features, with visual progress,
live speed, a continuous queue and dark mode.

### Features

| | |
|---|---|
| **Chunked downloads** | Files ≥ 16 MB are split across up to 8 parallel connections |
| **Resume** | Interrupted files continue from the exact byte, even inside a chunk |
| **No merge pass** | Each chunk is written straight to its offset in a preallocated `.temp` |
| **4 concurrent files** | Configurable per album, with album pagination fetched in parallel |
| **Filters** | `--ignore` / `--include` by substring on the filename |
| **Renaming** | `--clean-name` keeps the original filename and adds `(N)` on conflicts |
| **Speed limit** | `--rate-limit KB/S` shared across every connection |
| **Native GUI** | Opens in milliseconds, no heavy runtime, 0% idle CPU |
| **Configuration** | CLI > `bunkr.toml` > defaults |

### Build

Requires [Rust](https://rustup.rs) 1.80 or newer:

```bash
git clone https://github.com/jonperr/bunkr-rust-edition-downloader.git
cd bunkr-rust-edition-downloader

cargo build --release                      # CLI only   -> target/release/bunkr
cargo build --release --features gui       # CLI + GUI  -> target/release/bunkr-gui
```

On Linux the GUI needs the system's windowing/GL libraries (X11 or Wayland + OpenGL), which ship
with any desktop. On Windows and macOS nothing extra is required.

### CLI options

<details>
<summary>Show all options</summary>

| Option | Description |
|---|---|
| `-f, --file <PATH>` | Read URLs from this file (default: `URLs.txt`) |
| `--custom-path <DIR>` | Directory where downloaded content is saved |
| `--no-download-folder` | Do not create the `Downloads` subfolder |
| `--no-album-folder` | Do not create the `TITLE (ID)` subfolder |
| `--clean-name` | Keep the original filename |
| `--max-retries <N>` | Attempts per file (default: 5) |
| `--connections <N>` | Parallel connections per file (default: 8; 1 disables chunking) |
| `--workers <N>` | Concurrent files per album (default: 4) |
| `--rate-limit <KB/S>` | Total maximum speed (default: unlimited) |
| `--dry-run` | Show names/sizes/status without downloading |
| `--max-concurrent-urls <N>` | URLs processed at once in batch mode (default: 1) |
| `--ignore <TEXT>` | Skip files whose name contains the text (repeatable) |
| `--include <TEXT>` | Only download files whose name contains the text (repeatable) |
| `--refresh` | Ignore the cached album listing and crawl again |
| `--disable-ui` | Replace progress bars with plain log lines |
| `--disable-disk-check` | Skip the free-space check |
| `--disable-server-check` | (legacy, kept for compatibility) |
| `--config <PATH>` | TOML configuration file |

</details>

#### `bunkr.toml`

Copy `bunkr.toml.example` to `bunkr.toml` and set your defaults:

```toml
custom_path = "D:/Downloads/Bunkr"
connections = 16
workers = 6
rate_limit = 20000
ignore = ["sample", "trailer"]
```

### Performance

- **Chunking from 16 MB** with 8 connections per file and 4 concurrent files.
- Each chunk is written **straight to its final offset** in a preallocated file — there is no
  `.partN` merge step that re-read and re-wrote the whole file.
- A dedicated HTTP/1.1 client for downloads: every `Range` opens its own TCP connection
  (HTTP/2 would multiplex everything over one socket). Pages and the API use HTTP/2 with a
  reused connection pool.
- A failed chunk **resumes from the byte it reached** instead of restarting.
- With `--clean-name`, filtered or already-downloaded files never even call the signing API.

### GUI

Same screen and same features as before: links (with "Paste Links" and a valid-link count),
destination folder (Change/Open), collapsible advanced settings, **DOWNLOAD NOW**,
**Cancel All**, overall progress plus one row per link, an event log, and dark/light mode.
The queue is continuous: you can paste and start more links while others are still downloading.

It reads and writes the same `bunkr_gui_settings.ini` (next to the executable), so your existing
settings carry over.

**Why it opens fast:** a direct OpenGL renderer (`glow`), no network/TLS/thread pool before the
first frame, and progress is read per frame from shared state instead of going through event queues.

### How it works

1. Fetches the album page and extracts the item pages (all pagination pages in parallel).
2. For each item, extracts `jsCDN` (or the `data-file-id`) and requests a **signed token** from the signing API.
3. Sends a `HEAD` to learn size and `Range` support, splits the file and downloads the chunks in parallel.
4. Writes to `.temp`, atomically renames into the final folder, and records state in `.bunkr_state.json`.

Each file has its own retry budget with exponential backoff. Transient errors
(`429`, `502`, `503`, `521`, timeouts) are retried; permanent errors are reported.

### Known limitations

- `.bunkrparts` / `.partN` from the old Python version are **not** reused (new format).
- Cached albums are revalidated against the first page; `--refresh` forces a new crawl.
- Bunkr's server list and domains change over time — if the site moves, use the current URL;
  the client already tries the alternate domain on a `403`.

---

<div id="pt"></div>

## 🇧🇷 Português

*Troque para [🇺🇸 English](#-english) acima.*

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust&logoColor=white)](https://rustup.rs)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue)](#licença)
[![Platform](https://img.shields.io/badge/Windows%20%7C%20Linux%20%7C%20macOS-supported-success)](#download)

<br>

<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe">
  <img src="https://img.shields.io/badge/⬇_Baixar_agora-Instalador_Windows-2ea043?style=for-the-badge&logo=windows&logoColor=white" alt="Baixar para Windows" height="50">
</a>

</div>

### Download

<div align="center">

<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe">
  <img src="https://img.shields.io/badge/⬇_Baixar_para_Windows-Instalador_.exe-2ea043?style=for-the-badge&logo=windows&logoColor=white" alt="Baixar para Windows" height="46">
</a>
&nbsp;
<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-linux-x86_64">
  <img src="https://img.shields.io/badge/⬇_Linux_x64-CLI_binário-2ea043?style=for-the-badge&logo=linux&logoColor=white" alt="Baixar para Linux" height="46">
</a>
&nbsp;
<a href="https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest">
  <img src="https://img.shields.io/badge/Todas_as_versões-Releases-6c47ff?style=for-the-badge&logo=github&logoColor=white" alt="Todas as releases" height="46">
</a>

*Última versão: [v2.0.0](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest) • Windows 10/11 e Linux x64*

</div>

#### ✅ Recomendado: instalador Windows

Baixe **[`bunkr-setup-2.0.0-windows-x64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-setup-2.0.0-windows-x64.exe)**,
dê um duplo clique e siga o assistente. Ele instala a **CLI e a GUI** prontas, cria atalhos na
área de trabalho e no menu iniciar, adiciona `bunkr` ao `PATH` e inclui desinstalador.
**Não precisa compilar nada nem ter Rust instalado.**

#### 🐧 Linux — instalação de um comando

```bash
curl -fsSL https://raw.githubusercontent.com/jonperr/bunkr-rust-edition-downloader/main/install.sh | bash
```

Ou, a partir do repositório:

```bash
git clone https://github.com/jonperr/bunkr-rust-edition-downloader.git
cd bunkr-rust-edition-downloader
./install.sh            # baixa os binários prontos da release
./install.sh --source   # ou compila do código (requer Rust 1.80+)
./install.sh --no-gui   # instala só a CLI
```

#### Binários avulsos

| Plataforma | CLI | GUI |
|---|---|---|
| **Windows x64** | [`bunkr-cli-windows-x86_64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-cli-windows-x86_64.exe) | [`bunkr-gui-windows-x86_64.exe`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-gui-windows-x86_64.exe) |
| **Linux x64** | [`bunkr-linux-x86_64`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-linux-x86_64) | [`bunkr-gui-linux-x86_64`](https://github.com/jonperr/bunkr-rust-edition-downloader/releases/latest/download/bunkr-gui-linux-x86_64) |

No Linux, deixe o arquivo executável: `chmod +x bunkr-linux-x86_64 && ./bunkr-linux-x86_64 --version`

> Os binários são autocontidos. No Linux, a GUI usa OpenGL/Wayland/X11 do próprio sistema.
> No macOS e em arquiteturas diferentes, compile a partir do código (veja [Compilar](#compilar)).

### Uso rápido

```bash
# um álbum inteiro
bunkr https://bunkr.cr/a/xAsSbMzU

# um único arquivo
bunkr https://bunkr.cr/f/XXXXXXX

# sem argumentos: lê as URLs de URLs.txt (uma por linha)
bunkr

# lista o que seria baixado, com nomes e tamanhos, sem baixar
bunkr --dry-run https://bunkr.cr/a/xAsSbMzU
```

Ou abra a **GUI** (`bunkr-gui`) e cole os links — mesma função, com progresso visual,
velocidade em tempo real, fila contínua e modo escuro.

### Recursos

| | |
|---|---|
| **Download em pedaços** | Arquivos ≥ 16 MB são divididos e baixados por até 8 conexões paralelas |
| **Retomada** | Arquivo interrompido continua de onde parou (byte exato, até dentro de um pedaço) |
| **Sem merge de partes** | Cada pedaço é gravado direto na posição certa de um `.temp` pré-alocado |
| **4 arquivos simultâneos** | Ajustável por álbum, com paginação de álbum buscada em paralelo |
| **Filtros** | `--ignore` / `--include` por substring no nome do arquivo |
| **Renomeação** | `--clean-name` mantém o nome original e adiciona `(N)` em caso de conflito |
| **Limite de velocidade** | `--rate-limit KB/S` compartilhado por todas as conexões |
| **GUI nativa** | Abre em milissegundos, sem runtime pesado, 0% de CPU ociosa |
| **Configuração** | CLI > `bunkr.toml` > padrão |

### Compilar

Requer [Rust](https://rustup.rs) 1.80 ou mais recente:

```bash
git clone https://github.com/jonperr/bunkr-rust-edition-downloader.git
cd bunkr-rust-edition-downloader

cargo build --release                      # só a CLI   -> target/release/bunkr
cargo build --release --features gui       # + a GUI    -> target/release/bunkr-gui
```

No Linux, a GUI pede as bibliotecas de janela/GL do sistema (X11 ou Wayland + OpenGL), que já
vêm em qualquer desktop. No Windows e no macOS não é preciso nada extra.

### Opções da CLI

<details>
<summary>Ver todas as opções</summary>

| Opção | Descrição |
|---|---|
| `-f, --file <PATH>` | Lê as URLs deste arquivo (padrão: `URLs.txt`) |
| `--custom-path <DIR>` | Pasta onde o conteúdo é salvo |
| `--no-download-folder` | Não cria a subpasta `Downloads` |
| `--no-album-folder` | Não cria a subpasta `TÍTULO (ID)` |
| `--clean-name` | Mantém o nome original do arquivo |
| `--max-retries <N>` | Tentativas por arquivo (padrão: 5) |
| `--connections <N>` | Conexões paralelas por arquivo (padrão: 8; 1 desativa o chunking) |
| `--workers <N>` | Arquivos simultâneos por álbum (padrão: 4) |
| `--rate-limit <KB/S>` | Velocidade máxima total (padrão: ilimitada) |
| `--dry-run` | Mostra nomes/tamanhos/status sem baixar |
| `--max-concurrent-urls <N>` | URLs processadas ao mesmo tempo em lote (padrão: 1) |
| `--ignore <TEXT>` | Pula arquivos cujo nome contém o texto (repetível) |
| `--include <TEXT>` | Só baixa arquivos cujo nome contém o texto (repetível) |
| `--refresh` | Ignora o cache do álbum e refaz a listagem |
| `--disable-ui` | Substitui as barras de progresso por linhas de log |
| `--disable-disk-check` | Pula a checagem de espaço livre |
| `--disable-server-check` | (legado, mantido por compatibilidade) |
| `--config <PATH>` | Arquivo TOML de configuração |

</details>

#### `bunkr.toml`

Copie `bunkr.toml.example` para `bunkr.toml` e ajuste os padrões:

```toml
custom_path = "D:/Downloads/Bunkr"
connections = 16
workers = 6
rate_limit = 20000
ignore = ["sample", "trailer"]
```

### Performance

- **Chunking a partir de 16 MB** com 8 conexões por arquivo e 4 arquivos simultâneos.
- Cada pedaço é escrito **direto na posição final** de um arquivo pré-alocado — não existe o
  passo de juntar `.partN` que relia e regravava o arquivo inteiro.
- Cliente HTTP/1.1 dedicado a downloads: cada `Range` abre sua própria conexão TCP
  (HTTP/2 multiplexaria tudo num socket só). Páginas e API usam HTTP/2 com pool reaproveitado.
- Pedaço que falha **continua do byte onde parou**, em vez de recomeçar.
- Em `--clean-name`, arquivos filtrados ou já existentes nem chamam a API de assinatura.

### Interface gráfica

Mesma tela e mesmas funções: links (com "Colar Links" e contagem de válidos), pasta de destino
(Alterar/Abrir), configurações avançadas recolhíveis, **BAIXAR AGORA**, **Cancelar Tudo**,
progresso geral + uma linha por link, log de eventos, modo escuro/claro. A fila é contínua:
dá para colar e baixar mais links enquanto outros já estão baixando.

Ela lê e grava o mesmo `bunkr_gui_settings.ini` (ao lado do executável), então suas configurações
antigas continuam valendo.

**Por que abre rápido:** renderizador OpenGL direto (`glow`), nada de rede/TLS/thread pool antes
do primeiro quadro, e o progresso é lido por frame do estado compartilhado em vez de passar por
filas de eventos.

### Como funciona

1. Busca a página do álbum e extrai as páginas dos itens (todas as páginas de paginação em paralelo).
2. Para cada item, extrai `jsCDN` (ou o `data-file-id`) e pede um **token assinado** à API de assinatura.
3. Faz `HEAD` para descobrir tamanho e suporte a `Range`, divide o arquivo e baixa os pedaços em paralelo.
4. Grava no `.temp`, renomeia atomicamente na pasta final e registra o estado em `.bunkr_state.json`.

Cada arquivo tem seu próprio conjunto de tentativas com backoff exponencial. Erros transitórios
(`429`, `502`, `503`, `521`, timeouts) são repetidos; erros permanentes são reportados.

### Limitações conhecidas

- `.bunkrparts` / `.partN` da versão antiga em Python **não** são reaproveitados (formato novo).
- O album em cache é revalidado pela primeira página; `--refresh` força um novo crawl.
- A lista de servidores e domínios do Bunkr muda com o tempo — se o site trocar de domínio, use
  a URL atual; o cliente já tenta o domínio alternativo ao receber `403`.

---

## Licença

Distribuído sob a **GNU General Public License v3.0 ou posterior (GPL-3.0-or-later)**.
Veja [LICENSE](LICENSE) para o texto completo.

---

<div align="center">

Feito com 🦀 em Rust

</div>
