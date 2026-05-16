//! Starter stdlib for eval-6: pure functions bound in the root env.
//!
//! Each function lives in this module as a `BuiltinFn`. `root_env()` builds
//! the initial env containing every binding, used by callers that want a
//! stdlib-aware environment instead of an empty one (`EnvNode::empty()`).
//!
//! Function scope is corpus-driven: the top non-Arrow stdlib calls in the
//! user's actual queries (`Text.Replace`, `Text.Contains`, `List.Transform`,
//! `Number.From`, …). Arrow-backed Table.* and date/datetime/duration land
//! in eval-7+.

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::value::{BuiltinFn, Closure, FnBody, Value};

mod common;
mod number;
mod text;
mod logical;
mod list;
mod record;
pub(crate) mod table;
mod date;
mod time;
mod datetime;
mod datetimezone;
mod duration;
mod parquet;
mod odbc;
mod splitter;
mod combiner;
mod replacer;
mod comparer;
mod uri;
mod lines;
mod type_ops;
pub(super) mod value_ops;
mod binary;
mod function_ops;
mod expression;
mod error_ops;
mod json;
mod file;
mod excel;
mod web;
mod csv;
mod folder;
mod diagnostics;
mod variable;
mod mysql;
mod postgres;
mod sql;
mod xml;
mod html;


