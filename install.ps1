# costcrew installer for Windows
# powershell -c "irm https://raw.githubusercontent.com/Arch087146/costcrew/main/install.ps1 | iex"
$ErrorActionPreference = "Stop"

$Repo = "Arch087146/costcrew"
$Asset = "costcrew-windows-x86_64.zip"
$Url = "https://github.com/$Repo/releases/latest/download/$Asset"

$Dest = if ($env:COSTCREW_INSTALL_DIR) {
    $env:COSTCREW_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Programs\costcrew"
}
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

$tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "costcrew-install")
$zip = Join-Path $tmp $Asset

Write-Host "fetching $Asset ..."
Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing
Expand-Archive -Path $zip -DestinationPath $tmp -Force

Move-Item (Join-Path $tmp "costcrew.exe") (Join-Path $Dest "costcrew.exe") -Force

# make sure it's on PATH for future sessions
$dirOnPath = [Environment]::GetEnvironmentVariable("Path", "User") -split ";" -contains $Dest
if (-not $dirOnPath) {
    [Environment]::SetEnvironmentVariable(
        "Path",
        [Environment]::GetEnvironmentVariable("Path", "User") + ";$Dest",
        "User"
    )
    Write-Host "added $Dest to your user PATH (new shells will see it)"
}

& (Join-Path $Dest "costcrew.exe") --version
Write-Host ""
Write-Host "run  costcrew  in a NEW shell to open the app"
