pub(super) fn trace_search(message: &str) {
    if std::env::var_os("CORTEXDB_SEARCH_TRACE").is_some() {
        eprintln!("[cortexdb-search-trace] {message}");
    }
}
