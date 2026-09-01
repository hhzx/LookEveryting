$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "[1/3] Running tests..."
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/3] Building release..."
cargo build --release -p look-everyting
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[3/3] Packaging..."
$dist = Join-Path $PWD "dist\LookEveryting"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Path $dist | Out-Null
Copy-Item "target\release\LookEveryting.exe" "$dist\LookEveryting.exe"
Copy-Item "locales" "$dist\locales" -Recurse
Copy-Item "design" "$dist\design" -Recurse
Copy-Item "README.md" "$dist\README.md"

$size = (Get-Item "$dist\LookEveryting.exe").Length
Write-Host ""
Write-Host "Done: $dist\LookEveryting.exe"
Write-Host "Size: $size bytes ($([math]::Round($size/1MB, 2)) MB)"
