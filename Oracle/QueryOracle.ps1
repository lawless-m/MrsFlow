# Refresh Oracle.xlsx and dump every workbook Name to stdout. Used as
# the ground-truth oracle for differential testing against mrsflow.
#
# Output format: one Name per block. Single-cell values print as
#   <name>=<value>
# Multi-cell ranges print as
#   <name> [<rows>x<cols>]
#   row1col1  row1col2  ...
#   row2col1  ...
# Cells are tab-separated; the (LOAD_FAILED) marker indicates a query
# whose Power Query refresh errored.
#
# Background: when a PQ "Load To Table" runs, Excel creates a ListObject
# whose Name becomes a workbook-level Name. Iterating `Names` therefore
# enumerates every loaded query result without us having to hard-code
# cell addresses. Cases that aren't loaded as tables (e.g. a single
# Cube formula result in A2) need to be wrapped with an explicit
# defined Name pointing at the result range.

$path = Join-Path $PSScriptRoot 'Oracle.xlsx'

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false

function Format-CellText {
    param($cell)
    try {
        $t = $cell.Text
        if ($null -eq $t) { return '' }
        return [string]$t
    } catch {
        return '(LOAD_FAILED)'
    }
}

function Dump-Range {
    param($name, $range)
    $rows = $range.Rows.Count
    $cols = $range.Columns.Count

    if ($rows -eq 1 -and $cols -eq 1) {
        $val = Format-CellText $range
        Write-Output "$name=$val"
        return
    }

    Write-Output "$name [${rows}x${cols}]"
    for ($r = 1; $r -le $rows; $r++) {
        $cells = @()
        for ($c = 1; $c -le $cols; $c++) {
            $cells += (Format-CellText ($range.Cells.Item($r, $c)))
        }
        Write-Output ('  ' + ($cells -join "`t"))
    }
}

try {
    $wb = $excel.Workbooks.Open($path)
    $wb.RefreshAll()
    $excel.CalculateUntilAsyncQueriesDone()
    $wb.Save()

    # Names collection contains both workbook-scoped and sheet-scoped
    # names. ListObjects (PQ "Load To Table") generate workbook-scoped
    # names. Iterate and dump each.
    foreach ($name in $wb.Names) {
        try {
            $range = $name.RefersToRange
            Dump-Range $name.Name $range
        } catch {
            Write-Output "$($name.Name)=(NOT_A_RANGE)"
        }
    }

    # Also dump all ListObjects directly — their .Range covers headers
    # plus body, useful when a query was loaded as a Table but no
    # explicit Name was created.
    foreach ($sheet in $wb.Sheets) {
        foreach ($lo in $sheet.ListObjects) {
            Dump-Range ("ListObject:" + $lo.Name) $lo.Range
        }
    }

    $wb.Close($false)
}
finally {
    $excel.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($excel) | Out-Null
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}
