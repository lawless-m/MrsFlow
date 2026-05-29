let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    PRODGRP_Table = NISAINT_CS_Database{[Name="PRODGRP",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(PRODGRP_Table,{{"Code", "Group Code"}, {"Desc", "Category Description"}}),
    #"Capitalized Each Word" = Table.TransformColumns(#"Renamed Columns",{{"Category Description", Text.Proper}})
in
    #"Capitalized Each Word"