let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    Analysis_Table = NISAINT_CS_Database{[Name="Analysis",Kind="Table"]}[Data],
    #"Filtered Rows" = Table.SelectRows(Analysis_Table, each [SADATE] >= #date(2016, 1, 1)),
    #"Removed Columns" = Table.RemoveColumns(#"Filtered Rows",{"SASOURCE", "SAPOSTING", "SASEQ", "SAIGNORE", "SAADJUST"}),
    #"Renamed Columns" = Table.RenameColumns(#"Removed Columns",{{"SADATE", "Date"}}),
    #"Removed Columns1" = Table.RemoveColumns(#"Renamed Columns",{"SAINV"}),
    #"Renamed Columns1" = Table.RenameColumns(#"Removed Columns1",{{"SACUST", "Customer Code"}}),
    #"Removed Columns2" = Table.RemoveColumns(#"Renamed Columns1",{"SATERR", "SACLASS", "SAINCOTERMS", "SASIC", "SACREDIT", "SASELLCO"}),
    #"Renamed Columns2" = Table.RenameColumns(#"Removed Columns2",{{"SAPRODUCT", "Product Code"}, {"SAGROUP", "Product Group"}}),
    #"Removed Columns3" = Table.RemoveColumns(#"Renamed Columns2",{"SAEXTRA", "SAHAZCODE", "SASHIPTYPE", "SACOMMOD", "SARECIPE", "SAANALYSISH", "SAANALYSISI", "SAFWDR"}),
    #"Renamed Columns3" = Table.RenameColumns(#"Removed Columns3",{{"SAVAL", "Revenue"}}),
    #"Removed Columns4" = Table.RemoveColumns(#"Renamed Columns3",{"SAVALORG", "SACURR", "SAVALGROSS", "SAVALNET"}),
    #"Renamed Columns4" = Table.RenameColumns(#"Removed Columns4",{{"SACOST", "Cost of Goods"}}),
    #"Removed Columns5" = Table.RemoveColumns(#"Renamed Columns4",{"SACOSTGROSS", "SACOSTNET", "SAQTY"}),
    #"Renamed Columns5" = Table.RenameColumns(#"Removed Columns5",{{"SAORGQTY", "Volume"}}),
    #"Removed Columns6" = Table.RemoveColumns(#"Renamed Columns5",{"SAWGTNET", "SAWGTGROSS", "SAWGTSHIP", "SAVOL", "SAVOLSHIP"}),
    #"Renamed Columns6" = Table.RenameColumns(#"Removed Columns6",{{"SAREF", "Order Reference"}})
in
    #"Renamed Columns6"