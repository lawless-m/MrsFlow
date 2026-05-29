let
    Source = Excel.Workbook(File.Contents("\\rivsts01\Share\Finance\Finance Secure\Commercial Finance\Integrated Reporting\Budget\Budget 2022.xlsx"), null, true),
    #"Revenue PowerBI_Sheet" = Source{[Item="Revenue PowerBI",Kind="Sheet"]}[Data],
    #"Promoted Headers" = Table.PromoteHeaders(#"Revenue PowerBI_Sheet", [PromoteAllScalars=true]),
    #"Changed Type" = Table.TransformColumnTypes(#"Promoted Headers",{{"IBM", type text}, {"Date", type date}, {"Value", type number}}),
    #"Renamed Columns" = Table.RenameColumns(#"Changed Type",{{"Value", "Revenue Value"}})
in
    #"Renamed Columns"