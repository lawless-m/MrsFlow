let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    PRICES_Table = NISAINT_CS_Database{[Name="PRICES",Kind="Table"]}[Data]
in
    PRICES_Table