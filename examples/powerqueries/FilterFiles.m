let

	FilterFiles = (dir, suffix, instr) => Table.SelectRows(Folder.Files(dir), each Text.EndsWith([Extension], suffix) and Text.Contains([Name], instr))

in FilterFiles
    
