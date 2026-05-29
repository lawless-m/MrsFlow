(url) => Expression.Evaluate(Text.FromBinary(Web.Contents(url, [Headers=[Authorization=""], ManualStatusHandling={404}])), #shared)
