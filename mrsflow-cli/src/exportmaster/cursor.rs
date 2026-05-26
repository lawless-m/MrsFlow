//! Cursor advance: extract the 32-byte cursor-state block from a server
//! response and splice it into the next client fetch. Universal across
//! the four navigation modes (natural-PK, indexed-JOIN, ORDER BY,
//! unindexed-JOIN) per protocol §6.

// TODO: implement once schema.rs + row.rs land. PoC source:
// dbisam_client.py L296-333 (_extract_cursor_block, _splice_cursor_block).
