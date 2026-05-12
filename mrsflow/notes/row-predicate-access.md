# Row-by-record-predicate access — RESOLVED

Resolved in `cbfc8c4` (stdlib: Odbc.DataSource + lazy thunks; table{predicate} indexing). `Source{[Item="...",Kind="Sheet"]}[Data]` now works against both Arrow and Rows-backed tables.

Originally captured because `ItemAccess` only handled numeric indices, blocking the Excel and ODBC corpus from running end-to-end despite the stdlib functions being in place.
