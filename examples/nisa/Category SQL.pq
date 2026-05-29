let
    Source = Odbc.Query("dsn=Exportmaster", "SELECT PRODUCT.CODE, PRODUCT.DESC1, PRODUCT.""GROUP"", MAIN.DESC, SUB.DESC, SUBSUB.DESC#(lf)FROM NISAINT_CS\PRODUCT PRODUCT#(lf)LEFT JOIN PRODGRP MAIN ON LEFT(GROUP,2) = MAIN.CODE#(lf) #(lf)LEFT JOIN PRODGRP SUB ON LEFT(GROUP,4) = SUB.CODE#(lf)LEFT JOIN PRODGRP SUBSUB ON GROUP = SUBSUB.CODE"),
    #"Renamed Columns" = Table.RenameColumns(Source,{{"DESC", "Main Product Category"}, {"DESC1", "Product Description"}, {"DESC_1", "Sub Product Category"}, {"DESC_2", "Sub Sub Product Category"}, {"CODE", "Product Code"}})
in
    #"Renamed Columns"