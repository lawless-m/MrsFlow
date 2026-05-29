let
    StartDate = #date(2015, 1, 1),
    EndDate = Date.EndOfYear(DateTime.Date(DateTime.LocalNow())),
    DayCount = Duration.Days(EndDate - StartDate) + 1,
    Dates = List.Dates(StartDate, DayCount, #duration(1, 0, 0, 0)),
    Table = Table.FromList(Dates, Splitter.SplitByNothing(), {"Date"}, null, ExtraValues.Error),
    Typed = Table.TransformColumnTypes(Table, {{"Date", type date}}),
    AddYear = Table.AddColumn(Typed, "Year", each Date.Year([Date]), Int64.Type),
    AddMonthNum = Table.AddColumn(AddYear, "Month Number", each Date.Month([Date]), Int64.Type),
    AddMonthName = Table.AddColumn(AddMonthNum, "Month Name", each Date.MonthName([Date]), type text),
    AddMonthSort = Table.AddColumn(AddMonthName, "Month Name For Sorting", each Text.From(Date.Year([Date])) & "-" & Text.PadStart(Text.From(Date.Month([Date])), 2, "0"), type text),
    AddDayNum = Table.AddColumn(AddMonthSort, "Day Number", each Date.Day([Date]), Int64.Type),
    AddDayName = Table.AddColumn(AddDayNum, "Day Name", each Date.DayOfWeekName([Date]), type text),
    AddQuarter = Table.AddColumn(AddDayName, "Quarter", each "Q" & Text.From(Date.QuarterOfYear([Date])), type text),
    AddWeekNum = Table.AddColumn(AddQuarter, "Week Number", each "W" & Text.PadStart(Text.From(Date.WeekOfYear([Date])), 2, "0"), type text)
in
    AddWeekNum