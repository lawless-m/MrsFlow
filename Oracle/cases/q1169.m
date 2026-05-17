let
                r = try {
                    Value.Is(JoinAlgorithm.SortMerge, type number),
                    Value.Is(JoinAlgorithm.Dynamic, type number),
                    Value.Is(JoinAlgorithm.PairwiseHash, type number)
                } in
                    if r[HasError]
                        then [HasError=true, Reason=r[Error][Reason], Message=r[Error][Message]]
                        else [HasError=false, Value=r[Value]]
