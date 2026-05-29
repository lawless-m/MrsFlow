let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    Analysis_Table = NISAINT_CS_Database{[Name="Analysis",Kind="Table"]}[Data],
    #"Filtered Rows" = Table.SelectRows(Analysis_Table, each [SADATE] > #date(2024, 7, 1))
in
    #"Filtered Rows"