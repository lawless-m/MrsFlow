Table.SelectColumns(
    Table.SelectRows(
        Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
            {[Name="NISAINT_CS",Kind="Database"]}[Data]
            {[Name="RIGeographic",Kind="Table"]}[Data],
        each [RITerritoryCode] = "GB"),
    {"RITerritoryDesc"})
