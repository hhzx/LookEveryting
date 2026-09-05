# Per-user install (no admin). Copies portable build into LocalAppData and creates a Start Menu shortcut.
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

& "$PSScriptRoot\build.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$src = Join-Path $Root "dist\LookEveryting"
if (-not (Test-Path "$src\LookEveryting.exe")) {
    Write-Error "Build output missing: $src\LookEveryting.exe"
    exit 1
}

$dest = Join-Path $env:LOCALAPPDATA "LookEveryting"
if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
Copy-Item $src $dest -Recurse

$exe = Join-Path $dest "LookEveryting.exe"
$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
New-Item -ItemType Directory -Path $startMenu -Force | Out-Null
$lnkPath = Join-Path $startMenu "LookEveryting.lnk"

$wsh = New-Object -ComObject WScript.Shell
$lnk = $wsh.CreateShortcut($lnkPath)
$lnk.TargetPath = $exe
$lnk.WorkingDirectory = $dest
$lnk.Description = "LookEveryting — local media viewer"
$lnk.Save()

Write-Host ""
Write-Host "Installed to: $dest"
Write-Host "Shortcut:     $lnkPath"
Write-Host "Launch:       $exe"
