let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    NIINGRED_Table = NISAINT_CS_Database{[Name="NIINGRED",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(NIINGRED_Table,{{"NIEAN", "Inner Barcode/Product Code"}, {"NILANG", "Language"}}),
    #"Removed Columns" = Table.RemoveColumns(#"Renamed Columns",{"NISDESC", "NILDESC", "NIINGREDS", "NIALLERGENS", "NILABSZ", "NINOUPDATE", "NISOURCE", "NIDATE", "NIPREPRINTED", "NIINGREDSUNI", "NIUNICODEUSED", "NILIFESTYLE", "NIADDITIVES", "NICONTROL", "NINUTRITION", "NINUTRITIONHTML", "NINUTRITIONLIN", "NISTORAGETYPE", "NISTORAGE", "NISTORAGEUSAGE", "NIPREPUSAGE", "NICOOKGUIDE", "NICOOKCONCISE", "NIABV", "NIORIGIN", "NIPACKLANGS", "NIDRAINEDWGT", "NIGROSSWGT", "NINETWGT", "NIWGTUNIT", "NISHELFINFO1", "NISHELFINFO2", "NISHELFINFO3", "NISHELFINFO4", "NISHELFINFO5", "NISHELFINFO6", "NIMANUFNADD", "NITAGGEDALLERGENS", "NIPROMOPACK", "NIALLERGENTAG", "NIIGNORETRANSCHK"}),
    #"Renamed Columns1" = Table.RenameColumns(#"Removed Columns",{{"NIBRAND", "Brand"}}),
    #"Filtered Rows" = Table.SelectRows(#"Renamed Columns1", each ([Language] = "ENGLISH") and ([#"Inner Barcode/Product Code"] <> null))
in
    #"Filtered Rows"