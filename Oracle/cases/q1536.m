let
                src = Text.ToBinary("<r><a>1</a><b>2</b></r>", TextEncoding.Utf8),
                t = Xml.Document(src)
            in
                Table.RowCount(t)
