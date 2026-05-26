//! Row parser — decodes a record's on-disk byte layout into Arrow
//! ArrayRefs, one per column, dispatching on the ftType sub-code from
//! protocol §6b.

// TODO: implement once schema.rs lands.
