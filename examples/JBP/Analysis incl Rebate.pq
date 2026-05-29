let
    Source = Table.NestedJoin(Analysis, {"CustomerNumber"}, #"Customer Rebate", {"CODE"}, "Customer Rebate", JoinKind.LeftOuter),
    #"Expanded Customer Rebate" = Table.ExpandTableColumn(Source, "Customer Rebate", {"Retro 2019 G", "Retro 2019 S", "Retro 2020 G", "Retro 2020 S", "Retro 2021 G", "Retro 2021 S", "Retro 2022 G", "Retro 2022 S"}, {"Customer Rebate.Retro 2019 G", "Customer Rebate.Retro 2019 S", "Customer Rebate.Retro 2020 G", "Customer Rebate.Retro 2020 S", "Customer Rebate.Retro 2021 G", "Customer Rebate.Retro 2021 S", "Customer Rebate.Retro 2022 G", "Customer Rebate.Retro 2022 S"}),
    #"Inserted Merged Column" = Table.AddColumn(#"Expanded Customer Rebate", "Order&Product Number", each Text.Combine({[RIOrderNumber], [ProductCode]}, ""), type text),
    #"Merged Queries" = Table.NestedJoin(#"Inserted Merged Column", {"Order&Product Number"}, #"PowerBI tab", {"Order&ProductNumber"}, "PowerBI tab", JoinKind.LeftOuter),
    #"Expanded PowerBI tab" = Table.ExpandTableColumn(#"Merged Queries", "PowerBI tab", {"Rebate"}, {"PowerBI tab.Rebate"}),
    #"Renamed Columns" = Table.RenameColumns(#"Expanded PowerBI tab",{{"PowerBI tab.Rebate", "ProductRebate"}})
in
    #"Renamed Columns"