/// Build the initial environment containing every stdlib intrinsic plus
/// the two literal constants `#nan` and `#infinity`. Tests and shells pass
/// this as the starting env instead of `EnvNode::empty()`.
pub fn root_env() -> Env {
    let mut env = EnvNode::empty();
    for (name, params, body) in builtin_bindings() {
        let closure = Closure {
            params,
            body: FnBody::Builtin(body),
            env: EnvNode::empty(),
        };
        env = env.extend(name.to_string(), Value::Function(closure));
    }
    env = env.extend("#nan".into(), Value::Number(f64::NAN));
    env = env.extend("#infinity".into(), Value::Number(f64::INFINITY));

    // Type intrinsics (dotted-name values). Power Query exposes these as
    // type values; the corpus uses them in Table.AddColumn type args and
    // Table.TransformColumnTypes pairs. Several numeric intrinsics collapse
    // to TypeRep::Number for v1 (we have only f64 underlying) — the type
    // ascription path still works because TypeRep::Number → DataType::Float64.
    use super::value::TypeRep;
    for (name, tr) in [
        ("Number.Type",   TypeRep::Number),
        ("Int64.Type",    TypeRep::NamedNumeric("Int64.Type")),
        ("Int32.Type",    TypeRep::NamedNumeric("Int32.Type")),
        ("Int16.Type",    TypeRep::NamedNumeric("Int16.Type")),
        ("Int8.Type",     TypeRep::NamedNumeric("Int8.Type")),
        ("Currency.Type", TypeRep::NamedNumeric("Currency.Type")),
        ("Decimal.Type",  TypeRep::NamedNumeric("Decimal.Type")),
        ("Single.Type",   TypeRep::NamedNumeric("Single.Type")),
        ("Double.Type",   TypeRep::NamedNumeric("Double.Type")),
        ("Percentage.Type", TypeRep::NamedNumeric("Percentage.Type")),
        ("Text.Type",     TypeRep::Text),
        ("Logical.Type",  TypeRep::Logical),
        ("Date.Type",     TypeRep::Date),
        ("DateTime.Type", TypeRep::Datetime),
        ("DateTimeZone.Type", TypeRep::Datetimezone),
        ("Time.Type",     TypeRep::Time),
        ("Duration.Type", TypeRep::Duration),
        ("Binary.Type",   TypeRep::Binary),
        ("Null.Type",     TypeRep::Null),
        ("Any.Type",      TypeRep::Any),
    ] {
        env = env.extend(name.to_string(), Value::Type(tr));
    }

    // JoinKind enum constants — numeric per Power Query M spec.
    for (name, n) in [
        ("JoinKind.Inner",      0.0),
        ("JoinKind.LeftOuter",  1.0),
        ("JoinKind.RightOuter", 2.0),
        ("JoinKind.FullOuter",  3.0),
        ("JoinKind.LeftAnti",   4.0),
        ("JoinKind.RightAnti",  5.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // ExtraValues.* constants — Table.FromList extraValues arg. Per M spec:
    // List = 0 (excess goes into the last column as a list), Ignore = 1
    // (excess is dropped), Error = 2 (excess raises an error).
    for (name, n) in [
        ("ExtraValues.List",   0.0),
        ("ExtraValues.Ignore", 1.0),
        ("ExtraValues.Error",  2.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // GroupKind.* constants — Table.Group groupKind arg.
    // Global (default) groups across the whole table; Local only folds
    // consecutive rows with equal keys into the same group.
    for (name, n) in [
        ("GroupKind.Global", 0.0),
        ("GroupKind.Local",  1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // MissingField.* constants — Record.* missingField arg.
    // Error (default) raises on a missing field; Ignore silently
    // skips it; UseNull behaves as if it existed with value null.
    for (name, n) in [
        ("MissingField.Error",   0.0),
        ("MissingField.Ignore",  1.0),
        ("MissingField.UseNull", 2.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Occurrence.* constants — *.PositionOf / *.PositionOfAny occurrence arg.
    // First (default) returns the first match index (or -1); Last returns
    // the last; All returns a list of every match index.
    for (name, n) in [
        ("Occurrence.First", 0.0),
        ("Occurrence.Last",  1.0),
        ("Occurrence.All",   2.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // PercentileMode.* constants — List.Percentile options.PercentileMode.
    // ExcelInc (default): linear interpolation, rank = p*(n-1) — matches
    // Excel PERCENTILE.INC. Other modes are documented in M but not
    // implemented here (rejected at runtime with a clear message).
    for (name, n) in [
        ("PercentileMode.ExcelInc", 0.0),
        ("PercentileMode.ExcelExc", 1.0),
        ("PercentileMode.SqlCont",  2.0),
        ("PercentileMode.SqlDisc",  3.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // RankKind.* constants — Table.AddRankColumn options.RankKind.
    // Competition (default, 1224): ties share rank, gap after.
    // Ordinal     (1234)         : every row unique (orig-index tiebreak).
    // Dense       (1223)         : ties share rank, no gap.
    // Modified    (3344, not implemented yet — rejected at runtime).
    for (name, n) in [
        ("RankKind.Competition", 0.0),
        ("RankKind.Ordinal",     1.0),
        ("RankKind.Dense",       2.0),
        ("RankKind.Modified",    3.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // BinaryEncoding.* constants — Binary.FromText/ToText encoding arg.
    for (name, n) in [
        ("BinaryEncoding.Base64", 0.0),
        ("BinaryEncoding.Hex",    1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // TextEncoding.* constants — Text.FromBinary/ToBinary encoding arg.
    // Values match the Windows code-page numbers PQ uses. Only Utf8
    // (65001) is actually decoded; the others are accepted-as-numbers
    // so source compiles, but Text.ToBinary errors on non-65001 per
    // the strict-encodings memory.
    for (name, n) in [
        ("TextEncoding.Ascii",             20127.0),
        ("TextEncoding.BigEndianUnicode",  1201.0),
        ("TextEncoding.Unicode",           1200.0),
        ("TextEncoding.Utf16",             1200.0),
        ("TextEncoding.Utf8",              65001.0),
        ("TextEncoding.Windows",           1252.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Compression.* constants — Binary.Compress/Decompress compressionType arg.
    for (name, n) in [
        ("Compression.None",    0.0),
        ("Compression.GZip",    1.0),
        ("Compression.Deflate", 2.0),
        ("Compression.Brotli",  3.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Order.* constants — Table.Sort's per-column order argument.
    for (name, n) in [
        ("Order.Ascending",  0.0),
        ("Order.Descending", 1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // RoundingMode.* constants — Number.Round's 3rd argument.
    for (name, n) in [
        ("RoundingMode.AwayFromZero", 0.0),
        ("RoundingMode.Down",         1.0),
        ("RoundingMode.ToEven",       2.0),
        ("RoundingMode.TowardZero",   3.0),
        ("RoundingMode.Up",           4.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Math constants.
    for (name, n) in [
        ("Number.PI",       std::f64::consts::PI),
        ("Number.E",        std::f64::consts::E),
        ("Number.Epsilon",  f64::EPSILON),
        ("Number.PositiveInfinity", f64::INFINITY),
        ("Number.NegativeInfinity", f64::NEG_INFINITY),
        ("Number.NaN",      f64::NAN),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Day.* constants — Date.DayOfWeek's firstDayOfWeek argument.
    for (name, n) in [
        ("Day.Sunday",    0.0),
        ("Day.Monday",    1.0),
        ("Day.Tuesday",   2.0),
        ("Day.Wednesday", 3.0),
        ("Day.Thursday",  4.0),
        ("Day.Friday",    5.0),
        ("Day.Saturday",  6.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // QuoteStyle.* constants — Csv.Document QuoteStyle option.
    for (name, n) in [
        ("QuoteStyle.None", 0.0),
        ("QuoteStyle.Csv",  1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // TraceLevel.* constants — Diagnostics.Trace traceLevel argument.
    for (name, n) in [
        ("TraceLevel.Critical",    1.0),
        ("TraceLevel.Error",       2.0),
        ("TraceLevel.Warning",     3.0),
        ("TraceLevel.Information", 4.0),
        ("TraceLevel.Verbose",     5.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Stub entries for PQ stdlib names mrsflow doesn't implement. They exist
    // so `#shared` field count and Record.FieldCount(#shared) match PQ.
    // Calling them raises NotImplemented.
    for name in PQ_NAMES_STUB {
        if env.lookup(name).is_none() {
            env = env.extend(name.to_string(), Value::Function(Closure {
                params: vec![],
                body: FnBody::Builtin(not_implemented_stub),
                env: EnvNode::empty(),
            }));
        }
    }

    // #shared — global record of all stdlib bindings. PQ uses this in
    // Expression.Evaluate("...", #shared) to expose the standard library to
    // dynamically evaluated M.
    let shared = collect_shared_record(&env);
    env = env.extend("#shared".into(), shared);

    env
}

fn not_implemented_stub(_args: &[Value], _host: &dyn super::iohost::IoHost) -> Result<Value, super::value::MError> {
    Err(super::value::MError::NotImplemented("stub binding (PQ stdlib name without mrsflow impl)"))
}

/// PQ stdlib names mrsflow doesn't implement — registered as stub functions
/// so the size of `#shared` and the set of recognised identifiers in
/// Expression.Evaluate match Power Query's surface area.
const PQ_NAMES_STUB: &[&str] = &[
    // Access*, AdoDotNet*, AnalysisServices, AzureStorage.*, Cube.*
    "AccessControlEntry.ConditionToIdentities", "AccessControlKind.Allow", "AccessControlKind.Deny",
    "Access.Database",
    "AdoDotNet.DataSource", "AdoDotNet.Query",
    "AnalysisServices.Database", "AnalysisServices.Databases",
    "AzureStorage.BlobContents", "AzureStorage.Blobs", "AzureStorage.DataLake",
    "AzureStorage.DataLakeContents", "AzureStorage.Tables",
    "Cube.AddAndExpandDimensionColumn", "Cube.AddMeasureColumn", "Cube.ApplyParameter",
    "Cube.AttributeMemberId", "Cube.AttributeMemberProperty", "Cube.CollapseAndRemoveColumn",
    "Cube.Dimensions", "Cube.DisplayFolders", "Cube.MeasureProperties", "Cube.MeasureProperty",
    "Cube.Measures", "Cube.Parameters", "Cube.Properties", "Cube.PropertyKey",
    "Cube.ReplaceDimensions", "Cube.Transform",
    // DB2.*, Essbase.*, Exchange.*, Excel.* extras
    "DB2.Database",
    "Essbase.Cubes",
    "Excel.ShapeTable", "Excel.Workbook",
    "Exchange.Contents",
    // Facebook.*
    "Facebook.Graph",
    // File.*
    "File.Contents",
    // GoogleAnalytics.*
    "GoogleAnalytics.Accounts",
    // Hdfs.*, HdInsight.*
    "Hdfs.Contents", "Hdfs.Files",
    "HdInsight.Containers", "HdInsight.Contents", "HdInsight.Files",
    // Html.*
    "Html.Table",
    // Identity.*, Informix.*, Itemized.*
    "Identity.From", "Identity.IsMemberOf",
    "Informix.Database",
    // Json.* extras
    "Json.Encoding",
    // LineNumber, Loader, Logical*
    "LineNumber.From",
    "Loader.Module",
    "Logical.IsLogical",
    // M.*
    "MDX.Cube", "MDX.Statement",
    // MySQL.*, MailChimp.*
    "MailChimp.Database",
    "MySQL.Database",
    // OData.*, ODataOmitValues
    "OData.Feed",
    "ODataOmitValues.Nulls",
    // OleDb.*, Oracle.*
    "OleDb.DataSource", "OleDb.Query",
    "Oracle.Database",
    // Pdf.Tables, Postgres extras, Progress, PowerPlatform
    "Pdf.Tables",
    "PowerPlatform.Dataflows",
    "Progress.DataDirect",
    // RData.*, RScript.*
    "RData.FromBinary",
    "RScript.Execute",
    // Salesforce*, SapBusinessWarehouse*, SapHana*, SapErp, Sharepoint*
    "Salesforce.Data", "Salesforce.Reports",
    "SapBusinessWarehouse.Cubes",
    "SapErp.Reports",
    "SapHana.Database",
    "SharePoint.Contents", "SharePoint.Files", "SharePoint.Tables",
    // Snowflake, SoqlDataSource, SqlExpression, Sybase, Teradata, Tdms
    "Snowflake.Databases",
    "Soda.Feed",
    "SqlExpression.SchemaFrom", "SqlExpression.ToExpression",
    "Sybase.Database",
    "Teradata.Database",
    // Web.*
    "Web.BrowserContents", "Web.Contents", "Web.Headers", "Web.Page",
    // Xml.*
    "Xml.Document", "Xml.Tables",
    // BinaryFormat.* (Pq stream format combinators)
    "BinaryFormat.7BitEncodedSignedInteger", "BinaryFormat.7BitEncodedUnsignedInteger",
    "BinaryFormat.Binary", "BinaryFormat.Byte", "BinaryFormat.ByteOrder",
    "BinaryFormat.Choice", "BinaryFormat.Decimal", "BinaryFormat.Double",
    "BinaryFormat.Group", "BinaryFormat.Length", "BinaryFormat.List",
    "BinaryFormat.Null", "BinaryFormat.Record", "BinaryFormat.SignedInteger16",
    "BinaryFormat.SignedInteger32", "BinaryFormat.SignedInteger64",
    "BinaryFormat.Single", "BinaryFormat.Text", "BinaryFormat.Transform",
    "BinaryFormat.UnsignedInteger16", "BinaryFormat.UnsignedInteger32",
    "BinaryFormat.UnsignedInteger64",
    // ByteOrder.*, CertificateAction
    "ByteOrder.BigEndian", "ByteOrder.LittleEndian",
    "CertificateAction.From",
    // Cdm.* (Common Data Model)
    "Cdm.Contents",
    // Currency.*, Decimal.*, Double.*, Single.* extras
    "Currency.Hash", "Currency.Pad",
    "Decimal.AsCurrency", "Decimal.IsDecimal",
    // Function.*, Diagnostics extras
    "Function.From", "Function.IsDataSource", "Function.ScalarVector",
    "Function.IfNull",
    "Diagnostics.ActivityId", "Diagnostics.Database",
    // ItemExpression.*, RowExpression.*
    "ItemExpression.From", "ItemExpression.Item",
    "RowExpression.Column", "RowExpression.From", "RowExpression.Row",
    // Misc
    "ResourceExpression.From", "ResourceExpression.Resource",
    "Section.AccessibleItems", "Variable.Value",
    // Replacer extras
    "Replacer.ReplaceMissingPositionalKey",
    // Currency / Number additions
    "Number.Combinations", "Number.Permutations",
    // Date / DateTime extras
    "DateTime.IsInCurrentDay", "DateTime.IsInCurrentHour", "DateTime.IsInCurrentMinute",
    "DateTime.IsInCurrentMonth", "DateTime.IsInCurrentQuarter", "DateTime.IsInCurrentSecond",
    "DateTime.IsInCurrentWeek", "DateTime.IsInCurrentYear",
    "DateTime.IsInNextDay", "DateTime.IsInNextHour", "DateTime.IsInNextMinute",
    "DateTime.IsInNextMonth", "DateTime.IsInNextNDays", "DateTime.IsInNextNHours",
    "DateTime.IsInNextNMinutes", "DateTime.IsInNextNMonths", "DateTime.IsInNextNQuarters",
    "DateTime.IsInNextNSeconds", "DateTime.IsInNextNWeeks", "DateTime.IsInNextNYears",
    "DateTime.IsInNextQuarter", "DateTime.IsInNextSecond", "DateTime.IsInNextWeek",
    "DateTime.IsInNextYear",
    "DateTime.IsInPreviousDay", "DateTime.IsInPreviousHour", "DateTime.IsInPreviousMinute",
    "DateTime.IsInPreviousMonth", "DateTime.IsInPreviousNDays", "DateTime.IsInPreviousNHours",
    "DateTime.IsInPreviousNMinutes", "DateTime.IsInPreviousNMonths",
    "DateTime.IsInPreviousNQuarters", "DateTime.IsInPreviousNSeconds",
    "DateTime.IsInPreviousNWeeks", "DateTime.IsInPreviousNYears",
    "DateTime.IsInPreviousQuarter", "DateTime.IsInPreviousSecond",
    "DateTime.IsInPreviousWeek", "DateTime.IsInPreviousYear",
    // Date.IsIn* mirrors
    "Date.IsInCurrentDay", "Date.IsInCurrentMonth", "Date.IsInCurrentQuarter",
    "Date.IsInCurrentWeek", "Date.IsInCurrentYear",
    "Date.IsInNextDay", "Date.IsInNextMonth", "Date.IsInNextNDays",
    "Date.IsInNextNMonths", "Date.IsInNextNQuarters", "Date.IsInNextNWeeks",
    "Date.IsInNextNYears", "Date.IsInNextQuarter", "Date.IsInNextWeek",
    "Date.IsInNextYear",
    "Date.IsInPreviousDay", "Date.IsInPreviousMonth", "Date.IsInPreviousNDays",
    "Date.IsInPreviousNMonths", "Date.IsInPreviousNQuarters", "Date.IsInPreviousNWeeks",
    "Date.IsInPreviousNYears", "Date.IsInPreviousQuarter", "Date.IsInPreviousWeek",
    "Date.IsInPreviousYear",
    // Table.* extras
    "Table.FuzzyGroup", "Table.FuzzyJoin", "Table.FuzzyNestedJoin",
    "Table.AddFuzzyClusterColumn", "Table.AddRankColumn",
    "Table.Buffer", "Table.View", "Table.ViewError", "Table.ViewFunction",
    "Table.NavigationTableView", "Table.PromoteHeaders",
    "Table.SelectColumns", "Table.SplitAt", "Table.StopFolding",
    "Table.TransformColumns", "Table.ApproximateRowCount",
    // Action.*
    "Action.Return", "Action.Sequence", "Action.Try", "Action.WithErrorContext",
    // ValueAsAny, Value.Compare extras
    "Value.Alternates", "Value.Cause", "Value.Expression",
    "Value.Firewall", "Value.Lineage", "Value.Metadata",
    "Value.NativeQuery", "Value.NullableEquals", "Value.RemoveMetadata",
    "Value.Traits", "Value.Versions", "Value.VersionIdentity", "Value.ViewError",
    "Value.ViewFunction",
    // Type.* extras
    "Type.AddTableKey", "Type.IsOpenRecord", "Type.NonNullable",
    "Type.ReplaceFacets", "Type.ReplaceTableKeys", "Type.TableKeys", "Type.ForFunction",
    "Type.ForRecord",
    // Embedded.*, Splitter / Combiner extras
    "Embedded.Value",
    "Splitter.SplitByNothing",
    "Combiner.CombineTextByRanges",
    // Additional PQ stdlib names to bring #shared count closer to PQ's 856.
    "Action.Sequence", "AdobeAnalytics.Cubes",
    "AmazonRedshift.Database",
    "ApiKeyAuthKind.Custom", "ApiKeyAuthKind.Header", "ApiKeyAuthKind.Query",
    "AzureCosmosDB.Contents", "AzureDataExplorer.Contents",
    "AzureDataLakeStorage.Contents",
    "AzureEnterpriseApplication.Authorize", "AzureML.Predict",
    "Beehiveid", "BingSearch.Search",
    "Cognite.Contents",
    "CommonDataService.Database",
    "Currency.Round",
    "CustomFunctions.Documentation",
    "DateTime.AddZone", "DateTime.From",
    "DependencyAction.Always", "DependencyAction.Never",
    "Diagnostics.ReportIssue",
    "DocumentDb.Contents", "Dynamics365.Database", "Dynamics365.Webservice",
    "Edx.Cubes",
    "Eq.Equals", "Eq.IsHashable",
    "ExcelWorkbook.Workbook",
    "Folder.Files", "Folder.Contents",
    "FtpFiles.Contents",
    "Hash.Crc32", "Hash.Md5", "Hash.Sha1", "Hash.Sha256",
    "HostedService.Refresh",
    "HostedService.Configure",
    "Iif.From",
    "ImpalaDb.Database",
    "Informix.Tables",
    "Json.SerializeStable",
    "List.Concurrent", "List.Window",
    "Localizable.Documentation",
    "Localizable.From",
    "LocalSettings.Get", "LocalSettings.Set",
    "MachineLearning.Cluster", "MachineLearning.Predict",
    "MarketoLeads.Database",
    "Maximum.Decimal", "Maximum.Number", "Minimum.Decimal", "Minimum.Number",
    "Microsoft.Graph.Auth", "MicrosoftGraph.Tables",
    "OData.Authenticate", "OData.NavigationProperty",
    "PowerBI.Datasets", "PowerPlatformDataflows.Contents",
    "TextAnalytics.AnalyzeSentiment", "TextAnalytics.LanguageDetection",
    "SecureString.From",
    "Service.From",
    "ServiceNow.Database",
    "Soap.From",
];

fn collect_shared_record(env: &Env) -> Value {
    use std::collections::BTreeMap;
    use super::value::Record;
    let mut all: BTreeMap<String, Value> = BTreeMap::new();
    let mut cur: Option<&EnvNode> = Some(env);
    while let Some(node) = cur {
        for (k, v) in &node.bindings {
            all.entry(k.clone()).or_insert_with(|| v.clone());
        }
        cur = node.parent.as_deref();
    }
    Value::Record(Record {
        fields: all.into_iter().collect(),
        env: EnvNode::empty(),
    })
}


fn builtin_bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    let mut all = Vec::new();
    for slice in [
        number::bindings(),
        text::bindings(),
        logical::bindings(),
        list::bindings(),
        record::bindings(),
        table::bindings(),
        date::bindings(),
        time::bindings(),
        datetime::bindings(),
        datetimezone::bindings(),
        duration::bindings(),
        parquet::bindings(),
        odbc::bindings(),
        splitter::bindings(),
        combiner::bindings(),
        replacer::bindings(),
        comparer::bindings(),
        uri::bindings(),
        lines::bindings(),
        type_ops::bindings(),
        value_ops::bindings(),
        binary::bindings(),
        function_ops::bindings(),
        expression::bindings(),
        error_ops::bindings(),
        json::bindings(),
        file::bindings(),
        excel::bindings(),
        web::bindings(),
        csv::bindings(),
        folder::bindings(),
        diagnostics::bindings(),
        variable::bindings(),
        mysql::bindings(),
        postgres::bindings(),
        sql::bindings(),
        xml::bindings(),
        html::bindings(),
    ] {
        all.extend(slice);
    }
    all
}

