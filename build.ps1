# Zohara OS - Persistent Build Script
# 1. If image exists in Docker - skip build
# 2. If saved .tar exists - load from disk instantly
# 3. Otherwise build from scratch and save to disk
# 4. Run the ISO build with all caches mounted

param(
    [switch]$Rebuild
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$IMAGE_NAME  = "zohara-builder"
$IMAGE_TAG   = "latest"
$IMAGE_FULL  = "${IMAGE_NAME}:${IMAGE_TAG}"
$SAVED_IMAGE = Join-Path $PSScriptRoot "zohara-builder-image.tar"

function Test-DockerRunning {
    try { docker info 2>&1 | Out-Null; return $true }
    catch { return $false }
}

Write-Host "Checking Docker Desktop..." -ForegroundColor Cyan
if (-not (Test-DockerRunning)) {
    Write-Host "Starting Docker Desktop..." -ForegroundColor Yellow
    Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"
    $timeout = 90
    while (-not (Test-DockerRunning) -and $timeout -gt 0) {
        Start-Sleep -Seconds 3
        $timeout -= 3
        Write-Host "  Waiting for Docker engine... ($timeout s remaining)" -ForegroundColor DarkGray
    }
    if (-not (Test-DockerRunning)) {
        Write-Error "Docker Desktop did not start. Please start it manually."
        exit 1
    }
}
Write-Host "Docker is running." -ForegroundColor Green

$imageExists = if ($Rebuild) { $null } else { docker images -q $IMAGE_FULL 2>$null }
if ($imageExists) {
    Write-Host "Builder image already in Docker - skipping build." -ForegroundColor Green
} elseif ((-not $Rebuild) -and (Test-Path $SAVED_IMAGE)) {
    Write-Host "Loading saved image from disk..." -ForegroundColor Cyan
    docker load -i $SAVED_IMAGE
    Write-Host "Image loaded." -ForegroundColor Green
} else {
    Write-Host "Building from scratch (with BuildKit cache)..." -ForegroundColor Yellow
    $env:DOCKER_BUILDKIT = "1"
    docker build -t $IMAGE_FULL .
    Write-Host "Saving image to disk for future use..." -ForegroundColor Cyan
    docker save -o $SAVED_IMAGE $IMAGE_FULL
    $sizeGB = [math]::Round((Get-Item $SAVED_IMAGE).Length / 1GB, 2)
    Write-Host "Image saved ($sizeGB GB)." -ForegroundColor Green
}

New-Item -ItemType Directory -Force -Path "pkg-cache" | Out-Null

Write-Host ""
Write-Host "Building Zohara OS ISO..." -ForegroundColor Cyan

docker run --rm `
    --name zohara-build `
    --privileged `
    -v "${PSScriptRoot}:/build" `
    -v "${PSScriptRoot}/pkg-cache:/var/cache/pacman/pkg" `
    $IMAGE_FULL

Write-Host ""
Write-Host "ISO build complete!" -ForegroundColor Green
Write-Host "Output: $PSScriptRoot\out\" -ForegroundColor Cyan
Get-ChildItem -Path "out" -Filter "*.iso" | ForEach-Object {
    $gb = [math]::Round($_.Length / 1GB, 2)
    Write-Host ("  " + $_.Name + " - " + $gb + " GB")
}