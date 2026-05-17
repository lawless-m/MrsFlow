// Lines.FromBinary — split bytes into a list of text lines.
            // (\n in M source is literal "\n" text, not LF — both engines
            // see one logical "line".)
            Lines.FromBinary(
                Text.ToBinary("a\nb\nc", TextEncoding.Utf8))
