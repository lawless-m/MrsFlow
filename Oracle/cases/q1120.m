// Comparer.FromCulture ignoreCase flag. Real culture-specific collation
// (linguistic sort orders) needs ICU which mrsflow doesn't bundle —
// probe limited to identity + ignoreCase delegation behaviour.
let r = try {
        Comparer.FromCulture("en-US")("a", "a"),
        Comparer.FromCulture("en-US", true)("HELLO", "hello"),
        Comparer.FromCulture("en-US", true)("café", "CAFÉ"),
        Comparer.FromCulture("en-US")("a", "b"),
        Comparer.FromCulture("en-US")("b", "a")
    } in
        if r[HasError]
            then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
            else [HasError=false, Value=r[Value]]
