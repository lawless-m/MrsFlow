let
    Source = Odbc.DataSource("dsn=Exportmaster", [HierarchicalNavigation=true]),
    NISAINT_CS_Database = Source{[Name="NISAINT_CS",Kind="Database"]}[Data],
    RIGeographic_Table = NISAINT_CS_Database{[Name="RIGeographic",Kind="Table"]}[Data],
    #"Renamed Columns" = Table.RenameColumns(RIGeographic_Table,{{"RegionCode", "Region Code"}, {"CountryCode", "Country Code"}, {"RITerritoryCode", "Territory Code"}, {"RITerritoryDesc", "Territory Desc"}, {"Continentname", "Continent Desc"}, {"CountryName", "Country Desc"}, {"RegionDesc", "Region Desc"}}),
    #"Replaced Value" = Table.ReplaceValue(#"Renamed Columns"," All","",Replacer.ReplaceText,{"Region Desc"})
in
    #"Replaced Value"