Record.TransformFields([a=1], {"missing", each if _ = null then 99 else _},
    MissingField.UseNull)
