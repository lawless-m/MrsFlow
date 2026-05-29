let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    COSTS_Table = NISAINT_CS_Database{[Name="COSTS",Kind="Table"]}[Data]
in
    COSTS_Table