let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    ORDERH_Table = NISAINT_CS_Database{[Name="ORDERH",Kind="Table"]}[Data],
    #"Filtered Rows" = Table.SelectRows(ORDERH_Table, each [OHQTEXPIRYDATE] > #date(2021, 12, 31))
in
    #"Filtered Rows"