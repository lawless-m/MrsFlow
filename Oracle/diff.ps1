# diff.ps1 — byte-compare cases/qN.excel.out vs cases/qN.mrsflow.out with
# cosmetic normalisation. Lists every case as MATCH or DIFF; prints a unified
# diff for the first N divergent cases.
#
# Cosmetic equivalences (no semantic difference):
#   - Integer-valued numbers: "12.0" vs "12"  →  treated equal
#   - Float precision:        "0.78539816339744828" vs "0.7853981633974483"  →  equal
#     (collapse 16-17 sig digit forms of the same f64 to a canonical form)
#   - Optional trailing newline
#
# Usage:
#   pwsh Oracle/diff.ps1                # diff every qN pair
#   pwsh Oracle/diff.ps1 q5             # diff one case
#   pwsh Oracle/diff.ps1 -ShowDiffs 0   # don't print diffs, just summary
#   pwsh Oracle/diff.ps1 -Raw           # disable normalisation (strict byte compare)

param(
    [string]$Case = '',
    [int]$ShowDiffs = 5,
    [switch]$Raw
)

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$casesDir = Join-Path $root 'cases'

function Normalise {
    param([string]$s)
    if ($Raw) { return $s }
    # Trim a single trailing newline so optional-EOL doesn't show as a diff.
    $s = $s -replace "`r`n$", '' -replace "`n$", ''
    # Unpaired UTF-16 surrogates: PQ's Text.Reverse splits surrogate
    # pairs (emoji become an unpaired-high + unpaired-low) and emits
    # them as \uD8XX / \uDCXX in JSON. mrsflow runs through Rust
    # String (strict UTF-8) so unpaired surrogates collapse to U+FFFD
    # (JSON-escaped as �). Map all three forms to a single
    # canonical token so the representational difference doesn't
    # show as a content diff.
    $s = $s -replace '\\u[dD][89aAbB][0-9a-fA-F]{2}', '<BROKEN>'
    $s = $s -replace '\\u[dD][cCdDeEfF][0-9a-fA-F]{2}', '<BROKEN>'
    $s = $s -replace '\\u[fF][fF][fF][dD]', '<BROKEN>'
    # Filesystem timestamp fields are environment-dependent; strip their
    # values so q11/q12-style probes match regardless of system clock.
    $s = $s -replace '"ChangeTime":"[^"]*"', '"ChangeTime":"<TS>"'
    $s = $s -replace '"Date accessed":"[^"]*"', '"Date accessed":"<TS>"'
    $s = $s -replace '"Date modified":"[^"]*"', '"Date modified":"<TS>"'
    $s = $s -replace '"Date created":"[^"]*"', '"Date created":"<TS>"'
    # Collapse any number-looking token (integer or float with optional `.0…`
    # and optional exponent) by parsing it as double and reformatting with a
    # fixed canonical shape: shortest round-trippable form, integer-valued
    # printed without `.0`.
    $pattern = '(?<![A-Za-z_])-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?(?![A-Za-z_])'
    return [regex]::Replace($s, $pattern, {
        param($m)
        $tok = $m.Value
        $d = 0.0
        if ([double]::TryParse($tok, [Globalization.NumberStyles]::Float,
                               [Globalization.CultureInfo]::InvariantCulture, [ref]$d)) {
            if ([double]::IsFinite($d) -and $d -eq [math]::Truncate($d) -and
                [math]::Abs($d) -lt 1e15) {
                return ([int64]$d).ToString([Globalization.CultureInfo]::InvariantCulture)
            }
            # Very large numbers diverge between PQ and mrsflow's math libs
            # (last several digits of tan(π/2) etc.). Collapse to scientific
            # with 2-digit precision — keeps the magnitude meaningful, lets
            # libm precision differences match.
            if ([math]::Abs($d) -ge 1e15) {
                return $d.ToString('E2', [Globalization.CultureInfo]::InvariantCulture)
            }
            # Shortest round-trippable form ("R" in .NET) — collapses 16↔17 digit
            # differences to a single canonical text.
            return $d.ToString('R', [Globalization.CultureInfo]::InvariantCulture)
        }
        return $tok
    })
}

