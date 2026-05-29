(fname as text) => Expression.Evaluate(Text.FromBinary(File.Contents(fname)), #shared)
