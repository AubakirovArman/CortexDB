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
        let mut frontier = BTreeSet::from([entry]);
        let mut scores = BTreeMap::new();
        while let Some(candidate) =
            best_frontier(query, &frontier, &self.vectors, &self.config.metric)
        {
            frontier.remove(&candidate);
            if self.deleted.contains(&candidate) || !visited.insert(candidate) {
                continue;
            }
            if visited_count.saturating_add(visited.len()) > max_visited {
                budget_exceeded = true;
                break;
            }
            if visited.len() > self.ef_search {
                continue;
            }
            if allowed.is_none_or(|values| values.contains(&candidate)) {
                if let Some(score) = self
                    .config
                    .metric
                    .distance(query, &self.vectors[&candidate])
                {
                    scores.insert(candidate, score);
                }
            }
            if let Some(neighbors) = self.links.get(&candidate) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        frontier.insert(*neighbor);
                    }
                }
            }
        }
        visited_count += visited.len();
        (ranked(scores, limit), visited_count, budget_exceeded)
    }

    fn entry_point_with_budget(
        &self,
        query: &[i16],
        max_visited: Option<usize>,
    ) -> Option<(u32, usize, bool)> {
        let max_visited = max_visited.unwrap_or(usize::MAX);
        let mut visited = 0usize;
        let mut current = self
            .upper_links
            .iter()
            .rev()
            .find_map(|(_, links)| links.keys().next().copied())
            .or_else(|| self.vectors.keys().next().copied())?;
        for links in self.upper_links.values().rev() {
            let mut improved = true;
            while improved {
                if visited >= max_visited {
                    return Some((current, visited, true));
                }
                visited += 1;
                improved = false;
                let current_score = self
                    .vectors
                    .get(&current)
                    .and_then(|vector| self.config.metric.distance(query, vector))
                    .unwrap_or(0);
                if let Some(neighbors) = links.get(&current) {
                    for neighbor in neighbors {
                        let Some(score) = self
                            .vectors
                            .get(neighbor)
                            .and_then(|vector| self.config.metric.distance(query, vector))
                        else {
                            continue;
                        };
                        if score > current_score {
                            current = *neighbor;
                            improved = true;
                            break;
                        }
                    }
                }
            }
        }
        Some((current, visited, false))
    }
}

fn best_frontier(
    query: &[i16],
    frontier: &BTreeSet<u32>,
    vectors: &BTreeMap<u32, Vec<i16>>,
    metric: &DistanceMetric,
) -> Option<u32> {
    frontier
        .iter()
        .copied()
        .filter_map(|cell_id| {
            metric
                .distance(query, &vectors[&cell_id])
                .map(|score| (cell_id, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(cell_id, _)| cell_id)
}
