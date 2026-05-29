let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    Analysis_Table = NISAINT_CS_Database{[Name="Analysis",Kind="Table"]}[Data],
    #"Removed Columns" = Table.RemoveColumns(Analysis_Table,{"SASOURCE", "SAPOSTING", "SASEQ", "SAIGNORE", "SAADJUST", "SAEXTRA", "SAHAZCODE", "SARECIPE", "SAANALYSISH", "SAANALYSISI", "SAFWDR"}),
    #"Renamed Columns" = Table.RenameColumns(#"Removed Columns",{{"SAREF", "RIOrderNumber"}, {"SADATE", "InvoiceDate"}, {"SAINV", "InvoiceNumber"}, {"SACUST", "CustomerNumber"}, {"SATERR", "TerritoryCode"}, {"SACLASS", "CustomerClass"}, {"SAINCOTERMS", "IncoTerms"}, {"SASELLCO", "SellingCompany"}, {"SAPRODUCT", "ProductCode"}, {"SAGROUP", "ProductGrouping"}, {"SASHIPTYPE", "ShipmentMethod"}, {"SACOMMOD", "CommodityCode"}, {"SAVAL", "Revenue"}, {"SAVALORG", "OriginalRevenue"}, {"SACURR", "Currency"}, {"SAVALGROSS", "GrossRevenue"}}),
    #"Removed Columns1" = Table.RemoveColumns(#"Renamed Columns",{"OriginalRevenue", "GrossRevenue", "SAVALNET", "SACOSTGROSS"}),
    #"Renamed Columns1" = Table.RenameColumns(#"Removed Columns1",{{"SACOST", "Cost"}, {"SAORGQTY", "QTY"}}),
    #"Removed Columns2" = Table.RemoveColumns(#"Renamed Columns1",{"SACOSTNET", "SAQTY", "SAWGTNET", "SAWGTGROSS"}),
    #"Renamed Columns2" = Table.RenameColumns(#"Removed Columns2",{{"SAWGTSHIP", "ShipmentWeight"}, {"SAVOL", "Volume"}, {"SAVOLSHIP", "ShipmentVolume"}}),
    #"Filtered Rows" = Table.SelectRows(#"Renamed Columns2", each [InvoiceDate] > #date(2024, 6, 1))
in
    #"Filtered Rows"