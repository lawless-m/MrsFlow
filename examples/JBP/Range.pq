let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("Rc49D8IgEIDhv9Iwmi5+69iWNDo4OZIOSC/tRQSDxER+vceR6Pi8XLhTSixFLZrHDcFFMdRKrMh98Akcc03sZrQWRvaGfH2CQW0xfjht84iOENBNHHYcwvhi7Umt9/eiA+miJ53QQSlHKudI3+mysSGfQNs4M9sfqz4AJODa/Stfx1FSlBjAxErSG7dFvhfCG01eN3wB", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Range Number" = _t, #"Range Name" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Range Number", type text}, {"Range Name", type text}})
in
    #"Changed Type"