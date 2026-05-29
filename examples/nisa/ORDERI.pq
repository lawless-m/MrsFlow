let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    ORDERI_Table = NISAINT_CS_Database{[Name="ORDERI",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(ORDERI_Table,{{"REF", "RIORderNumber"}}),
    #"Removed Columns" = Table.RemoveColumns(#"Renamed Columns",{"EXTRASUBTOTAL"})
in
    #"Removed Columns"