$path = Join-Path $PSScriptRoot 'Oracle.xlsx'

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false

try {
    $wb = $excel.Workbooks.Open($path)
    $wb.RefreshAll()
    $excel.CalculateUntilAsyncQueriesDone()
    $wb.Save()

    $ws = $wb.Sheets.Item(1)
    $a2 = $ws.Cells.Item(2, 1).Text
    Write-Output $a2

    $wb.Close($false)
}
finally {
    $excel.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($excel) | Out-Null
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}
