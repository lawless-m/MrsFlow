Table.Profile(
    #table({"n","s"}, {{1,"a"},{2,"b"},{3,"c"}}),
    {{"Sum", each Type.Is(_, type number), each List.Sum(_)}})
