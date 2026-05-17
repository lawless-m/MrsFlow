let
                body = Text.FromBinary(File.Contents("coverage/coverage.m"), TextEncoding.Utf8)
            in
                Expression.Evaluate(body, #shared)
