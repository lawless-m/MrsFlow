let
    Source = Table.FromRows(Json.Document(Binary.Decompress(Binary.FromText("VdDJbQMxDEDRXuZMINyXWgz3llpSWUYaGyIB3R5ISf/1upB+7sPIesElQOigLn+/1xse5C8yGBOUeEM5yFHAaQ31IImCCDW0NmkKqtjQGyJCOTeMtpYE2Ptknq94IZhUw/qigxuCUvsn4ZlcD+Js/yQ6dwoGWGbDVgh93dnwaWtrLQZW5uy6gJktEWfTBeSOYTp7PpCuwbPlhmA1p9nxs+q+wmbDDWIhZLPffpWKWPlst8C0UtBntz0RVMk4m32eq6w0e+0JNHK+4f0P", BinaryEncoding.Base64), Compression.Deflate)), let _t = ((type nullable text) meta [Serialized.Text = true]) in type table [#"Budget Date" = _t, #"Budget Value" = _t]),
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"Budget Date", type date}, {"Budget Value", Int64.Type}})
in
    #"Changed Type"