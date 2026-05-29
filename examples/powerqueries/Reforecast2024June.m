let
    ibms = {"Alex McRae","Bastian Laastad","Carl Poelstra","Ceirwen Williams","Connor Plumtree","Elvis Lopez","Irina Racheva","Jamie Mapley","Joshua Smith","Kelly Leary","Linh Jones","Lucas Bertolini","Lucy Clewlow","Maegan Jenkins","Maggie Andrearczyk","Marjorie Astrion","Olivia Rogers","Philip Borum","Remi Bou Sleiman","Richard Smith","Saad Oueini","Saffron Thomsen","Sally Hou","Samantha Wilding","Sarah Robinson","Scott Taylor","Stephen Heather","Timothy Nalpon","X Ramsden","Yolande Clark"},
	firstrow = 9, numcols = 34,
    finalColumns = {"Company Country", "Company ID", "Account No", "Account Name", "Year", "Margin", "Total Revenue", "Total GP£", "January Revenue", "January GP£", "February Revenue", "February GP£", "March Revenue", "March GP£", "April Revenue", "April GP£", "May Revenue", "May GP£", "June Revenue", "June GP£", "July Revenue", "July GP£", "August Revenue", "August GP£", "September Revenue", "September GP£", "October Revenue", "October GP£", "November Revenue", "November GP£", "December Revenue", "December GP£", "Comments", "Source", "Owner", "Selling Company", "CRM IBM/EAA", "RI Territory", "Continent", "ActualGP£", "ForecastGP£", "ActualRev", "ForecastRev"},

    exceldir = "S:\Budget2024\Reforecast 2024 Sales Team",
       
    ibmSheets = (suffix, sheet, skip, numcols, source) => let
        files = Table.SelectRows(Folder.Files(exceldir), each List.AnyTrue(List.Transform(ibms, (ibm) => Text.EndsWith([Extension], ".xlsx") and Text.StartsWith([Name], ibm) and Text.Contains([Name], suffix)))),
        addData = Table.AddColumn(files, "Data", each Excel.Workbook([Content])),
        expand = Table.ExpandTableColumn(addData, "Data", {"Name", "Data"}, {"SheetName", "SheetData"}),
        rows = Table.SelectRows(expand, each [SheetName] = sheet),
        load = Table.AddColumn(rows, "ProcessedSheetData", each loadFileSheet([SheetData], skip, [Name], "_" & suffix & ".xlsx", numcols))
        in Table.AddColumn(Table.Combine(load[ProcessedSheetData]), "Source", each source),

    loadFileSheet = (sheet, skip, filename, namematch, numcols) => let
        rows = Table.Skip(sheet, skip),
        columns = Table.SelectColumns(rows, List.FirstN(Table.ColumnNames(rows), numcols-1)),
        nonblank = Table.SelectRows(columns, each not List.IsEmpty(List.RemoveMatchingItems(Record.FieldValues(_), {"", null}))),
        namedrows = Table.PromoteHeaders(nonblank, [PromoteAllScalars=true])
        in Table.AddColumn(namedrows, "Filename", each Text.Replace(filename, namematch, ""), type text),

    attachGeography = (rows, regions) => Table.NestedJoin(rows, {"Company Country"}, regions, {"CountryName"}, "Regions", JoinKind.LeftOuter),

    Existing = ibmSheets("Existing Business Reforecast 2024", "Existing", firstrow, numcols, "Existing"),

    Regions = Odbc.Query("dsn=Exportmaster", "SELECT DISTINCT RegionCode, RITerritoryDesc, ContinentName, CountryName FROM rigeographic;"),

    Seller = Table.AddColumn(Odbc.Query("dsn=Exportmaster", "select code, sellco from customer"), "Selling Company", each if [sellco] = 2 then "RI" else "REBV"),

    CRMCompany = let
        crmsql = "SELECT Account.Acc_CompanyId, Company.Comp_Name, Account.Acc_AccountID, Account.acc_n_prospectno, Account.acc_code, rtrim(ltrim(Coalesce(acc_code COLLATE DATABASE_DEFAULT, acc_code COLLATE DATABASE_DEFAULT, Account.acc_n_prospectno COLLATE DATABASE_DEFAULT))) as Account_Key, Account.Acc_Name, Account.acc_n_terrman, Account.acc_n_region, rtrim(Users.User_FirstName)+' '+ltrim(users.user_LastName), company.comp_salesregion FROM Sage1000.dbo.Account Account Left join Sage1000.dbo.Company Company on Account.Acc_CompanyId = Company.Comp_CompanyId Left join Sage1000.dbo.Users Users on Account.acc_n_terrman = Users.User_UserId",
        loadWithTypes = Table.TransformColumnTypes(Odbc.Query("dsn=Sage1000 CRM", crmsql),{{"Acc_CompanyId", type text}, {"Acc_AccountID", type text}, {"acc_n_terrman", type text}}),
        fixCols = Table.RemoveColumns(Table.AddColumn(loadWithTypes, "Custom", each Text.TrimEnd([#""])),{""}),
        renameCols = Table.RenameColumns(fixCols,{{"Custom", "CRM IBM/EAA"}}),
        mergeRegions = Table.NestedJoin(renameCols, {"comp_salesregion"}, Regions, {"RegionCode"}, "Regions", JoinKind.LeftOuter)
        in Table.ExpandTableColumn(mergeRegions, "Regions", {"RITerritoryDesc", "ContinentName"}, {"Regions.RITerritoryDesc", "Regions.ContinentName"}),

    NewBiz = let
        rows = ibmSheets("NBRO Reforecast 2024", "New Business", firstrow, numcols, "New Business"),
        FilteredRows = Table.SelectRows(rows, each ([Year] = "2024 Reforecast") and ([#"Country (Required)"] <> null)),
        AddedAccountNo = Table.AddColumn(FilteredRows, "Account No", each "New Business - " & [Main Driver]),
        RemoveMainDrv = Table.RemoveColumns(AddedAccountNo,{"Main Driver"}),
        AddCompanyID = Table.AddColumn(RemoveMainDrv, "Company ID", each 0),
        AddIBM = Table.AddColumn(AddCompanyID, "CRM IBM/EAA", each [Filename]),
        renameCountry = Table.RenameColumns(AddIBM,{{"Country (Required)", "Company Country"}}),
        SetRegion = Table.NestedJoin(renameCountry, {"Company Country"}, Regions, {"CountryName"}, "RegionTbl", JoinKind.LeftOuter),
        ExpandRegionTbl = Table.ExpandTableColumn(SetRegion, "RegionTbl", {"RITerritoryDesc", "ContinentName"}, {"RegionTbl.RITerritoryDesc","RegionTbl.ContinentName"}),
        DistinctRows = Table.Distinct(ExpandRegionTbl) // Remove duplicate rows
        in Table.RenameColumns(DistinctRows ,{{"Company or Comment", "Account Name"}, {"RegionTbl.RITerritoryDesc","RI Territory"}, {"RegionTbl.ContinentName","Continent"}}),

    Coop = Table.SelectRows(ibmSheets("Coop Reforecast 2024", "Coop", firstrow, numcols, "Co-Op"), each ([Company Country] <> "-")),

    MergeCRM = Table.NestedJoin(Existing, {"Account No"}, CRMCompany, {"Account_Key"}, "CRM Company", JoinKind.LeftOuter),
    ExpandedCRMCompany = Table.ExpandTableColumn(MergeCRM, "CRM Company", {"CRM IBM/EAA", "Regions.RITerritoryDesc", "Regions.ContinentName"}, {"CRM Company.CRM IBM/EAA", "CRM Company.Regions.RITerritoryDesc", "CRM Company.Regions.ContinentName"}),
    MergeSeller = Table.NestedJoin(ExpandedCRMCompany, {"Account No"}, Seller, {"code"}, "Seller", JoinKind.LeftOuter),
    RxSeller = Table.ExpandTableColumn(MergeSeller, "Seller", {"Selling Company"}, {"Seller.Selling Company"}),
    NameSeller = Table.RenameColumns(RxSeller,{{"Seller.Selling Company", "Selling Company"}, {"CRM Company.Regions.RITerritoryDesc", "RI Territory"}, {"CRM Company.CRM IBM/EAA", "CRM IBM/EAA"}}),
    RenameContinent = Table.RenameColumns(NameSeller,{{"CRM Company.Regions.ContinentName", "Continent"}, {"Filename", "Owner"}}),
    AppendNewbiz = Table.Combine({RenameContinent, NewBiz}),
    AppendCoop = Table.Combine({AppendNewbiz, Coop}),
    Budget = AppendCoop,
    
    ActualGP = Table.AddColumn(Budget, "ActualGP£", each List.Sum(List.Transform({[#"January GP£"],[#"February GP£"],[#"March GP£"],[#"April GP£"],[#"May GP£"]}, each try Number.From(_) otherwise 0))),
    ForecastGP = Table.AddColumn(ActualGP, "ForecastGP£", each List.Sum(List.Transform({[#"June GP£"],[#"July GP£"],[#"August GP£"],[#"September GP£"],[#"October GP£"],[#"November GP£"],[#"December GP£"]}, each try Number.From(_) otherwise 0))),
    AddActualRev = Table.AddColumn(ForecastGP, "ActualRev", each List.Sum(List.Transform({[January Revenue], [February Revenue], [March Revenue], [April Revenue], [May Revenue]}, each try Number.From(_) otherwise 0))),
    AddForecastRev = Table.AddColumn(AddActualRev, "ForecastRev", each List.Sum(List.Transform({[June Revenue],[July Revenue],[August Revenue],[September Revenue],[October Revenue],[November Revenue],[December Revenue]}, each try Number.From(_) otherwise 0))),
    AddYearGP = Table.AddColumn(AddForecastRev, "YearGP£", each List.Sum(List.Transform({[#"ActualGP£"],[#"ForecastGP£"]}, each try Number.From(_) otherwise 0))),
    AddYearRev = Table.AddColumn(AddYearGP, "YearRev", each List.Sum(List.Transform({[ActualRev],[ForecastRev]}, each try Number.From(_) otherwise 0))),
    AddYearGPpc = Table.AddColumn(AddYearRev, "YearGP%", each if([YearRev] <> 0) then [#"YearGP£"]/[YearRev] else 0),
    RenameNewTotals = Table.RenameColumns(AddYearGPpc,{{"Margin", "MarginInExcel"}, {"Total Revenue", "TotalRevenueInExcel"}, {"Total GP£", "TotalGP£InExcel"}, {"YearGP%", "Margin"}, {"YearRev", "Total Revenue"}, {"YearGP£", "Total GP£"}}),
    RemoveExcelFigures = Table.RemoveColumns(RenameNewTotals,{"TotalGP£InExcel", "TotalRevenueInExcel", "MarginInExcel"}),
    ReplaceExcelFigures = Table.ReorderColumns(RemoveExcelFigures, finalColumns),
    Reforecast2024 = ReplaceExcelFigures
    
in
    Reforecast2024