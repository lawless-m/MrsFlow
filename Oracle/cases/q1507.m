let
                html = "<table><tr><td>a</td><td>1</td></tr><tr><td>b</td><td>2</td></tr></table>",
                bin = Text.ToBinary(html, TextEncoding.Utf8),
                t = Html.Table(bin,
                    {{"k", "td:nth-child(1)"}, {"v", "td:nth-child(2)"}},
                    [RowSelector="tr"])
            in
                Table.RowCount(t)
