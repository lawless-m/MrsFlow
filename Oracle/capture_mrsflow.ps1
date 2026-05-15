# capture_mrsflow.ps1 — run Oracle.m through mrsflow, parse the
# resulting Catalog s-expression, and write per-case .mrsflow.out
# files in the same Oracle.Serialize text shape that
# QueryOracle.ps1 writes for .excel.out. This makes the two
# sides byte-comparable.

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$casesDir = Join-Path $root 'cases'
$oracleFile = Join-Path $root 'Oracle.m'
$mrsflow = Join-Path (Split-Path $root) 'target\release\mrsflow.exe'

# Only capture stdout; mrsflow writes diagnostic noise (e.g. the
# Odbc.Query columnar-fallback notice) to stderr, which would break
# the s-expression parser below if interleaved.
$output = & $mrsflow $oracleFile --sexpr 2>$null | Out-String

# Rows look like: ((text "qN") (text "<escaped-json>"))
# Match the two (text ...) groups; the second's body is the Result.
$pattern = '\(\(text "(q\d+)"\)\s+\(text "((?:[^"\\]|\\.)*)"\)\)'
$matches = [regex]::Matches($output, $pattern)

if ($matches.Count -eq 0) {
    Write-Error 'No catalog rows parsed from mrsflow output. First 500 chars:'
    Write-Error $output.Substring(0, [Math]::Min(500, $output.Length))
    exit 1
}

foreach ($m in $matches) {
    $q = $m.Groups[1].Value
    $raw = $m.Groups[2].Value
    # Unescape s-expression string. Order matters: handle \\ as a placeholder
    # first so the other escapes don't double-process its trailing char.
    $placeholder = [char]0x1
    $decoded = $raw.Replace('\\', [string]$placeholder)
    $decoded = $decoded -replace '\\"', '"'
    $decoded = $decoded -replace '\\r', "`r"
    $decoded = $decoded -replace '\\n', "`n"
    $decoded = $decoded -replace '\\t', "`t"
    $decoded = $decoded.Replace([string]$placeholder, '\')
    $outFile = Join-Path $casesDir ($q + '.mrsflow.out')
    [System.IO.File]::WriteAllText($outFile, $decoded, [System.Text.UTF8Encoding]::new($false))
    Write-Output ("{0,-5} {1}" -f $q, $decoded.Substring(0, [Math]::Min(80, $decoded.Length)))
}
