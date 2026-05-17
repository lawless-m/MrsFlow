let
                L = Table.FromRecords({[id=1,name="a"],[id=2,name="b"],[id=3,name="c"]}),
                R = Table.FromRecords({[id=1,v=10],[id=1,v=11],[id=3,v=30]}),
                r = try {
                    Table.Join(L, "id", R, "id", JoinKind.LeftSemi),
                    Table.Join(L, "id", R, "id", JoinKind.RightSemi)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
