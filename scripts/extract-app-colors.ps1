<#
.SYNOPSIS
  Ricalcola in blocco il colore "di marchio" di TUTTE le icone già
  presenti in static/app-icons/ e riscrive src/util/appIconColors.json
  da zero.

.DESCRIZIONE
  Normalmente non serve lanciare questo script a mano: extract-app-
  icons.ps1 calcola già il colore in automatico per ogni icona nuova o
  ri-estratta con -Force, nello stesso passaggio (vedi quel file). Usa
  questo script solo quando serve ricalcolare i colori di icone GIA'
  presenti SENZA ri-estrarle (es. dopo aver modificato la formula in
  color-utils.ps1) — altrimenti bisognerebbe rilanciare extract-app-
  icons.ps1 con -Force su tutto solo per far ripartire il calcolo
  colore, ributtando via anche le icone già buone.

.USO
  powershell -File scripts/extract-app-colors.ps1
#>

Add-Type -AssemblyName System.Drawing

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$iconsDir = Join-Path $repoRoot "static\app-icons"
$outputPath = Join-Path $repoRoot "src\util\appIconColors.json"

. (Join-Path $scriptDir "color-utils.ps1")

if (-not (Test-Path $iconsDir)) {
    Write-Error "Non trovo $iconsDir - esegui prima extract-app-icons.ps1"
    exit 1
}

$results = [ordered]@{}
$pngFiles = Get-ChildItem -Path $iconsDir -Filter "*.png" | Sort-Object Name

foreach ($file in $pngFiles) {
    $key = $file.BaseName
    try {
        $bitmap = New-Object System.Drawing.Bitmap $file.FullName
        $dominant = Get-DominantColor -Bitmap $bitmap
        $bitmap.Dispose()

        if ($null -eq $dominant) {
            Write-Warning "Nessun colore abbastanza saturo in $key (icona neutra/grigia) - salto, resta il colore hash-per-nome"
            continue
        }

        $color = Get-PastelWarmColor -Hue $dominant.Hue -Sat $dominant.Sat -Light $dominant.Light
        $results[$key] = $color
        $hueRounded = [Math]::Round($dominant.Hue)
        Write-Host "OK   $key -> $color (hue sorgente: ${hueRounded} gradi)"
    } catch {
        Write-Warning "Estrazione colore fallita per '$key': $_"
    }
}

$outputDir = Split-Path -Parent $outputPath
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}
$results | ConvertTo-Json | Set-Content -Path $outputPath -Encoding utf8

Write-Host ""
Write-Host "Salvato: $outputPath ($($results.Count) colori)"
