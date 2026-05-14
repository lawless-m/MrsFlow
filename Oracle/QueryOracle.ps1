# QueryOracle.ps1 — refresh Oracle.xlsx and dump the Catalog table.
#
# Oracle.m loads a single 2-column ListObject named "Catalog" with
# rows {Q = "q1", Result = "..."} ... Each Result is a single text
# cell produced by Oracle.Serialize inside the workbook, so this
# script never has to know per-case shapes — it just walks rows.
#
# Output:
#   - stdout: one line per row, `qN<TAB><result>` with embedded
#     newlines/tabs in the result escaped (\n and \t). Suitable for
#     redirection + line-by-line diffing.
#   - cases/<qN>.excel.out: raw result text per row, one file each.
#     Matches the cases/<qN>.mrsflow.out layout for direct diffing
#     (once the mrsflow side adopts the same Oracle.Serialize shape).
#
# Adding a test = add a row to Oracle.m's `cases` list. No PS1 change
# needed.

$path = Join-Path $PSScriptRoot 'Oracle.xlsx'
$casesDir = Join-Path $PSScriptRoot 'cases'

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false

function Escape-Inline {
    param([string]$s)
    if ($null -eq $s) { return '' }
    return ($s -replace "`r`n", '\n') -replace "`n", '\n' -replace "`t", '\t'
}

try {
    $wb = $excel.Workbooks.Open($path)
    $wb.RefreshAll()
    $excel.CalculateUntilAsyncQueriesDone()
    $wb.Save()

    # Locate the catalog ListObject by COLUMN SHAPE, not by name. PQ
    # "Load To Table" names the LO after the query, and the query name
    # is whatever the workbook author saved (e.g. "Invoked_FunctionEvalFile"
    # for the EvalFile-wrapper setup). Matching on `Q` + `Result` headers
    # works regardless of rename.
    $catalog = $null
    foreach ($sheet in $wb.Sheets) {
        foreach ($lo in $sheet.ListObjects) {
            $headers = @($lo.HeaderRowRange.Cells | ForEach-Object { [string]$_.Value2 })
            if ($headers.Count -ge 2 -and $headers[0] -eq 'Q' -and $headers[1] -eq 'Result') {
                $catalog = $lo
                break
            }
        }
        if ($catalog) { break }
    }

    if (-not $catalog) {
        Write-Error 'No Q/Result ListObject found. Did Oracle.m load successfully?'
        exit 1
    }

    $body = $catalog.DataBodyRange
    if (-not $body) {
        Write-Error 'Catalog has no rows.'
        exit 1
    }

    $rows = $body.Rows.Count
    for ($r = 1; $r -le $rows; $r++) {
        # Column order matches Oracle.m's Table.FromRecords: Q then Result.
        $q = [string]$body.Cells.Item($r, 1).Value2
        $result = [string]$body.Cells.Item($r, 2).Value2
        if ([string]::IsNullOrEmpty($q)) { continue }

        $outFile = Join-Path $casesDir ($q + '.excel.out')
        Set-Content -Path $outFile -Value $result -NoNewline -Encoding UTF8

        Write-Output ("{0}`t{1}" -f $q, (Escape-Inline $result))
    }

    $wb.Close($false)
}
finally {
    $excel.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($excel) | Out-Null
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}
