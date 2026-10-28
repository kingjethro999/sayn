<#
.SYNOPSIS
    Sayn Official Installer for Windows PowerShell
.DESCRIPTION
    Installs Sayn AI-native programming language toolchain and prerequisites on Windows.
#>

$ErrorActionPreference = "Stop"

Write-Host "✦ Sayn Windows Installer" -ForegroundColor Cyan
Write-Host "========================="

# 1. Check Git
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "Git not found. Installing via winget..." -ForegroundColor Yellow
    winget install --id Git.Git -e --source winget
} else {
    Write-Host "✔ Git is available: $((git --version))" -ForegroundColor Green
}

# 2. Check Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust toolchain not found. Installing rustup..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --source winget
} else {
    Write-Host "✔ Rust is available: $((cargo --version))" -ForegroundColor Green
}

# 3. Check Node.js
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Node.js not found. Installing Node.js LTS via winget..." -ForegroundColor Yellow
    winget install --id OpenJS.NodeJS.LTS -e --source winget
} else {
    Write-Host "✔ Node.js is available: $((node --version))" -ForegroundColor Green
}

# 4. Install Sayn Binary
$InstallDir = "$HOME\.local\bin"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

if (Test-Path "$ScriptDir\Cargo.toml") {
    Write-Host "Building Sayn from source repository..." -ForegroundColor Cyan
    cargo build --release --bin sayn --manifest-path "$ScriptDir\Cargo.toml"
    Copy-Item "$ScriptDir\target\release\sayn.exe" "$InstallDir\sayn.exe" -Force
} else {
    Write-Host "Downloading latest release binary for windows-x86_64..." -ForegroundColor Cyan
    $AssetUrl = "https://github.com/kingjethro999/sayn/releases/latest/download/sayn-windows-x86_64.zip"
    $ZipPath = "$env:TEMP\sayn-windows.zip"
    Invoke-WebRequest -Uri $AssetUrl -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force
    Remove-Item $ZipPath -Force
}

# 5. Add to User PATH if missing
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path += ";$InstallDir"
    Write-Host "✔ Added $InstallDir to user PATH" -ForegroundColor Green
}

Write-Host "`n✔ Sayn installed successfully!" -ForegroundColor Green
Write-Host "Run 'sayn --help' or 'sayn create --web my-project' to get started." -ForegroundColor Cyan
