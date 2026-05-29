let
    Source = Excel.Workbook(File.Contents("S:\Finance\Finance Secure\Commercial Finance\Integrated Reporting\Customer Rebate\Customer Rebate.xlsx"), null, true),
    #"Customer Rebate_Sheet" = Source{[Item="Customer Rebate",Kind="Sheet"]}[Data],
    #"Promoted Headers" = Table.PromoteHeaders(#"Customer Rebate_Sheet", [PromoteAllScalars=true]),
    #"Changed Type" = Table.TransformColumnTypes(#"Promoted Headers",{{"CODE", type text}, {"Retro 2019 G", type number}, {"Retro 2019 S", type number}, {"Retro 2020 G", type number}, {"Retro 2020 S", type number}, {"Retro 2021 G", type number}, {"Retro 2021 S", type number}, {"Retro 2022 G", type number}, {"Retro 2022 S", type number}, {"Column10", type any}})
in
    #"Changed Type"