Table.AddColumn(#table({"d"}, {{#date(2026,1,1)}}),
    "next", each Date.AddDays([d], 1), type date)
