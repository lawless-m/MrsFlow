let
                decodeSrc = Text.FromBinary(
                    File.Contents("c:/Users/matthew.heath/Git/MrsFlow/tools/png-decoder/m/Decode.m"),
                    TextEncoding.Utf8),
                renderSrc = Text.FromBinary(
                    File.Contents("c:/Users/matthew.heath/Git/MrsFlow/tools/png-decoder/m/Render.m"),
                    TextEncoding.Utf8),
                PngDecode = Expression.Evaluate(decodeSrc, #shared),
                QuadrantTable = Expression.Evaluate(renderSrc, #shared),
                decoded = PngDecode(File.Contents(
                    "c:/Users/matthew.heath/Git/MrsFlow/tools/png-decoder/png-suite/rough-collie-96x64.png"))
            in
                if decoded[Success]
                    then QuadrantTable(decoded[RGBA8], decoded[Width], decoded[Height])
                    else error decoded[Error]
