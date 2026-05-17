// ReplaceType stamps a new type on a value (which round-trips
            // through Type.Is on the result).
            Type.Is(Value.Type(Value.ReplaceType(42, type number)), Number.Type)
