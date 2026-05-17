# extract_cases.ps1 — for each `SafeSerialize("qN", () => <body>)` block
# in Oracle.m, write the body to cases/qN.m if no such file exists.
#
# The dashboard's name-occurrence scan (gen_status.ps1) walks cases/*.m.
# When new SafeSerialize entries are added directly to Oracle.m without
# also creating cases/qN.m, those names are invisible to coverage.
# This script reconciles.
#
# Output: cases/qN.m for each missing case, content = the M expression
# inside the SafeSerialize lambda body.

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path  # Oracle/coverage
$oracleDir = Split-Path -Parent $root                    # Oracle
$catalog = Join-Path $oracleDir 'Oracle.m'
$casesDir = Join-Path $oracleDir 'cases'

$text = [System.IO.File]::ReadAllText($catalog)

# Find every `SafeSerialize("qN", () =>` start, then walk parens until
# balanced — that's the lambda invocation. The lambda body is the
# content between `() =>` and the matching `)`.
$pattern = 'SafeSerialize\("(q\d+)",\s*\(\)\s*=>'
$matches = [regex]::Matches($text, $pattern)

$written = 0
$skipped = 0

foreach ($m in $matches) {
    $qid = $m.Groups[1].Value
    $outFile = Join-Path $casesDir ($qid + '.m')
    if (Test-Path $outFile) {
        $skipped++
        continue
    }

    # The lambda body starts right after the `=>`. Walk forward until
    # we've closed one paren that wasn't matched by an open — that's the
    # closing paren of the SafeSerialize call.
    $start = $m.Index + $m.Length
    $depth = 1
    $i = $start
    $inString = $false
    $strDelim = $null
    while ($i -lt $text.Length -and $depth -gt 0) {
        $c = $text[$i]
        if ($inString) {
            # M strings double `"` to escape. Skip a doubled-quote.
            if ($c -eq $strDelim -and $i + 1 -lt $text.Length -and $text[$i+1] -eq $strDelim) {
                $i += 2
                continue
            }
            if ($c -eq $strDelim) {
                $inString = $false
            }
        } else {
            if ($c -eq '"') {
                $inString = $true
                $strDelim = '"'
            }
            elseif ($c -eq '(') { $depth++ }
            elseif ($c -eq ')') { $depth-- }
        }
        $i++
    }

    if ($depth -ne 0) {
        Write-Warning "$qid : paren walk did not balance (skipping)"
        continue
    }

    # Body is from $start to $i-1 (i is one past the closing paren).
    $body = $text.Substring($start, $i - 1 - $start)
    # Trim leading/trailing whitespace and any common leading indent so
    # the per-case file reads cleanly.
    $body = $body.Trim()
    # De-indent: find the minimum leading-whitespace count across non-
    # blank lines and strip that prefix.
    $lines = $body -split "`r?`n"
    $minIndent = [int]::MaxValue
    foreach ($l in $lines) {
        if ($l -match '^\s*$') { continue }
        if ($l -match '^(\s*)') {
            $leading = $Matches[1].Length
            if ($leading -lt $minIndent) { $minIndent = $leading }
        }
    }
    if ($minIndent -eq [int]::MaxValue) { $minIndent = 0 }
    $deindented = ($lines | ForEach-Object {
        if ($_.Length -ge $minIndent) { $_.Substring($minIndent) } else { $_ }
    }) -join "`n"

    [System.IO.File]::WriteAllText($outFile, $deindented + "`n",
        [System.Text.UTF8Encoding]::new($false))
    $written++
}

Write-Output ("wrote {0} per-case files; skipped {1} existing" -f $written, $skipped)
