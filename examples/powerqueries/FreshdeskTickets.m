let

    air_fields = (fields) =>
        #table(
        {
            "Shipment Reference", "Quote/Order Number", "Customer Name", "Final Destination", "What is the gross cargo weight? (Kilograms)", "What is the total volume (cubic meters)?", "Collection Reference (Z or X account/s)", "Arrival Port/Airport/Border", "Delivery date into forwarder", "Requested ETA", "Incoterms 2020", "Estimate number of pallets", "Special Delivery Instructions (e.g booking reference for delivery point)", "Is chill packing/dry ice required?", "Do original documents need to fly with the goods?", "Collection required?", "Temperature required"
        },
            {
                {
                    fields[cf_shipment_reference], 
                    fields[cf_quoteorder_number], 
                    fields[cf_customer_name], 
                    fields[cf_final_destination], 
                    fields[cf_what_is_the_gross_cargo_weight_kilograms],
                    fields[cf_what_is_the_total_volume_cubic_meters],
                    fields[cf_collection_reference_z_or_x_accounts], 
                    Date.ToText(Date.FromText(fields[cf_delivery_date_into_forwarder]), "dd-MMM-yy"),
                    fields[cf_arrival_portairportborder], 
                    Date.ToText(Date.FromText(fields[cf_requested_loading_date]), "dd-MMM-yy"),
                    fields[cf_incoterms_2020], 
                    fields[cf_estimated_number_of_pallets], 
                    fields[cf_does_the_container_contain_chilled_or_frozen_products_if_so_please_request_a_genset], 
                    fields[cf_is_genset_required], 
                    fields[cf_delivery_address_dap_only], 
                    fields[cf_required_bill], 
                    fields[cf_delivery_address145519]
                }
            }
        ),

    sea_fields = (fields) => 
        #table( 
        {
            "Shipment Reference", "Quote/Order Number", "Customer Name", "Final Destination", "What is the gross cargo weight? (Kilograms)", "What is the total volume (cubic meters)?", "Collection Reference (Z or X account/s)", "Arrival Port/Airport/Border", "Requested Loading Date:", "Requested Loading Time:", "Incoterms 2020", "Container required", "Does the container contain chilled or frozen products? If so, please request a GENSET", "Is GENSET required?", "What temperature is required?", "Estimate number of pallets", "Special Instructions (e.g vessel request, ETA request)", "Where is the container loading from?", "Delivery Address (DAP Only)", "Required Bill", "Consignee details - Name, Address, Email Address, Phone Number"
        },
            {
                {
                    fields[cf_shipment_reference], 
                    fields[cf_quoteorder_number], 
                    fields[cf_customer_name], 
                    fields[cf_final_destination], 
                    fields[cf_what_is_the_gross_cargo_weight_kilograms],
                    fields[cf_what_is_the_total_volume_cubic_meters],
                    fields[cf_collection_reference_z_or_x_accounts],
                    fields[cf_arrival_portairportborder],
                    Date.ToText(Date.FromText(fields[cf_requested_loading_date]), "dd-MMM-yy"),
                    fields[cf_requested_loading_time], //a //r
                    fields[cf_incoterms_2020],
                    fields[cf_container_required], //a
                    fields[cf_does_the_container_contain_chilled_or_frozen_products_if_so_please_request_a_genset],
                    fields[cf_is_genset_required],
                    fields[cf_what_temperature_is_required], //a
                    fields[cf_estimated_number_of_pallets],
                    fields[cf_special_instructions_eg_vessel_request_eta_request], //a
                    fields[cf_where_is_the_container_loading_from], //a
                    fields[cf_delivery_address_dap_only],
                    fields[cf_required_bill],
                    fields[cf_consignee_details_name_address_email_address_phone_number] //a
                }
            }
        ),

    road_fields = (fields) => 
        #table(
        {
            "Shipment Reference", "Quote/Order Number", "Customer Name", "Final Destination", "What is the gross cargo weight? (Kilograms)", "What is the total volume (cubic meters)?", "Collection Reference (Z or X account/s)", "Arrival Port/Airport/Border", "Requested Loading Date", "Collection address", "Incoterms 2020", "Estimate number of pallets", "Special Delivery Instructions (e.g booking reference for delivery point)", "Delivery Address", "Clearing agent contact details", "UK or Euro pallets?", "Temperature"
        },
            {
                {
                    fields[cf_shipment_reference],
                    fields[cf_quoteorder_number],
                    fields[cf_customer_name],
                    fields[cf_final_destination],
                    fields[cf_what_is_the_gross_cargo_weight_kilograms],
                    fields[cf_what_is_the_total_volume_cubic_meters],
                    fields[cf_collection_reference_z_or_x_accounts],
                    fields[cf_arrival_portairportborder],
                    Date.ToText(Date.FromText(fields[cf_requested_loading_date]), "dd-MMM-yy"),
                    fields[cf_collection_address643819], //a
                    fields[cf_incoterms_2020],
                    fields[cf_estimated_number_of_pallets],
                    fields[cf_special_delivery_instructions_eg_booking_reference_for_delivery_point], //a
                    fields[cf_delivery_address145519],
                    fields[cf_clearing_agent_contact_details], //a //s
                    fields[cf_uk_or_euro_pallets], //a
                    fields[cf_temperature] //a
                }
            }
        ),

    default_fields = (place) =>
        #table(
        {
            "Shipment Reference", "Quote/Order Number", "Customer Name", "Final Destination", "Incoterms"
        },
            {
                {
                    "Correct ID?","No Ticket found","For Road","or Sea","or Air"
                }
            }
        ),


    ApiKey = Text.Trim(Text.FromBinary(Web.Contents("https://dw.ramsden-international.com/freshdesk_api_key.txt"))),
    EncodedCredentials = Binary.ToText(Text.ToBinary(ApiKey, BinaryEncoding.Base64)),
        
    TicketId = Excel.CurrentWorkbook(){[Name="ID"]}[Content]{0}[Value],

    // Build URL with the ticket ID
    Source = try
        Json.Document(Web.Contents("https://productdata.freshdesk.com/api/v2/tickets/" & Text.From(TicketId), [
            Headers = [
                #"Authorization" = "Basic " & EncodedCredentials,
                #"Content-Type" = "application/json"
            ],
            ManualStatusHandling = {404, 400, 401, 403, 500, 502, 503}
        ]))
    otherwise
        null,

    Result = if Source = null then 
                default_fields("Empty")
            else if Source[custom_fields][cf_clearing_agent_contact_details] <> null then 
                road_fields(Source[custom_fields])
            else if Source[custom_fields][cf_what_temperature_is_required] <> null then 
                sea_fields(Source[custom_fields])
            else if Source[custom_fields][cf_arrival_portairportborder] <> null then
                air_fields(Source[custom_fields])
            else
                default_fields("Empty")
in
    Result