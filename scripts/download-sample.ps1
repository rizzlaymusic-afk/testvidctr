#!/usr/bin/env pwsh
# downloads a small CC0 test video into assets/sample/sample.webm
param(
    [string]$Url = 'https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4',
    [string]$Out = '..\assets\sample\sample.webm'
)
Write-Host "Downloading sample from $Url -> $Out"
try {
    if (Test-Path $Out) { Remove-Item $Out -Force }
    Invoke-WebRequest -Uri $Url -OutFile $Out -UseBasicParsing -TimeoutSec 60
    Write-Host "Downloaded sample to $Out"
}
catch {
    Write-Error "Failed to download sample: $_"
    exit 1
}
