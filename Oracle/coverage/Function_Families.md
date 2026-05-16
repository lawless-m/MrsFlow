# PQ stdlib families — what mrsflow does

One row per top-level family in PQ's `#shared`. Status labels:

- **Tested** — full implementation, Oracle has at least one passing case.
- **Implemented** — every name in the family has a binding, no Oracle case yet.
- **Partial** — mrsflow has some names in the family, not all.
- **Untouched** — none of the family is implemented (typically a connector we've scoped out).

Counts come from `COVERAGE.md`; refresh both via `render.ps1`.

| Family | Status | mrsflow / PQ | Description |
|---|---|---:|---|
| (top-level) | Untouched | 0 / 2 | Bare names without a `.Family` prefix (e.g. EvalFile, Beehiveid). Mostly engine internals. |
| Access | Untouched | 0 / 1 | Microsoft Access database connector. |
| AccessControlEntry | Untouched | 0 / 3 | ACL types used by the firewall / trust model. |
| AccessControlKind | Untouched | 0 / 3 | ACL allow / deny / type constants. |
| Action | Implemented | 1 / 1 | Side-effectful action runner (Action.Try / WithErrorContext). Niche. |
| ActiveDirectory | Untouched | 0 / 1 | Active Directory domain enumeration. |
| AdobeAnalytics | Untouched | 0 / 1 | Adobe Analytics OLAP cubes connector. |
| AdoDotNet | Untouched | 0 / 2 | ADO.NET generic database connector. |
| AnalysisServices | Untouched | 0 / 2 | SQL Server Analysis Services / Tabular OLAP connector. |
| Any | Implemented | 1 / 1 | Type-record companion for the `any` type. |
| AzureDataExplorer | Untouched | 0 / 3 | Azure Data Explorer / Kusto connector entry. |
| AzureStorage | Untouched | 0 / 5 | Azure Blob / Table / Data Lake storage connector. |
| Binary | Tested | 18 / 18 | Binary value operations (length, slice, encode, base64, compression). |
| BinaryEncoding | Implemented | 3 / 3 | Base64 / Hex encoding constants for Binary.FromText / ToText. |
| BinaryFormat | Partial | 10 / 22 | Declarative parser-combinator framework for binary streams ("Wireshark in M" — describe a wire format, get a typed parser). |
| BinaryOccurrence | Implemented | 4 / 4 | Occurrence constants (optional / repeating / required) used by BinaryFormat. |
| BufferMode | Implemented | 3 / 3 | Buffer eagerness mode (Eager / Delayed) for *.Buffer. |
| Byte | Implemented | 2 / 2 | Byte-typed numeric (0–255) conversion + type companion. |
| ByteOrder | Implemented | 3 / 3 | Big / little endian constants used by BinaryFormat. |
| Certificate | Untouched | 0 / 1 | Certificate type-record companion. |
| Character | Tested | 3 / 3 | Character-codepoint conversion (FromNumber / ToNumber). |
| Combiner | Tested | 5 / 5 | Text-combine combinators (delimiter, lengths, positions). Symmetric with Splitter. |
| CommonDataService | Untouched | 0 / 1 | Microsoft Dataverse / Common Data Service connector. |
| Comparer | Tested | 4 / 4 | Sort / equality comparer constants and factories (Ordinal, OrdinalIgnoreCase, FromCulture). |
| Compression | Implemented | 8 / 8 | Compression algorithm constants for Binary.Compress / Decompress (GZip, Deflate, etc.). |
| Csv | Tested | 1 / 1 | CSV document parser (Csv.Document). |
| CsvStyle | Implemented | 3 / 3 | Csv.Document quote-style options. |
| Cube | Untouched | 0 / 16 | OLAP cube operations (Analysis Services / SAP BW / Essbase). Niche enterprise OLAP. |
| Culture | Untouched | 0 / 1 | Current-thread culture probe. |
| Currency | Tested | 2 / 2 | Currency-typed decimal value support. |
| DataLake | Untouched | 0 / 2 | Azure Data Lake file enumeration. |
| Date | Tested | 58 / 58 | Date value operations: arithmetic, parts, IsIn*, FromText, formatting. |
| DateTime | Tested | 26 / 26 | DateTime value operations (date + time without zone). |
| DateTimeZone | Tested | 16 / 16 | DateTimeZone value operations (date + time + offset). Includes culture-aware ToText / FromText. |
| Day | Implemented | 8 / 8 | Day-of-week enum constants (Monday=0 .. Sunday=6). |
| DB2 | Untouched | 0 / 1 | IBM DB2 connector. |
| Decimal | Tested | 2 / 2 | Decimal-typed numeric (high-precision) operations. |
| Diagnostics | Implemented | 3 / 3 | Tracing primitives (Diagnostics.Trace, ActivityId). |
| DirectQueryCapabilities | Untouched | 0 / 1 | Connector folding-capability advertisement. |
| Double | Implemented | 2 / 2 | Double-precision float type companion. |
| Duration | Tested | 13 / 13 | Duration (timespan) operations and arithmetic. |
| Embedded | Untouched | 0 / 1 | Embedded-value support. |
| Error | Tested | 1 / 1 | Error-record introspection. |
| Excel | Partial | 3 / 4 | Excel workbook reader (Excel.Workbook, Excel.CurrentWorkbook, Excel.ShapeTable). |
| Exchange | Untouched | 0 / 1 | Microsoft Exchange Server connector. |
| Expression | Tested | 3 / 3 | Expression evaluation primitives (Expression.Evaluate — run M-as-text against an env). |
| ExtraValues | Implemented | 4 / 4 | Csv ragged-row handling enum. |
| Fabric | Untouched | 0 / 1 | Microsoft Fabric (workspace) connector. |
| File | Implemented | 2 / 1 | File reader (File.Contents, File.Modified). |
| Folder | Tested | 2 / 2 | Folder enumeration (Folder.Contents, Folder.Files). |
| Function | Partial | 6 / 7 | Function-value introspection and invocation primitives. |
| Geography | Untouched | 0 / 2 | Geography (lat/lon) WKT conversion. |
| GeographyPoint | Untouched | 0 / 1 | Geography point constructor. |
| Geometry | Untouched | 0 / 2 | Geometry (planar) WKT conversion. |
| GeometryPoint | Untouched | 0 / 1 | Geometry point constructor. |
| Graph | Untouched | 0 / 1 | Microsoft Graph entity-graph navigation. |
| GroupKind | Tested | 3 / 3 | Table.Group kind (Global / Local). |
| Guid | Partial | 1 / 2 | GUID type companion. |
| Hdfs | Untouched | 0 / 2 | HDFS file system connector. |
| HdInsight | Untouched | 0 / 3 | Azure HDInsight Hadoop connector. |
| Html | Implemented | 1 / 1 | HTML table scraper. |
| Identity | Untouched | 0 / 3 | User-identity record. |
| IdentityProvider | Untouched | 0 / 2 | Identity-provider constants for auth flows. |
| Informix | Untouched | 0 / 1 | IBM Informix connector. |
| Int16 | Implemented | 2 / 2 | 16-bit signed integer numeric type. |
| Int32 | Implemented | 2 / 2 | 32-bit signed integer numeric type. |
| Int64 | Tested | 2 / 2 | 64-bit signed integer numeric type. |
| Int8 | Implemented | 2 / 2 | 8-bit signed integer numeric type. |
| ItemExpression | Untouched | 0 / 2 | Query-folding helper for per-item expressions inside List.* projections. |
| JoinAlgorithm | Implemented | 8 / 8 | Table.NestedJoin algorithm hint (Hash / SortMerge / etc.). |
| JoinKind | Tested | 9 / 9 | Table.Join / NestedJoin kind (Inner / LeftOuter / FullOuter / etc.). |
| JoinSide | Implemented | 3 / 3 | Join-side enum for asymmetric joins. |
| Json | Tested | 2 / 2 | JSON parser / serialiser (Json.Document, Json.FromValue). |
| Kusto | Untouched | 0 / 2 | Azure Data Explorer (KQL) connector. |
| Lakehouse | Untouched | 0 / 1 | Microsoft Fabric Lakehouse connector. |
| LimitClauseKind | Implemented | 6 / 6 | Folded-SQL LIMIT/TOP/OFFSET dialect enum. |
| Lines | Tested | 4 / 4 | Line-oriented text helpers (FromText / ToText / FromBinary / ToBinary). |
| List | Partial | 71 / 72 | List operations — the broadest family. Filter, map, reduce, sort, generate, statistical. |
| Logical | Tested | 4 / 4 | Boolean conversion + type companion. |
| MissingField | Partial | 3 / 4 | Record.SelectFields missingField option (Error / Ignore / UseNull). |
| Module | Untouched | 0 / 1 | Module-versions introspection. |
| MySQL | Implemented | 2 / 1 | MySQL database connector. mrsflow exposes both `MySQL.Database` and an extension `MySQL.Query` for raw SQL. |
| None | Untouched | 0 / 1 | None type companion (uninhabited). |
| Null | Implemented | 1 / 1 | Null type companion. |
| Number | Tested | 49 / 49 | Number value operations: arithmetic, rounding, formatting, parsing, bitwise. |
| Occurrence | Partial | 3 / 7 | Text/List PositionOf occurrence enum (First / Last / All) + BinaryFormat reuse. |
| OData | Untouched | 0 / 1 | OData v3/v4 feed connector. |
| ODataOmitValues | Untouched | 0 / 2 | OData null-handling enum. |
| Odbc | Tested | 3 / 3 | Generic ODBC connector. mrsflow has a real implementation with lazy/folded queries. |
| Office | Untouched | 0 / 1 | Office shape inference (Excel chart properties). |
| OleDb | Untouched | 0 / 2 | OLE DB connector (legacy Windows DB connectivity). |
| Oracle | Untouched | 0 / 1 | Oracle Database connector. |
| Order | Partial | 2 / 3 | Sort order enum (Ascending / Descending). |
| Parquet | Implemented | 1 / 0 | Parquet file reader. mrsflow extension; PQ exposes `Parquet.Document` natively. |
| Password | Untouched | 0 / 1 | Password-credential type companion. |
| Pdf | Untouched | 0 / 1 | PDF table extractor. |
| Percentage | Tested | 2 / 2 | Percentage-typed decimal. |
| PercentileMode | Partial | 4 / 5 | List.Percentile mode (ExcelInc / ExcelExc / SqlCont / SqlDisc). |
| PostgreSQL | Implemented | 2 / 1 | PostgreSQL database connector. mrsflow extension `PostgreSQL.Query` for raw SQL. |
| PowerPlatform | Untouched | 0 / 1 | Power Platform dataflows connector. |
| Precision | Implemented | 3 / 3 | Decimal / Double precision enum for numeric typing. |
| Progress | Untouched | 0 / 1 | Progress / DataDirect connector. |
| QuoteStyle | Tested | 3 / 3 | Csv.Document quote-style enum (Csv / None). |
| RankKind | Partial | 3 / 4 | Table.AddRankColumn tie-handling (Competition / Ordinal / Dense). |
| RData | Untouched | 0 / 1 | R serialisation format reader. |
| Record | Partial | 17 / 18 | Record operations: field access, transform, combine, FromList / FromTable round-trips. |
| RelativePosition | Untouched | 0 / 3 | Text.Range / .Middle relative-position enum (FromStart / FromEnd). |
| Replacer | Tested | 2 / 2 | Replacer.ReplaceValue / .ReplaceText — passed as the replacer arg to Table.ReplaceValue. |
| Resource | Untouched | 0 / 1 | Connector resource-access plumbing. |
| RoundingMode | Implemented | 6 / 6 | Number.Round mode (ToEven / Up / Down / AwayFromZero / TowardZero). |
| RowExpression | Untouched | 0 / 3 | Query-folding helper for per-row expressions. |
| Salesforce | Untouched | 0 / 2 | Salesforce Data / Reports connector. |
| SapBusinessWarehouse | Untouched | 0 / 1 | SAP BW cube connector. |
| SapBusinessWarehouseExecutionMode | Untouched | 0 / 4 | SAP BW execution-mode enum. |
| SapHana | Untouched | 0 / 1 | SAP HANA database connector. |
| SapHanaDistribution | Untouched | 0 / 5 | SAP HANA query-distribution enum. |
| SapHanaRangeOperator | Untouched | 0 / 7 | SAP HANA range-filter operator enum. |
| SharePoint | Untouched | 0 / 3 | SharePoint Online connector. |
| Single | Implemented | 2 / 2 | Single-precision float type companion. |
| Soda | Untouched | 0 / 1 | Socrata Open Data API connector. |
| Splitter | Tested | 10 / 10 | Text-split combinators (by delimiter, lengths, character transition). Symmetric with Combiner. |
| Sql | Implemented | 2 / 2 | SQL Server / TDS connector. mrsflow has a native TDS implementation via tiberius. |
| SqlExpression | Untouched | 0 / 2 | SQL query-folding helper (SchemaFrom / ToExpression). Used by custom connector authors, not end users. |
| Sybase | Untouched | 0 / 1 | Sybase database connector. |
| Table | Partial | 113 / 114 | Table operations — the largest family by function count. Filter, group, join, pivot, transform, expand. |
| Tables | Untouched | 0 / 1 | Cross-connector "list tables" facade. |
| Teradata | Untouched | 0 / 1 | Teradata database connector. |
| Text | Tested | 42 / 42 | Text (string) operations: split, replace, format, length, encoding-conversion. UTF-16 code-unit semantics matching .NET. |
| TextEncoding | Partial | 6 / 7 | Encoding constants for Text.FromBinary / ToBinary (UTF-8 the only one decoded). |
| Time | Tested | 10 / 10 | Time-of-day operations. |
| TimeZone | Implemented | 1 / 1 | TimeZone.Current — host timezone probe. |
| TraceLevel | Partial | 5 / 6 | Diagnostics.Trace level enum. |
| Type | Partial | 24 / 25 | Type-value construction and introspection (Type.Is, RecordFields, TableSchema, etc.). |
| Uri | Implemented | 5 / 5 | URI parsing / building. |
| Value | Partial | 26 / 27 | Generic value introspection (Compare, Equals, Is, NativeQuery, Metadata). |
| Variable | Implemented | 2 / 2 | Variable.Value / .ValueOrDefault — env probe. |
| Web | Implemented | 4 / 4 | HTTP-fetching connector (Web.Contents, Web.Headers, Web.Page). |
| WebAction | Implemented | 1 / 1 | Web.Contents action-request constants. |
| WebMethod | Implemented | 7 / 7 | HTTP verb constants (Get / Post / Put / Delete / Patch / Head). |
| Xml | Implemented | 2 / 2 | XML document / table parser. |
