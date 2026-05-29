let
    Source = Excel.Workbook(File.Contents("S:\Finance\Finance Secure\Commercial Finance\Integrated Reporting\Retro Receivable\Terrys 2022.xlsx"), null, true),
    #"PowerBI tab _Sheet" = Source{[Item="PowerBI tab ",Kind="Sheet"]}[Data],
    #"Promoted Headers" = Table.PromoteHeaders(#"PowerBI tab _Sheet", [PromoteAllScalars=true]),
    #"Changed Type" = Table.TransformColumnTypes(#"Promoted Headers",{{"Order&ProductNumber", type text}, {"REF", Int64.Type}, {"SAPRODUCT", Int64.Type}, {"Rebate", type number}})
in
    #"Changed Type"