use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SegmentPayloadCacheKey {
    pub segment_id: u64,
    pub candidate_id: u32,
}

impl SegmentPayloadCacheKey {
    pub(crate) const fn new(segment_id: u64, candidate_id: u32) -> Self {
        Self {
            segment_id,
            candidate_id,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SegmentPayloadCache {
    max_bytes: usize,
    resident_bytes: usize,
    clock: u64,
    segment_loads: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
    entries: BTreeMap<SegmentPayloadCacheKey, CacheEntry>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PayloadCacheStats {
    pub max_bytes: usize,
    pub resident_bytes: usize,
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub segment_loads: u64,
    pub evictions: u64,
}

#[derive(Debug)]
struct CacheEntry {
    payload: Vec<u8>,
    bytes: usize,
    last_used: u64,
}

impl SegmentPayloadCache {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            resident_bytes: 0,
            clock: 0,
            segment_loads: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            entries: BTreeMap::new(),
        }
    }

    pub(crate) fn get(&mut self, key: SegmentPayloadCacheKey) -> Option<Vec<u8>> {
        let next_clock = self.next_clock();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = next_clock;
            let payload = entry.payload.clone();
            self.hits = self.hits.saturating_add(1);
            Some(payload)
        } else {
            self.misses = self.misses.saturating_add(1);
            None
        }
    }

    pub(crate) fn insert(&mut self, key: SegmentPayloadCacheKey, payload: Vec<u8>) {
        if self.max_bytes == 0 || payload.len() > self.max_bytes {
            self.remove(key);
            return;
        }

        self.remove(key);
        let bytes = payload.len();
        let last_used = self.next_clock();
        self.entries.insert(
            key,
            CacheEntry {
                payload,
                bytes,
                last_used,
            },
        );
        self.resident_bytes = self.resident_bytes.saturating_add(bytes);
        self.evict_over_budget();
    }

    pub(crate) fn record_segment_load(&mut self) {
        self.segment_loads = self.segment_loads.saturating_add(1);
    }

    pub(crate) fn stats(&self) -> PayloadCacheStats {
        PayloadCacheStats {
            max_bytes: self.max_bytes,
            resident_bytes: self.resident_bytes,
            entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
            segment_loads: self.segment_loads,
            evictions: self.evictions,
        }
    }

    fn next_clock(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    fn remove(&mut self, key: SegmentPayloadCacheKey) -> bool {
        if let Some(entry) = self.entries.remove(&key) {
            self.resident_bytes = self.resident_bytes.saturating_sub(entry.bytes);
            true
        } else {
            false
        }
    }

    fn evict_over_budget(&mut self) {
        while self.resident_bytes > self.max_bytes {
            let Some(key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            if self.remove(key) {
                self.evictions = self.evictions.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SegmentPayloadCache, SegmentPayloadCacheKey};

    #[test]
    fn cache_returns_payload_and_evicts_least_recently_used_entry() {
        let mut cache = SegmentPayloadCache::new(6);
        let first = SegmentPayloadCacheKey::new(1, 1);
        let second = SegmentPayloadCacheKey::new(1, 2);
        let third = SegmentPayloadCacheKey::new(1, 3);

        cache.insert(first, b"aa".to_vec());
        cache.insert(second, b"bb".to_vec());
        assert_eq!(cache.get(first), Some(b"aa".to_vec()));
        cache.insert(third, b"cccc".to_vec());

        assert_eq!(cache.get(first), Some(b"aa".to_vec()));
        assert_eq!(cache.get(second), None);
        assert_eq!(cache.get(third), Some(b"cccc".to_vec()));
        assert_eq!(cache.stats().resident_bytes, 6);
        assert_eq!(cache.stats().evictions, 1);
        assert_eq!(cache.stats().hits, 3);
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn cache_skips_entries_larger_than_budget() {
        let mut cache = SegmentPayloadCache::new(2);
        let key = SegmentPayloadCacheKey::new(7, 9);

        cache.insert(key, b"large".to_vec());

        assert_eq!(cache.get(key), None);
        assert_eq!(cache.stats().max_bytes, 2);
        assert_eq!(cache.stats().entries, 0);
        assert_eq!(cache.stats().resident_bytes, 0);
    }
}
