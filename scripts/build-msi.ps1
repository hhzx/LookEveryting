# Build portable zip, then optionally produce an MSI via WiX Toolset if candle/light are on PATH.
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

& "$PSScriptRoot\build.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$candle = Get-Command candle -ErrorAction SilentlyContinue
$light = Get-Command light -ErrorAction SilentlyContinue
if (-not $candle -or -not $light) {
    Write-Warning "WiX Toolset (candle/light) not found on PATH."
    Write-Warning "Install WiX v3 and re-run, or use scripts\install.ps1 for per-user install."
    Write-Host "Portable package is ready: dist\LookEveryting-portable.zip"
    exit 0
}

New-Item -ItemType Directory -Path "dist\msi" -Force | Out-Null
Push-Location packaging
& candle -nologo -out "..\dist\msi\LookEveryting.wixobj" "LookEveryting.wxs"
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
& light -nologo -out "..\dist\LookEveryting.msi" "..\dist\msi\LookEveryting.wixobj"
$code = $LASTEXITCODE
Pop-Location
if ($code -ne 0) { exit $code }

Write-Host "MSI: dist\LookEveryting.msi"
