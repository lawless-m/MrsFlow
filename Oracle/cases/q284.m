List.Generate(
    () => [i=0, total=0],
    each [i] < 4,
    each [i=[i]+1, total=[total]+[i]+1],
    each [total])
