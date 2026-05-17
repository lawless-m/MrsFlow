// ForRecord builds a record-type from field name → type record;
            // Type.IsOpenRecord on the result returns a boolean.
            Type.IsOpenRecord(
                Type.ForRecord([a = [Type = type number, Optional = false]], true))
