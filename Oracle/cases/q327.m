List.Generate(
    () => [n=1, done=false],
    each not [done],
    each [n=[n]+1, done=([n]+1) >= 5],
    each [n])
