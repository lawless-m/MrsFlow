// Number subtypes — Int*.Type, Double.Type. Each Type.Is
            // probe answers true (PQ treats these as facets of Number).
            { Type.Is(42, Int8.Type),
              Type.Is(42, Int16.Type),
              Type.Is(42, Int32.Type),
              Type.Is(42, Int64.Type) }
