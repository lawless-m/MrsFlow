let names = List.Sort(Record.FieldNames(#shared)) in
                Text.Combine(names, "#(lf)")
