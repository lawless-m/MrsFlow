let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    RIGeographic_Table = NISAINT_CS_Database{[Name="RIGeographic",Kind="Table"]}[Data]
in
    RIGeographic_Table