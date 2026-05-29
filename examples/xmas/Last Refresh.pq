let
    Source = DateTime.LocalNow(),
    #"Converted to Table" = #table(1, {{Source}})
in
    #"Converted to Table"