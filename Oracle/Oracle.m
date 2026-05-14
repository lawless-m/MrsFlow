let
    Oracle.Serialize = (v as any) as text =>
        if v = null then "null"
        else if v is text then v
        else if v is number then Text.From(v)
        else if v is logical then (if v then "true" else "false")
        else Text.FromBinary(Json.FromValue(v), TextEncoding.Utf8),

    SafeSerialize = (label as text, expr as function) as record =>
        let
            r = try expr()
        in
            if r[HasError]
                then [Q = label, Result = "ERROR: " & r[Error][Message]]
                else [Q = label, Result = Oracle.Serialize(r[Value])],

    Catalog = Table.FromRecords({
        SafeSerialize("q74", () =>  Table.Reverse(#table({"A"}, {{1},{2},{3}}))
)
    })
in
    Catalog
