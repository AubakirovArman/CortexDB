use super::{AppendEntriesRequest, LogIndex, ReplicatedEntry, Term};

pub(super) fn append_entries(
    log: &mut Vec<ReplicatedEntry>,
    request: &AppendEntriesRequest,
) -> Option<LogIndex> {
    if !matches_previous(log, request.prev_log_index, request.prev_log_term) {
        return None;
    }
    for entry in &request.entries {
        match log
            .iter()
            .position(|existing| existing.index == entry.index)
        {
            Some(position) if log[position].term != entry.term => {
                log.truncate(position);
                log.push(entry.clone());
            }
            Some(_) => {}
            None => log.push(entry.clone()),
        }
    }
    log.sort_by_key(|entry| entry.index);
    Some(log.last().map(|entry| entry.index).unwrap_or_default())
}

pub(super) fn committed_from_leader(leader_commit: LogIndex, last_index: LogIndex) -> LogIndex {
    if leader_commit > last_index {
        last_index
    } else {
        leader_commit
    }
}

fn matches_previous(log: &[ReplicatedEntry], index: LogIndex, term: Term) -> bool {
    if index == LogIndex(0) {
        return term == Term(0);
    }
    log.iter()
        .any(|entry| entry.index == index && entry.term == term)
}
