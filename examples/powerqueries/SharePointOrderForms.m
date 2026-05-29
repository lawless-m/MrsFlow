let
    // Function to navigate through the folder tree
    GetFolderContents = (url as text, folderPath as text) as table =>
        let
            // Get the root contents
            Source = SharePoint.Contents(url, [ApiVersion = 15]),

            // Escape the folder path
            EscapedFolderPath = Uri.EscapeDataString(folderPath),

            // Split the folder path into a list of folder names
            PathList = Text.Split(EscapedFolderPath, "%2F"), // %2F is the encoded value for "/"

            // Function to unescape folder names
            UnescapeFolderName = (folderName as text) as text =>
                Text.Replace(Text.Replace(folderName, "%20", " "), "%27", "'"),

            // Recursive function to navigate through the folder path
            NavigateFolders = (currentFolder as table, path as list) as table =>
                if List.IsEmpty(path) then
                    currentFolder
                else
                    let
                        nextFolderName = UnescapeFolderName(List.First(path)),
                        nextFolder = currentFolder{[Name=nextFolderName]}[Content],
                        remainingPath = List.RemoveFirstN(path, 1)
                    in
                        @NavigateFolders(nextFolder, remainingPath),

            // Get the final folder contents
            FinalFolderContents = NavigateFolders(Source, PathList)
        in
            FinalFolderContents,

    // Function to convert folder contents to a table
    cont2tab = (content as table) as table =>
        let
            // Function to create a record from a row
            CreateRecord = (row) =>
                [Name = row[Name], Content = row[Content], FolderPath = row[Folder Path]],

            // Create a list of records from the content table
            records = List.Transform(Table.ToRecords(content), each CreateRecord(_)),
            tab = Table.FromRecords(records)
        in
            Table.SelectRows(tab, each Text.EndsWith([Name], ".xlsx")),

    cont2fileList = (conts) as table =>
        let
            FolderContentsList = Table.ToRecords(conts),
            AllTables = List.Transform(FolderContentsList, each cont2tab(_[Content]))            
        in
            Table.Combine(AllTables),

    url = "https://ramsdenint.sharepoint.com/sites/CustomerServices/",

    baset = cont2fileList(GetFolderContents(url, "7 Sales/Easter 2025/Order Forms")),
    eaas =  cont2fileList(GetFolderContents(url, "7 Sales/Easter 2025/Order Forms/EAAs"))

in 
    Table.Combine({baset, eaas})
