use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use super::super::{ranked, ScoredCandidate};
use super::{DistanceMetric, HnswIndex};

impl HnswIndex {
    pub fn search(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        let (results, ..) = self.search_filtered_with_budget(query, None, limit, None);
        results
    }

    pub fn search_allowed(
        &self,
        query: &[i16],
        allowed: &BTreeSet<u32>,
        limit: usize,
    ) -> Vec<ScoredCandidate> {
        self.search_allowed_with_budget(query, allowed, limit, None)
            .0
    }

    pub fn search_allowed_with_budget(
        &self,
        query: &[i16],
        allowed: &BTreeSet<u32>,
        limit: usize,
        max_visited: Option<usize>,
    ) -> (Vec<ScoredCandidate>, usize, bool) {
        self.search_filtered_with_budget(query, Some(allowed), limit, max_visited)
    }

    fn search_filtered_with_budget(
        &self,
        query: &[i16],
        allowed: Option<&BTreeSet<u32>>,
        limit: usize,
        max_visited: Option<usize>,
    ) -> (Vec<ScoredCandidate>, usize, bool) {
        let Some((entry, mut visited_count, mut budget_exceeded)) =
            self.entry_point_with_budget(query, max_visited)
        else {
            return (Vec::new(), 0, false);
        };
        let max_visited = max_visited.unwrap_or(usize::MAX);
        if max_visited == 0 {
            return (Vec::new(), 0, true);
        }
        if budget_exceeded {
            return (Vec::new(), visited_count, true);
        }

        let mut visited = BTreeSet::new();
        let mut frontier: BTreeSet<(Reverse<u64>, u32)> = BTreeSet::new();
        let mut top_scores: BTreeSet<(u64, u32)> = BTreeSet::new();
        let mut scores = BTreeMap::new();

        if let Some(entry_vector) = self.vectors.get(&entry) {
            if let Some(entry_score) = self.config.metric.distance(query, entry_vector) {
                frontier.insert((Reverse(entry_score), entry));
                top_scores.insert((entry_score, entry));
                if top_scores.len() > self.ef_search {
                    pop_worst_score(&mut top_scores);
                }
            }
        }

        while let Some((candidate_score, candidate)) = pop_best_frontier(&mut frontier) {
            let _ = candidate_score;
            if !visited.insert(candidate) {
                continue;
            }
            visited_count = visited_count.saturating_add(1);
            if visited_count > max_visited {
                budget_exceeded = true;
                break;
            }

            let Some(vector) = self.vectors.get(&candidate) else {
                continue;
            };
            let Some(score) = self.config.metric.distance(query, vector) else {
                continue;
            };

            let expand = top_scores.len() < self.ef_search
                || score > top_scores.iter().next().map(|item| item.0).unwrap_or(0);

            if !self.deleted.contains(&candidate) {
                if allowed.is_none_or(|values| values.contains(&candidate)) {
                    scores.insert(candidate, score);
                }

                if expand {
                    top_scores.insert((score, candidate));
                    if top_scores.len() > self.ef_search {
                        pop_worst_score(&mut top_scores);
                    }
                }
            }

            if expand {
                if let Some(neighbors) = self.links.get(&candidate) {
                    for neighbor in neighbors {
                        push_candidate(
                            query,
                            *neighbor,
                            &self.vectors,
                            self.config.metric,
                            &visited,
                            &self.deleted,
                            &mut frontier,
                        );
                    }
                }
            }
        }

        (ranked(scores, limit), visited_count, budget_exceeded)
    }

    fn entry_point_with_budget(
        &self,
        query: &[i16],
        max_visited: Option<usize>,
    ) -> Option<(u32, usize, bool)> {
        let max_visited = max_visited.unwrap_or(usize::MAX);
        let mut visited_count = 0usize;

        let mut current = self
            .upper_links
            .iter()
            .rev()
            .find_map(|(_, layer_links)| first_live_node_in_map(layer_links, &self.deleted))
            .or_else(|| first_live_node_in_map(&self.links, &self.deleted))
            .or_else(|| {
                self.vectors.iter().find_map(|(cell_id, _)| {
                    if self.deleted.contains(cell_id) {
                        None
                    } else {
                        Some(*cell_id)
                    }
                })
            })?;

        for layer in self.upper_links.keys().rev() {
            let Some(layer_links) = self.upper_links.get(layer) else {
                continue;
            };

            if !self.deleted.contains(&current) && layer_links.contains_key(&current) {
                // keep
            } else if let Some(next) = first_live_node_in_map(layer_links, &self.deleted) {
                current = next;
            }

            loop {
                if visited_count >= max_visited {
                    return Some((current, visited_count, true));
                }
                visited_count = visited_count.saturating_add(1);

                let current_vector = self.vectors.get(&current)?;
                let current_score = self.config.metric.distance(query, current_vector)?;

                let mut improved = false;
                let mut next = current;
                let mut next_score = current_score;
                if let Some(neighbors) = layer_links.get(&current) {
                    for neighbor in neighbors {
                        if self.deleted.contains(neighbor) {
                            continue;
                        }
                        let Some(neighbor_vector) = self.vectors.get(neighbor) else {
                            continue;
                        };
                        let Some(neighbor_score) =
                            self.config.metric.distance(query, neighbor_vector)
                        else {
                            continue;
                        };
                        if neighbor_score > next_score
                            || (neighbor_score == next_score && *neighbor < next)
                        {
                            next = *neighbor;
                            next_score = neighbor_score;
                            improved = true;
                        }
                    }
                }

                if !improved {
                    break;
                }
                current = next;
            }
        }

        if visited_count >= max_visited {
            return Some((current, visited_count, true));
        }
        visited_count = visited_count.saturating_add(1);
        Some((current, visited_count, false))
    }
}

pub(super) fn first_live_node_in_map(
    layer_links: &BTreeMap<u32, BTreeSet<u32>>,
    deleted: &BTreeSet<u32>,
) -> Option<u32> {
    layer_links.iter().find_map(|(candidate, _)| {
        if deleted.contains(candidate) {
            None
        } else {
            Some(*candidate)
        }
    })
}

pub(super) fn pop_worst_score(top_scores: &mut BTreeSet<(u64, u32)>) {
    if let Some(worst) = top_scores.iter().next().copied() {
        top_scores.remove(&worst);
    }
}

pub(super) fn pop_best_frontier(
    frontier: &mut BTreeSet<(Reverse<u64>, u32)>,
) -> Option<(u64, u32)> {
    let (score, candidate) = frontier.iter().next().copied()?;
    frontier.remove(&(score, candidate));
    Some((score.0, candidate))
}

pub(super) fn push_candidate(
    query: &[i16],
    candidate: u32,
    vectors: &BTreeMap<u32, Vec<i16>>,
    metric: DistanceMetric,
    visited: &BTreeSet<u32>,
    deleted: &BTreeSet<u32>,
    frontier: &mut BTreeSet<(Reverse<u64>, u32)>,
) {
    if visited.contains(&candidate) || deleted.contains(&candidate) {
        return;
    }
    let Some(vector) = vectors.get(&candidate) else {
        return;
    };
    let Some(score) = metric.distance(query, vector) else {
        return;
    };
    frontier.insert((Reverse(score), candidate));
}

#[cfg(test)]
mod tests {
    include!("search_impl/tests.rs");
}
