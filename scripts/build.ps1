$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "[1/4] Running tests..."
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/4] Building release..."
cargo build --release -p look-everyting
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[3/4] Packaging..."
$dist = Join-Path $PWD "dist\LookEveryting"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Path $dist | Out-Null
Copy-Item "target\release\LookEveryting.exe" "$dist\LookEveryting.exe"
Copy-Item "locales" "$dist\locales" -Recurse
Copy-Item "design" "$dist\design" -Recurse
Copy-Item "README.md" "$dist\README.md"

# Bundle CJK fonts so portable dist renders Chinese without system fonts.
$fontDist = Join-Path $dist "fonts"
New-Item -ItemType Directory -Path $fontDist -Force | Out-Null
$fontCandidates = @(
    (Join-Path $PWD "assets\fonts\NotoSansSC-Regular.otf"),
    (Join-Path $PWD "assets\fonts\NotoSansSC-Regular.ttf"),
    (Join-Path $env:WINDIR "Fonts\msyh.ttc"),
    (Join-Path $env:WINDIR "Fonts\simhei.ttf")
)
foreach ($src in $fontCandidates) {
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $fontDist (Split-Path $src -Leaf))
        Write-Host "Bundled font: $src"
        break
    }
}
if (-not (Get-ChildItem $fontDist -ErrorAction SilentlyContinue)) {
    Write-Warning "No CJK font bundled. Place NotoSansSC-Regular.otf in assets/fonts or build on Windows with CJK system fonts."
}

Write-Host "[4/4] Done."
$size = (Get-Item "$dist\LookEveryting.exe").Length
Write-Host ""
Write-Host "Done: $dist\LookEveryting.exe"
Write-Host "Size: $size bytes ($([math]::Round($size/1MB, 2)) MB)"
