# run_one.ps1 — drive QueryOracle.ps1 against a single cases/qN.m by
# wrapping the body in a single-row Catalog shape, refreshing, then
# writing the result to cases/qN.excel.out.
#
# Usage: pwsh run_one.ps1 q5
# Or:    pwsh run_one.ps1 q5 -TimeoutSec 60

param(
    [Parameter(Mandatory=$true)][string]$Q,
    [int]$TimeoutSec = 30
)

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$casesDir = Join-Path $root 'cases'
$caseFile = Join-Path $casesDir ($Q + '.m')
$oracleFile = Join-Path $root 'Oracle.m'

if (-not (Test-Path $caseFile)) {
    Write-Error "Case file not found: $caseFile"
    exit 2
}

$body = Get-Content -Raw $caseFile

$wrapped = @"
let
    Oracle.Serialize = (v as any) as text =>
        if v = null then "null"
        else if v is text then v
        else if v is number then Number.ToText(v, "G", "en-US")
        else if v is logical then (if v then "true" else "false")
        else Text.FromBinary(Json.FromValue(v), TextEncoding.Utf8),

    SafeSerialize = (label as text, expr as function) as record =>
        let
            r = try expr()
        in
            if r[HasError]
                then [Q = label, Result = "ERROR: " & r[Error][Message]]
                else [Q = label, Result = Oracle.Serialize(r[Value])],

    Catalog = Table.FromRecords({
        SafeSerialize("$Q", () => $body)
    })
in
    Catalog
"@

# Kill any lingering Excel before we start.
Get-Process EXCEL -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Set-Content -Path $oracleFile -Value $wrapped -Encoding UTF8

$job = Start-Job -ScriptBlock {
    param($scriptPath)
    & $scriptPath 2>&1
} -ArgumentList (Join-Path $root 'QueryOracle.ps1')

$ok = Wait-Job $job -Timeout $TimeoutSec
if (-not $ok) {
    Stop-Job $job
    Remove-Job $job -Force
    Get-Process EXCEL -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Output ("{0}`tTIMEOUT after {1}s" -f $Q, $TimeoutSec)
    exit 1
}

$out = Receive-Job $job
Remove-Job $job -Force
Get-Process EXCEL -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Output $out
