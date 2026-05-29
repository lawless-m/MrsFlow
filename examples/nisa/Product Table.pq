let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    PRODUCT_Table = NISAINT_CS_Database{[Name="PRODUCT",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(PRODUCT_Table,{{"CODE", "ProductCode"}, {"COMMOD", "CommodityCode"}, {"GROUP", "ProductGrouping"}, {"DESC1", "ProductDescription"}, {"DESC3", "Brand"}}),
    #"Renamed Columns1" = Table.RenameColumns(#"Renamed Columns",{{"DESC5", "ProductWeight"}}),
    #"Removed Columns1" = Table.RemoveColumns(#"Renamed Columns1",{"LONGDESC"}),
    #"Renamed Columns2" = Table.RenameColumns(#"Removed Columns1",{{"CUNIT", "UnitsperCase"}}),
    #"Removed Columns2" = Table.RemoveColumns(#"Renamed Columns2",{"PRICE", "PUNIT", "PRICEPER", "QUNIT", "PACKCONV", "CUNITFAC", "STDCOST", "STDCOSTPER", "PACKULOAD", "PACKCOST", "PACKCOSTPER", "MASSNET", "MASSPACKED", "MASSGROSS", "MASSPER"}),
    #"Renamed Columns3" = Table.RenameColumns(#"Removed Columns2",{{"PRUSER01", "Discontinued?"}, {"PRUSER02", "BannedCode?"}}),
    #"Removed Columns3" = Table.RemoveColumns(#"Renamed Columns3",{"PRSERVICE", "VOL", "VOLPACKED", "VOLPER", "VATRATE", "EXTRA", "CONVFACT", "SUPPUNITS", "SSM", "RECIPE", "ORIGIN", "HAZIND", "HAZCODE", "ANALYSISCODE", "PERTEXT", "PRNOTES"}),
    #"Renamed Columns4" = Table.RenameColumns(#"Removed Columns3",{{"PRUSER03", "Exclude from Price List (Website)?"}}),
    #"Removed Columns4" = Table.RemoveColumns(#"Renamed Columns4",{"PRALERTTEXT", "PRUSER04", "PRUSER05", "PRUSER06", "PRUSER07", "PRUSER08", "PRUSER09", "PRUSER10", "PRALERT", "PRSUPPPRICEPER", "PRSUPP", "PRSUPPPART", "PRSUPPPRICE", "PRSUPPCURR", "PRSUPPDATE", "PRSUPPOFFREF", "PRSUPPCOMMENT", "PRNOTCOUNTED", "PRNEEDQC", "PRNEEDBATCH", "PRFROMSTOCK", "PRNEEDMANDATE", "PRNEEDEXPDATE", "PRNOTFORSALE", "PRREPLACED", "PRREPLACEMENT", "PRFREESTOCKNO"}),
    #"Renamed Columns5" = Table.RenameColumns(#"Removed Columns4",{{"UF_IBARCODE", "InnerBarcode"}, {"UF_UKLDESC", "ProductLongDescription"}, {"UF_O_BARCODE", "OuterBarcode"}, {"UF_PSFLAG", "Presell Flag"}})
in
    #"Renamed Columns5"