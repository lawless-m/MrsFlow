let
    Source = Odbc.Query("dsn=Exportmaster", "SELECT [ORDERI].[REF],[ORDERI].[PRODUCT],[ORDERI].[DESC2]   FROM [ORDERI]")
in
    Source