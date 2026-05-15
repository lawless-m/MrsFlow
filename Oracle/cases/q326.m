List.Generate(
    () => [a=0, b=1],
    each [a] <= 100,
    each [a=[b], b=[a]+[b]],
    each [a])
