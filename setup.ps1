# Instala o Bunkr Downloader para o usuário atual (Windows).
#   .\setup.ps1               compila (se necessário) e instala em %LOCALAPPDATA%\Bunkr
#   .\setup.ps1 -NoGui        instala só a CLI
param(
    [switch]$NoGui,
    [string]$Prefix = "$env:LOCALAPPDATA\Bunkr"
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
New-Item -ItemType Directory -Force -Path $Prefix | Out-Null

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Rust nao encontrado. Instale em https://rustup.rs e rode de novo."
}

Write-Host "Compilando..." -ForegroundColor Cyan
if ($NoGui) { cargo build --release } else { cargo build --release --features gui }
if ($LASTEXITCODE -ne 0) { Write-Error "A compilacao falhou." }

Copy-Item "target\release\bunkr.exe" "$Prefix\bunkr.exe" -Force
Write-Host "CLI instalada em $Prefix\bunkr.exe" -ForegroundColor Green

if (-not $NoGui -and (Test-Path "target\release\bunkr-gui.exe")) {
    Copy-Item "target\release\bunkr-gui.exe" "$Prefix\bunkr-gui.exe" -Force
    Write-Host "GUI instalada em $Prefix\bunkr-gui.exe" -ForegroundColor Green
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$Prefix*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$Prefix", "User")
    Write-Host "Adicionado ao PATH do usuario. Abra um novo terminal para usar o comando 'bunkr'." -ForegroundColor Yellow
}

Write-Host "Pronto! Teste com: bunkr --version" -ForegroundColor Green
