# gen_status.ps1 — pre-process the Oracle case files into TSVs the
# coverage dashboard query (q1167) consumes.
#
# Emits:
#   coverage/cases_status.tsv  — Q<TAB>Status (one row per case)
#   coverage/case_names.tsv    — Q<TAB>Names (space-separated #shared
#                                names mentioned in that case's .m source).
#                                Pre-computed here rather than in M because
#                                the substring search is O(N_names × N_cases)
#                                and PowerShell does it in ~1 second.
#
# Both files are loaded by coverage.m via Csv.Document, which is fast and
# present in both engines.
#
# Usage: pwsh Oracle/coverage/gen_status.ps1
#        Run before QueryOracle.ps1 + capture_mrsflow.ps1.

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path  # Oracle/coverage
$oracleDir = Split-Path -Parent $root                    # Oracle
$casesDir = Join-Path $oracleDir 'cases'

# --- cases_status.tsv: invoke diff.ps1 and parse its DIFF list -----------

$diffOut = & pwsh -NoProfile -File (Join-Path $oracleDir 'diff.ps1') -ShowDiffs 0 2>&1 | Out-String

$diffNames = [System.Collections.Generic.HashSet[string]]::new()
$inDiff = $false
foreach ($line in $diffOut -split "`r?`n") {
    if ($line -match '^DIFF cases:')    { $inDiff = $true;  continue }
    if ($line -match '^MISSING cases:') { $inDiff = $false; continue }
    if (-not $inDiff) { continue }
    if ($line -match '^\s+(q\d+)\s*$') { [void]$diffNames.Add($Matches[1]) }
}

$allCases = Get-ChildItem $casesDir -Filter '*.m' |
    ForEach-Object { $_.BaseName } |
    Sort-Object { [int]($_ -replace 'q', '') }

$statusSb = [System.Text.StringBuilder]::new()
[void]$statusSb.AppendLine("Q`tStatus")
foreach ($q in $allCases) {
    $s = if ($diffNames.Contains($q)) { 'DIFF' } else { 'MATCH' }
    [void]$statusSb.AppendLine("$q`t$s")
}
$statusPath = Join-Path $root 'cases_status.tsv'
[System.IO.File]::WriteAllText($statusPath, $statusSb.ToString(),
    [System.Text.UTF8Encoding]::new($false))
Write-Output ("wrote {0} ({1} cases, {2} DIFF)" -f $statusPath, $allCases.Count, $diffNames.Count)

# --- case_names.tsv: substring-match every Excel #shared name vs every  ---
# --- case .m source. Heuristic: name followed by "(", ".", or ",".      ---

$sharedPath = Join-Path $casesDir 'q1165.excel.out'
if (-not (Test-Path $sharedPath)) {
    Write-Warning "no q1165.excel.out yet - case_names.tsv will be empty"
    $names = @()
} else {
    $names = [System.IO.File]::ReadAllText($sharedPath) -split "`r?`n" |
        Where-Object { $_ -match '\S' }
}

# Sort long-to-short so a longer prefix like "Text.From" is attributed
# before the bare "Text" identifier (which is a PQ type name).
$sortedNames = $names | Sort-Object -Property Length -Descending

$namesSb = [System.Text.StringBuilder]::new()
[void]$namesSb.AppendLine("Q`tNames")
foreach ($q in $allCases) {
    $src = [System.IO.File]::ReadAllText((Join-Path $casesDir ($q + '.m')))
    $hit = [System.Collections.Generic.List[string]]::new()
    foreach ($n in $sortedNames) {
        if ($src.Contains($n + '(') -or
            $src.Contains($n + '.') -or
            $src.Contains($n + ',')) {
            [void]$hit.Add($n)
            $src = $src.Replace($n, '')   # strip so shorter prefixes don't double-count
        }
    }
    $hitText = ($hit | Sort-Object) -join ' '
    [void]$namesSb.AppendLine("$q`t$hitText")
}

$namesPath = Join-Path $root 'case_names.tsv'
[System.IO.File]::WriteAllText($namesPath, $namesSb.ToString(),
    [System.Text.UTF8Encoding]::new($false))
Write-Output ("wrote {0} ({1} cases, {2} names checked)" -f $namesPath, $allCases.Count, $names.Count)