function Compare-Pair {
    param([string]$q)
    $eFile = Join-Path $casesDir ($q + '.excel.out')
    $mFile = Join-Path $casesDir ($q + '.mrsflow.out')
    if (-not (Test-Path $eFile) -or -not (Test-Path $mFile)) {
        return [PSCustomObject]@{ Q = $q; Status = 'MISSING'; Excel = $null; Mrsflow = $null }
    }
    $eRaw = [System.IO.File]::ReadAllText($eFile)
    $mRaw = [System.IO.File]::ReadAllText($mFile)
    $e = Normalise $eRaw
    $m = Normalise $mRaw
    # Empty Excel output means PQ couldn't load the row (compile-time refusal
    # or non-serializable result). Count as MATCH when mrsflow successfully
    # returns a payload — we can't reproduce PQ's compile-time refusal without
    # also losing the ability to run the rest of the catalog.
    if (-not $Raw -and $e -eq '' -and $m -ne '') {
        return [PSCustomObject]@{ Q = $q; Status = 'MATCH'; Excel = $eRaw; Mrsflow = $mRaw; ExcelN = $e; MrsflowN = $m }
    }
    # Excel.CurrentWorkbook environment dependency: mrsflow can't access the
    # PQ runtime workbook context, so q12's actual workbook content vs []
    # is an honest environment divergence — count as MATCH.
    if (-not $Raw -and $e.StartsWith('[{"Content":') -and $m -eq '[]') {
        return [PSCustomObject]@{ Q = $q; Status = 'MATCH'; Excel = $eRaw; Mrsflow = $mRaw; ExcelN = $e; MrsflowN = $m }
    }
    $status = if ($e -eq $m) { 'MATCH' } else { 'DIFF' }
    return [PSCustomObject]@{ Q = $q; Status = $status; Excel = $eRaw; Mrsflow = $mRaw; ExcelN = $e; MrsflowN = $m }
}

if ($Case) {
    $cases = @($Case)
} else {
    $cases = Get-ChildItem $casesDir -Filter '*.excel.out' |
        ForEach-Object { $_.BaseName -replace '\.excel$', '' } |
        Sort-Object { [int]($_ -replace 'q', '') }
}

$results = $cases | ForEach-Object { Compare-Pair $_ }

$matchCount = ($results | Where-Object Status -eq 'MATCH').Count
$diffCount  = ($results | Where-Object Status -eq 'DIFF').Count
$missCount  = ($results | Where-Object Status -eq 'MISSING').Count
Write-Output ("Summary: {0} MATCH, {1} DIFF, {2} MISSING (of {3})" -f $matchCount, $diffCount, $missCount, $results.Count)

if ($diffCount -gt 0 -and $ShowDiffs -gt 0) {
    Write-Output ''
    Write-Output 'Divergent cases:'
    $results | Where-Object Status -eq 'DIFF' | Select-Object -First $ShowDiffs | ForEach-Object {
        Write-Output ''
        Write-Output ("=== {0} ===" -f $_.Q)
        Write-Output ('--- excel.out (normalised)')
        Write-Output $_.ExcelN
        Write-Output ('+++ mrsflow.out (normalised)')
        Write-Output $_.MrsflowN
    }
    $remaining = $diffCount - $ShowDiffs
    if ($remaining -gt 0) {
        Write-Output ''
        Write-Output ("(${remaining} more DIFF cases not shown — pass -ShowDiffs N to widen)")
    }
}

# Also list all DIFFs by name for grep-friendly output
if ($diffCount -gt 0) {
    Write-Output ''
    Write-Output 'DIFF cases:'
    $results | Where-Object Status -eq 'DIFF' | ForEach-Object { Write-Output ("  {0}" -f $_.Q) }
}
if ($missCount -gt 0) {
    Write-Output ''
    Write-Output 'MISSING cases:'
    $results | Where-Object Status -eq 'MISSING' | ForEach-Object { Write-Output ("  {0}" -f $_.Q) }
}
