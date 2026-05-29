let
    Commodities = Excel.Workbook(Web.Contents("https://ramsdenint.sharepoint.com/sites/ProductData2/Shared%20Documents/Data%20Tasks/Drinks-Commodities.xlsx"), null, true),
    CommodityCodes = Table.SelectColumns(Commodities{[Item="Commodities",Kind="Table"]}[Data],{"Code"}),
    CodeList = (list as list) as text =>
        let
            textList = List.Transform(list, each Text.From(_)),
            quotedList = List.Transform(textList, each "'" & Text.Replace(_, "'", "''") & "'")
        in
            Text.Combine(quotedList, ","),
    // Source data from SQL query (shipment.sql)
    sql = "
        select 
            oh.ref, p.code, p.uf_ibarcode, p.desc1 as product, ni.ninutritionlin, p.group, p.commod,
            cast(coalesce(oi.quantity, 0) as int) * cast(coalesce(oi.cunit, '0') as int) as units,
            case when p.commod in (" & CodeList(Table.Column(CommodityCodes, "Code")) & ") then 1 else 0 end as is_drink
        from
            ORDERH oh
            left join ORDERI oi on oh.ref = oi.ref
            left join PRODUCT p on oi.product = p.code 
            left join NIINGRED ni on ni.niean = p.uf_ibarcode
        where 
            uf_shipmentref = '" & ShipRef & "'
            AND ni.nilang = 'ENGLISH'
            AND oi.quantity > 0
    ",

    Source = Odbc.Query("DSN=Exportmaster", sql),

    // Function to normalize nutrition text
    NormalizeNutritionText = (nutritionText as text) as text =>
        let
            // Convert to lowercase
            step1 = Text.Lower(nutritionText),
            
            // Replace "trace" with "0g"
            step2 = Text.Replace(step1, "trace", "0g"),
            
            // Handle "<X" patterns and fix decimal points without leading zero
            step3 = Text.Replace(Text.Replace(Text.Replace(step2, "<0.5", "0"), "<1", "0"), "<", "0"),
            step3a = Text.Replace(Text.Replace(Text.Replace(step3, " .", " 0."), ",.", ",0."), "(.", "(0."),
            
            // Add "g" suffix where missing - only for nutrition values that should have g
            step4 = let
                // Split by commas and process each part
                parts = Text.Split(step3a, ","),
                processedParts = List.Transform(parts, each
                    let
                        trimmed = Text.Trim(_),
                        // Only add "g" to specific nutrition components that should have grams
                        shouldHaveG = Text.Contains(trimmed, "fat") or 
                                     Text.Contains(trimmed, "saturates") or 
                                     Text.Contains(trimmed, "carbohydrate") or 
                                     Text.Contains(trimmed, "sugars") or 
                                     Text.Contains(trimmed, "protein") or 
                                     Text.Contains(trimmed, "salt"),
                        lastChar = Text.End(trimmed, 1),
                        needsG = shouldHaveG and Text.Contains("0123456789", lastChar) and not Text.EndsWith(trimmed, "g")
                    in
                        if needsG then trimmed & "g" else trimmed
                )
            in
                Text.Combine(processedParts, ","),
            
            // Handle different formats: "Saturates" and "of which saturates"
            step5 = if Text.Contains(step4, "of which saturates (g)") then step4 
                   else if Text.Contains(step4, "of which saturates ") then Text.Replace(step4, "of which saturates ", "of which saturates (g) ")
                   else Text.Replace(step4, "saturates ", "of which saturates (g) "),
            step6 = if Text.Contains(step5, "of which sugars (g)") then step5 
                   else if Text.Contains(step5, "of which sugars ") then Text.Replace(step5, "of which sugars ", "of which sugars (g) ")
                   else Text.Replace(step5, "sugars ", "of which sugars (g) ")
        in
            step6,

    // Function to extract saturated fat value
    ExtractSaturatedFat = (normalizedText as text) as number =>
        let
            // Look for "of which saturates (g) " followed by a number
            searchText = "of which saturates (g) ",
            pos = Text.PositionOf(normalizedText, searchText),
            value = if pos = -1 then 
                // If saturates not found, check if total fat is 0
                let
                    fatSearchText = "fat (g) ",
                    fatPos = Text.PositionOf(normalizedText, fatSearchText),
                    fatValue = if fatPos = -1 then null else
                        let
                            afterFat = Text.End(normalizedText, Text.Length(normalizedText) - fatPos - Text.Length(fatSearchText)),
                            fatCommaPos = Text.PositionOf(afterFat, ","),
                            fatNumberPart = if fatCommaPos = -1 then afterFat else Text.Start(afterFat, fatCommaPos),
                            cleanFatNumber = Text.Trim(Text.Replace(Text.Replace(fatNumberPart, "g", ""), " ", "")),
                            fatResult = if cleanFatNumber = "" then null else 
                                try Number.FromText(cleanFatNumber) otherwise null
                        in
                            fatResult
                in
                    if fatValue = 0 then 0 else null
            else
                let
                    afterSaturates = Text.End(normalizedText, Text.Length(normalizedText) - pos - Text.Length(searchText)),
                    // Find the next comma or end of string
                    commaPos = Text.PositionOf(afterSaturates, ","),
                    numberPart = if commaPos = -1 then afterSaturates else Text.Start(afterSaturates, commaPos),
                    // Clean up the number part - remove any non-numeric characters except decimal point
                    cleanNumber = Text.Trim(Text.Replace(Text.Replace(numberPart, "g", ""), " ", "")),
                    result = if cleanNumber = "" then null else 
                        try Number.FromText(cleanNumber) otherwise null
                in
                    result
        in
            value,

    // Function to extract sugar value  
    ExtractSugar = (normalizedText as text) as number =>
        let
            // Look for "of which sugars (g) " followed by a number
            searchText = "of which sugars (g) ",
            pos = Text.PositionOf(normalizedText, searchText),
            value = if pos = -1 then null else
                let
                    afterSugars = Text.End(normalizedText, Text.Length(normalizedText) - pos - Text.Length(searchText)),
                    // Find the next comma or end of string
                    commaPos = Text.PositionOf(afterSugars, ","),
                    numberPart = if commaPos = -1 then afterSugars else Text.Start(afterSugars, commaPos),
                    // Clean up the number part - remove any non-numeric characters except decimal point
                    cleanNumber = Text.Trim(Text.Replace(Text.Replace(numberPart, "g", ""), " ", "")),
                    // If empty but carbohydrates exist, treat as 0
                    hasCarbs = Text.Contains(normalizedText, "carbohydrate (g)"),
                    result = if cleanNumber = "" then 
                                (if hasCarbs then 0 else null)
                             else 
                                try Number.FromText(cleanNumber) otherwise null
                in
                    result
        in
            value,

    // Function to determine grade for a single parameter
    GetParameterGrade = (value as nullable number, thresholds as list) as nullable number =>
        if value = null then null
        else if value <= thresholds{0} then 1      // Grade A
        else if value <= thresholds{1} then 2 // Grade B  
        else if value <= thresholds{2} then 3 // Grade C
        else 4,                               // Grade D

    // Function to classify nutrition grade
    ClassifyNutrition = (sugarValue as nullable number, satFatValue as nullable number) as record =>
        let
            sugarThresholds = {1, 5, 10},     // A: ≤1, B: ≤5, C: ≤10, D: >10
            satFatThresholds = {0.7, 1.2, 2.8}, // A: ≤0.7, B: ≤1.2, C: ≤2.8, D: >2.8
            
            sugarGrade = if sugarValue = null then null else GetParameterGrade(sugarValue, sugarThresholds),
            satFatGrade = if satFatValue = null then null else GetParameterGrade(satFatValue, satFatThresholds),
            
            // Take the worst grade (highest number) if both are available
            finalGrade = if sugarGrade = null and satFatGrade = null then null
                        else if sugarGrade = null then satFatGrade
                        else if satFatGrade = null then sugarGrade  
                        else if sugarGrade >= satFatGrade then sugarGrade else satFatGrade,
            
            gradeText = if finalGrade = null then "NO_DATA"
                       else if finalGrade = 1 then "A"
                       else if finalGrade = 2 then "B"
                       else if finalGrade = 3 then "C"
                       else "D",
            
            dataQuality = if sugarValue <> null and satFatValue <> null then "COMPLETE"
                         else if sugarValue <> null or satFatValue <> null then "PARTIAL_DATA"
                         else "NO_NUTRITION_DATA"
        in
            [Grade = gradeText, DataQuality = dataQuality],

    // Add normalized text column
    AddNormalizedText = Table.AddColumn(Source, "normalized_text", each
        if [ninutritionlin] = null then null
        else NormalizeNutritionText(Text.From([ninutritionlin]))
    ),

    // Extract saturated fat values
    AddSaturatedFat = Table.AddColumn(AddNormalizedText, "saturated_fat_g", each
        if [normalized_text] = null then null
        else try ExtractSaturatedFat([normalized_text]) otherwise null
    ),

    // Extract sugar values
    AddSugar = Table.AddColumn(AddSaturatedFat, "sugar_g", each
        if [normalized_text] = null then null
        else try ExtractSugar([normalized_text]) otherwise null
    ),

    // Add rounded sugar values
    AddRoundedSugar = Table.AddColumn(AddSugar, "sugar_g_rounded", each
        if [sugar_g] = null then null
        else Number.Round([sugar_g], 0)
    ),

    // Add classification
    AddClassification = Table.AddColumn(AddRoundedSugar, "classification", each
        try ClassifyNutrition([sugar_g], [saturated_fat_g]) otherwise [Grade = "ERROR", DataQuality = "PROCESSING_ERROR"]
    ),

    // Expand classification record into separate columns
    ExpandClassification = Table.ExpandRecordColumn(AddClassification, "classification", {"Grade", "DataQuality"}, {"grade", "data_quality"}),

    // Reorder and select final columns - include debug info
    FinalTable = Table.SelectColumns(ExpandClassification, {
        "ref", 
        "code",
        "uf_ibarcode", 
        "group",
        "commod",
        "is_drink",
        "product", 
        "grade", 
        "data_quality",
        "sugar_g", 
        "sugar_g_rounded",
        "saturated_fat_g",
        "ninutritionlin",
        "units"
    }),
    #"Renamed Columns" = Table.RenameColumns(FinalTable,{{"ref", "Order Ref"}, {"code", "Product Code"}, {"uf_ibarcode", "EAN"}, {"group", "Prod Grp"}, {"product", "Desc"}, {"grade", "Nutri-Grade"}, {"data_quality", "Data quality"}, {"sugar_g", "Sugar g / %"}, {"sugar_g_rounded", "Sugar g Rounded"}, {"saturated_fat_g", "Saturated fat g / %"}, {"ninutritionlin", "Linear Nutrition"}, {"units", "Units"}})
    
in
    #"Renamed Columns"
    