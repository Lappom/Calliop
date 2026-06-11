# Generates French benchmark WAV files via Windows SAPI (16 kHz mono via ffmpeg if available).
param(
    [string]$OutputDir = (Join-Path $PSScriptRoot "..\benchmarks\corpus")
)

$ErrorActionPreference = "Stop"
$manifestPath = Join-Path $OutputDir "fr.json"
if (-not (Test-Path $manifestPath)) {
    throw "Missing manifest: $manifestPath"
}

$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer

foreach ($sample in $manifest.samples) {
    $rawPath = Join-Path $OutputDir ("_{0}" -f $sample.wav)
    $outPath = Join-Path $OutputDir $sample.wav
    Write-Host "Synthesizing $($sample.id)..."
    $synth.SetOutputToWaveFile($rawPath)
    $synth.Speak($sample.reference)
    $synth.SetOutputToDefaultAudioDevice()

    if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
        ffmpeg -y -i $rawPath -ar 16000 -ac 1 $outPath | Out-Null
        Remove-Item $rawPath -Force
    } else {
        Move-Item -Force $rawPath $outPath
        Write-Warning "ffmpeg not found; kept native SAPI sample rate for $($sample.wav)"
    }
}

Write-Host "Done. Corpus written to $OutputDir"
