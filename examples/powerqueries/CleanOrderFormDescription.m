// for some reason, the descriptions were ending in "  . "
// get rid


let
    CleanDescription = (grid) => 
        Table.TransformColumns(grid, {"Description", each 
        let
            trimmed = Text.TrimEnd(_),
            final = if Text.EndsWith(trimmed, ".") then Text.TrimEnd(Text.Start(trimmed, Text.Length(trimmed) - 1)) else trimmed
        in
            final
        })
in CleanDescription