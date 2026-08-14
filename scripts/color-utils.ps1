<#
.SYNOPSIS
  Funzioni pure per calcolare un colore "di marchio" pastello/caldo a
  partire da un'icona già estratta. Nessun effetto collaterale quando
  il file viene "dot-sourced" (`. .\color-utils.ps1`) — serve solo a
  condividere queste funzioni fra extract-app-icons.ps1 (le usa in
  automatico per ogni icona nuova) e extract-app-colors.ps1 (le usa per
  ricalcolare tutti i colori in blocco dalle PNG già su disco, utile se
  si cambia la formula qui sotto senza voler ri-estrarre le icone).
#>

function ConvertFrom-Hsl {
    param([double]$H, [double]$S, [double]$L)
    $h = $H / 360.0
    if ($S -eq 0) {
        $r = $L; $g = $L; $b = $L
    } else {
        function HueToRgb([double]$p, [double]$q, [double]$t) {
            if ($t -lt 0) { $t += 1 }
            if ($t -gt 1) { $t -= 1 }
            if ($t -lt (1.0 / 6)) { return $p + ($q - $p) * 6 * $t }
            if ($t -lt (1.0 / 2)) { return $q }
            if ($t -lt (2.0 / 3)) { return $p + ($q - $p) * ((2.0 / 3) - $t) * 6 }
            return $p
        }
        $q = if ($L -lt 0.5) { $L * (1 + $S) } else { $L + $S - $L * $S }
        $p = 2 * $L - $q
        $r = HueToRgb $p $q ($h + (1.0 / 3))
        $g = HueToRgb $p $q $h
        $b = HueToRgb $p $q ($h - (1.0 / 3))
    }
    return @([Math]::Round($r * 255), [Math]::Round($g * 255), [Math]::Round($b * 255))
}

# Blend circolare fra due tonalità (0-360), via più breve sul cerchio.
function Get-BlendedHue {
    param([double]$H1, [double]$H2, [double]$T)
    $diff = (($H2 - $H1 + 540) % 360) - 180
    $result = $H1 + $diff * $T
    return (($result % 360) + 360) % 360
}

# Tonalità dominante via istogramma a bucket di 10°, ignorando pixel
# trasparenti e quasi grigi/bianchi/neri (di solito bordo/sfondo
# dell'icona, non il suo colore caratteristico).
function Get-DominantColor {
    param([System.Drawing.Bitmap]$Bitmap)

    $thumb = New-Object System.Drawing.Bitmap 40, 40
    $g = [System.Drawing.Graphics]::FromImage($thumb)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($Bitmap, 0, 0, 40, 40)
    $g.Dispose()

    $buckets = @{}
    for ($x = 0; $x -lt 40; $x++) {
        for ($y = 0; $y -lt 40; $y++) {
            $px = $thumb.GetPixel($x, $y)
            if ($px.A -lt 128) { continue }
            $sat = $px.GetSaturation()
            $light = $px.GetBrightness()
            if ($sat -lt 0.15 -or $light -lt 0.12 -or $light -gt 0.92) { continue }
            $hue = $px.GetHue()
            $bucketIdx = [int]([Math]::Floor($hue / 10)) % 36
            if (-not $buckets.ContainsKey($bucketIdx)) {
                $buckets[$bucketIdx] = @{ Count = 0; HueSum = 0.0; SatSum = 0.0; LightSum = 0.0 }
            }
            $buckets[$bucketIdx].Count++
            $buckets[$bucketIdx].HueSum += $hue
            $buckets[$bucketIdx].SatSum += $sat
            $buckets[$bucketIdx].LightSum += $light
        }
    }
    $thumb.Dispose()

    if ($buckets.Count -eq 0) { return $null }

    $winner = $buckets.GetEnumerator() | Sort-Object { $_.Value.Count } -Descending | Select-Object -First 1
    $b = $winner.Value
    return @{
        Hue   = $b.HueSum / $b.Count
        Sat   = $b.SatSum / $b.Count
        Light = $b.LightSum / $b.Count
    }
}

# Ammorbidisce il colore dominante in pastello e gli dà un tocco caldo
# LEGGERO e uniforme — deliberatamente leggero: la riconoscibilità del
# logo originale conta più della coerenza col resto della palette
# calda (regola scoperta essere sbagliata qui: una prima versione
# spingeva forte verso l'arancione per evitare la fascia viola/magenta
# vietata nel resto del tema, ma il risultato per app dal logo blu
# — VS Code, RealVNC, Discord — non si riconosceva più come "quel
# colore lì, ma tenue": diventava verde o rosa. Tolta quella spinta
# forte, resta solo un nudge leggero uniforme.
function Get-PastelWarmColor {
    param([double]$Hue, [double]$Sat, [double]$Light)

    $targetSat = [Math]::Max(0.30, [Math]::Min($Sat, 0.50))
    $targetLight = [Math]::Max(0.55, [Math]::Min($Light, 0.68))

    $warmAnchor = 30.0
    $finalHue = Get-BlendedHue $Hue $warmAnchor 0.15

    $rgb = ConvertFrom-Hsl $finalHue $targetSat $targetLight
    return ('#{0:X2}{1:X2}{2:X2}' -f [int]$rgb[0], [int]$rgb[1], [int]$rgb[2])
}
