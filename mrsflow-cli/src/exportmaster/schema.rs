//! Schema parser — decodes the 772-byte column-block region of a SELECT
//! response into typed `Column` descriptors. See protocol §4 + §6b.

// TODO: port from dbisam_client.py L411-435 + protocol §6b type table.
