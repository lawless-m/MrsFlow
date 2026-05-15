let r = try Table.CombineColumns(
        #table({"first", "second", "third"}, {{"a", "b", "c"}, {"d", "e", "f"}}),
        {"first", "second", "third"},
        Combiner.CombineTextByDelimiter("-"),
        "joined"
    ) in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
