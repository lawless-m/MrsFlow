/*
    given a base folder, allFiles returns a table of every file in the directory tree
	giving
    
    "Folder Path", "Name", "Extension", "Size", "Date created", "Date modified"
    
    let 
    	evalURL = (url) => Expression.Evaluate(Text.FromBinary(Web.Contents(url, [Headers=[Authorization=""], ManualStatusHandling={404}])), #shared)
    in
    	evalURL("https://dw.ramsden-international.com/gogs/Ramsden-International/PowerQueries/raw/master/walkDir.m")
    
    name it walkDir or whatever
    
    invoke it with a folder name
    
    walkDir("\\rivsts05\IMAGE2\Trading\Christmas 2024\Christmas 2024 Artwork & All Side Images\All Side Images")
     
*/

let

    visibleFiles = (folder as text, kind as text) as table =>
        let
            flist = try Table.SelectRows(Folder.Contents(folder), each [Attributes]?[Hidden]? <> true) otherwise Table.FromRecords({})
        in Table.SelectRows(flist, each [Attributes][Kind] = kind),

    dirFiles = (folder as text) as table =>
        let
            sz = Table.ExpandRecordColumn(visibleFiles(folder, "File"), "Attributes", {"Size"}, {"Size"})
        in
            Table.SelectColumns(sz, {"Folder Path", "Name", "Extension", "Size", "Date created", "Date modified"}),

    dirFolders = (folder as text) as table => Table.SelectColumns(Table.AddColumn(visibleFiles(folder, "Folder"), "FullPath", each [Folder Path] & [Name]), {"FullPath"}),

    FilesFolders = (folder as text) as table =>
        let
            subFolders = Table.AddColumn(dirFolders(folder), "SubFolders", each @FilesFolders([FullPath])),
            expandedSubFolders = Table.ExpandTableColumn(subFolders, "SubFolders", {"Folder Path", "Name", "Extension", "Size", "Date created", "Date modified"})
        in
            Table.Combine({dirFiles(folder), Table.SelectColumns(expandedSubFolders, {"Folder Path", "Name", "Extension", "Size", "Date created", "Date modified"})})

in
    (folder as text) as table => Table.TransformColumnTypes(FilesFolders(folder), {{"Date modified", type date}, {"Date created", type date}, {"Size", Int64.Type}})
    
