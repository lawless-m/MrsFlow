(url as text, folderPaths as list) as table =>
    let
        // Get the root contents
        Source = SharePoint.Contents(url, [ApiVersion = 15]),

        // Recursive function to navigate through the folder path
        NavigateFolders = (currentFolder as table, path as list) as table =>
            if List.IsEmpty(path) then
                currentFolder
            else
                let
                    nextFolderName = List.First(path),
                    nextFolder = currentFolder{[Name=nextFolderName]}[Content],
                    remainingPath = List.RemoveFirstN(path, 1)
                in
                    @NavigateFolders(nextFolder, remainingPath),

        FinalFolderContents = NavigateFolders(Source, folderPaths)
    in
        FinalFolderContents

    // Example usage
    // url = "https://ramsdenint.sharepoint.com/sites/CustomerServices/",
    // folderPaths = {"7 Sales"},
    // FolderContents = GetFolderContents(url, folderPaths)
    
