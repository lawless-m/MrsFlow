let
    Source = Odbc.DataSource("dsn=OCS1", [HierarchicalNavigation=true]),
    Sage1000_Database = Source{[Name="Sage1000",Kind="Database"]}[Data],
    dbo_Schema = Sage1000_Database{[Name="dbo",Kind="Schema"]}[Data],
    Users_Table = dbo_Schema{[Name="Users",Kind="Table"]}[Data]
in
    Users_Table