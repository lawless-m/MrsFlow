// TransformMany — for each item, generate a list and project
            // pairs. Equivalent to a flat-map / SQL CROSS APPLY.
            List.TransformMany({1, 2, 3}, each {10, 20}, (x, y) => x * y)
