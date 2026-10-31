# Morph Windows Installer (MSI) Build Script
# Usage: powershell -ExecutionPolicy Bypass -File .\installer\build_msi.ps1

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Building Morph Windows Installer (.msi) " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Ensure Cargo release binaries are up-to-date
Write-Host "`n[1/3] Building release binaries via Cargo..." -ForegroundColor Yellow
Push-Location $rootDir
try {
    # Release file locks if a previous process is still lingering
    Get-Process "morph-ui", "morph" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 200
    if (Test-Path "$rootDir\target\release\morph-ui.exe") {
        try {
            [System.IO.File]::OpenWrite("$rootDir\target\release\morph-ui.exe").Close()
        } catch {
            Move-Item "$rootDir\target\release\morph-ui.exe" "$rootDir\target\release\morph-ui.old.exe" -Force -ErrorAction SilentlyContinue
        }
    }

    cargo build --release -p morph-cli -p morph-ui
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with code $LASTEXITCODE" }
    Remove-Item "$rootDir\target\release\morph-ui.old.exe" -Force -ErrorAction SilentlyContinue
} finally {
    Pop-Location
}

# 2. Locate wix.exe
$wixExe = "$HOME\.dotnet\tools\wix.exe"
if (-not (Test-Path $wixExe)) {
    $found = Get-Command "wix" -ErrorAction SilentlyContinue
    if ($found) {
        $wixExe = $found.Source
    } else {
        throw "wix.exe not found! Please run: dotnet tool install --global wix --version 4.0.6"
    }
}
Write-Host "[2/3] Using WiX tool: $wixExe" -ForegroundColor Yellow

# 3. Compile MSI package
$outMsi = Join-Path $rootDir "dist\Morph-v0.1.0-windows-x64.msi"
$wxsFile = Join-Path $scriptDir "morph.wxs"

Write-Host "[3/4] Compiling WiX MSI package to $outMsi..." -ForegroundColor Yellow
Push-Location $scriptDir
try {
    & $wixExe build -ext WixToolset.UI.wixext $wxsFile -out $outMsi
    if ($LASTEXITCODE -ne 0) { throw "WiX MSI build failed with code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# 4. Compile EXE Bootstrapper with embedded Morph Logo Icon
$outExe = Join-Path $rootDir "dist\MorphSetup-v0.1.0-windows-x64.exe"
$bundleFile = Join-Path $scriptDir "bundle.wxs"

Write-Host "[4/4] Compiling WiX Bootstrapper EXE (with embedded logo) to $outExe..." -ForegroundColor Yellow
Push-Location $scriptDir
try {
    & $wixExe build -ext WixToolset.Bal.wixext $bundleFile -out $outExe
    if ($LASTEXITCODE -ne 0) { throw "WiX Bootstrapper EXE build failed with code $LASTEXITCODE" }
} finally {
    Pop-Location
}

# Verification and Checksum
$msiItem = Get-Item $outMsi
$msiSha = (Get-FileHash -Path $outMsi -Algorithm SHA256).Hash
$msiMb = [math]::Round($msiItem.Length / 1MB, 2)

$exeItem = Get-Item $outExe
$exeSha = (Get-FileHash -Path $outExe -Algorithm SHA256).Hash
$exeMb = [math]::Round($exeItem.Length / 1MB, 2)

Write-Host "`n========================================================" -ForegroundColor Green
Write-Host " Installer Release Packages Built Successfully!" -ForegroundColor Green
Write-Host " 1. MSI Installer: $outMsi ($msiMb MB)" -ForegroundColor White
Write-Host "    SHA-256:       $msiSha" -ForegroundColor Gray
Write-Host " 2. Setup EXE:     $outExe ($exeMb MB) [With Logo Icon]" -ForegroundColor White
Write-Host "    SHA-256:       $exeSha" -ForegroundColor Gray
Write-Host "========================================================" -ForegroundColor Green
