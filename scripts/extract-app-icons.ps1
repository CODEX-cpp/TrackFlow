<#
.SYNOPSIS
  Estrae l'icona reale di ogni .exe elencato in app-icons.json, la
  salva come PNG dentro static/app-icons/ — così TopSummary.vue può
  mostrarla con un normale <img>, senza dover leggere il filesystem
  dal browser (impossibile lato client) e senza doverla ri-estrarre
  a ogni avvio — e ne calcola anche il colore pastello/caldo (vedi
  color-utils.ps1), salvato in src/util/appIconColors.json. Un solo
  comando fa entrambe le cose per ogni app nuova o ri-estratta: non
  serve un secondo passaggio manuale per il colore.

.DESCRIZIONE
  Il browser non ha nessuna API per leggere l'icona di un eseguibile:
  questo script gira una volta (a mano, quando serve aggiungere o
  aggiornare un'app) DIRETTAMENTE sulla macchina Windows dove le app
  sono installate, usa .NET (System.Drawing) per estrarre l'icona
  associata a ogni percorso, e salva il risultato come file statico
  che aw-webui serve normalmente da static/app-icons/.

  L'elenco delle app è in scripts/app-icons.json — un file separato
  (non hardcoded qui) così aggiungere una nuova app significa solo
  aggiungere una riga lì e rilanciare lo script, senza toccare codice.

.USO
  powershell -File scripts/extract-app-icons.ps1

  Da rilanciare ogni volta che si aggiunge una voce nuova in
  app-icons.json, o se un'app viene reinstallata/aggiornata e si vuole
  un'icona più recente. Le icone già estratte non vengono ri-processate
  a meno di passare -Force (il colore segue la stessa regola: se
  un'icona viene saltata, il suo colore in appIconColors.json non
  viene toccato).
#>

param(
    [switch]$Force
)

Add-Type -AssemblyName System.Drawing

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$configPath = Join-Path $scriptDir "app-icons.json"
$repoRoot = Split-Path -Parent $scriptDir
$outputDir = Join-Path $repoRoot "static\app-icons"
$colorsPath = Join-Path $repoRoot "src\util\appIconColors.json"

. (Join-Path $scriptDir "color-utils.ps1")

if (-not (Test-Path $configPath)) {
    Write-Error "Non trovo $configPath"
    exit 1
}

if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

# Colori già calcolati in precedenza, per aggiornarli in-place invece
# di sovrascrivere l'intero file ad ogni run (le icone saltate perché
# già estratte devono tenersi il loro colore già calcolato).
$colors = [ordered]@{}
if (Test-Path $colorsPath) {
    $existing = Get-Content $colorsPath -Raw | ConvertFrom-Json
    foreach ($prop in $existing.PSObject.Properties) {
        $colors[$prop.Name] = $prop.Value
    }
}

$apps = Get-Content $configPath -Raw | ConvertFrom-Json

$estratte = 0
$saltate = 0
$fallite = 0

foreach ($app in $apps.PSObject.Properties) {
    $chiave = $app.Name
    $percorsi = $app.Value
    if ($percorsi -is [string]) { $percorsi = @($percorsi) }

    $destinazione = Join-Path $outputDir "$chiave.png"

    if ((Test-Path $destinazione) -and -not $Force) {
        Write-Host "Salto $chiave (già estratta, usa -Force per rifarla)"
        $saltate++
        continue
    }

    $percorsoTrovato = $null
    foreach ($p in $percorsi) {
        $espanso = [System.Environment]::ExpandEnvironmentVariables($p)
        if (Test-Path $espanso) {
            $percorsoTrovato = $espanso
            break
        }
    }

    if (-not $percorsoTrovato) {
        Write-Warning "Nessun percorso valido trovato per '$chiave' (provati: $($percorsi -join ', '))"
        $fallite++
        continue
    }

    try {
        $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($percorsoTrovato)
        if (-not $icon) {
            throw "ExtractAssociatedIcon ha restituito null"
        }
        $bitmap = $icon.ToBitmap()
        $bitmap.Save($destinazione, [System.Drawing.Imaging.ImageFormat]::Png)

        # Colore calcolato subito, dallo stesso bitmap già in memoria —
        # niente bisogno di rileggere il PNG da disco né di un secondo
        # script da lanciare a mano.
        $dominant = Get-DominantColor -Bitmap $bitmap
        if ($null -ne $dominant) {
            $color = Get-PastelWarmColor -Hue $dominant.Hue -Sat $dominant.Sat -Light $dominant.Light
            $colors[$chiave] = $color
            Write-Host "OK   $chiave <- $percorsoTrovato (colore: $color)"
        } else {
            Write-Host "OK   $chiave <- $percorsoTrovato (nessun colore abbastanza saturo, resta l'hash-per-nome)"
        }

        $bitmap.Dispose()
        $icon.Dispose()
        $estratte++
    } catch {
        Write-Warning "Estrazione fallita per '$chiave' ($percorsoTrovato): $_"
        $fallite++
    }
}

$colorsDir = Split-Path -Parent $colorsPath
if (-not (Test-Path $colorsDir)) {
    New-Item -ItemType Directory -Path $colorsDir | Out-Null
}
$colors | ConvertTo-Json | Set-Content -Path $colorsPath -Encoding utf8

Write-Host ""
Write-Host "Estratte: $estratte, saltate: $saltate, fallite: $fallite"
Write-Host "Cartella: $outputDir"
