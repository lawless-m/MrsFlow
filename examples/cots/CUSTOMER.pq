let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    CUSTOMER_Table = NISAINT_CS_Database{[Name="CUSTOMER",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(CUSTOMER_Table,{{"CPYNAME", "CustomerName"}})
in
    #"Renamed Columns"