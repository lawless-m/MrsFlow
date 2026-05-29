let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    RIGeographic_Table = NISAINT_CS_Database{[Name="RIGeographic",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(RIGeographic_Table,{{"RITerritoryCode", "Territory Short"}, {"RITerritoryDesc", "Territory"}})
in
    #"Renamed Columns"