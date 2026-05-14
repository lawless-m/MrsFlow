Table.Distinct(
    #table({"k","v","w"},
        {{"a",1,"x"},{"a",1,"y"},{"a",2,"z"},{"b",1,"w"}}),
    {"k","v"})
