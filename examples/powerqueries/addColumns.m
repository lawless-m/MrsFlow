(tab as table, cols as list) =>
        List.Accumulate(cols, tab, (newtab, col) => Table.AddColumn(newtab, col{0}, each col{1}, col{2}))
        
/*
Example usage

let

    addColumns = (tab as table, cols as list) =>
        List.Accumulate(cols, tab, (newtab, col) => Table.AddColumn(newtab, col{0}, each col{1}, col{2})),

in
    addColumns(#table({"a"}, {{"a"}}), {{"d", 1, type number}, {"e", 2, Int64.Type}, {"f", "t", type text}})

produces a table

abc123 a | 1.2 b | 123 c | abc d
"a" | 1 | 2 | "t"

*/


