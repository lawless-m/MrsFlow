let
    Source = Excel.Workbook(File.Contents("\\rivsts01\Share\Finance\Finance Secure\Commercial Finance\Integrated Reporting\Budget\Budget 2022.xlsx"), null, true),
    #"GP PowerBI_Sheet" = Source{[Item="GP PowerBI",Kind="Sheet"]}[Data],
    #"Promoted Headers" = Table.PromoteHeaders(#"GP PowerBI_Sheet", [PromoteAllScalars=true]),
    #"Changed Type" = Table.TransformColumnTypes(#"Promoted Headers",{{"IBM", type text}, {"Date", type date}, {"Value", type number}})
in
    #"Changed Type"