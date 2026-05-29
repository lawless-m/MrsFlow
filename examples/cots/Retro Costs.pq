let
    Source = Excel.Workbook(File.Contents("\\rivsts01\Share\Finance\Finance Secure\Commercial Finance\Jacks Docs\Power BI Dev Work\Retro Costs sept 2022.xlsx"), null, true),
    #"Retro Costs_Sheet" = Source{[Item="Retro Costs",Kind="Sheet"]}[Data],
    #"Promoted Headers" = Table.PromoteHeaders(#"Retro Costs_Sheet", [PromoteAllScalars=true]),
    #"Changed Type" = Table.TransformColumnTypes(#"Promoted Headers",{{"Prod Code", Int64.Type}, {"Description", type text}, {"Retro Receivable", type number}})
in
    #"Changed Type"