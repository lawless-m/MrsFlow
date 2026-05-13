Table.SelectColumns(
    Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true])
        {[Name="NISAINT_CS",Kind="Database"]}[Data]
        {[Name="RIGeographic",Kind="Table"]}[Data],
    {"RITerritoryCode", "RITerritoryDesc"})
