use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::backend::MessageInfo;

pub struct QueueComparisonResult {
    pub queue_a: String,
    pub queue_b: String,
    pub only_in_a: Vec<MessageInfo>,
    pub only_in_b: Vec<MessageInfo>,
    pub in_both: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonTab {
    Summary,
    OnlyInA,
    OnlyInB,
}

pub fn compute_comparison(queue_a: &str, queue_b: &str, messages_a: Vec<MessageInfo>, messages_b: Vec<MessageInfo>) -> QueueComparisonResult {
    fn hash_body(body: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        body.hash(&mut hasher);
        hasher.finish()
    }

    // Build hash maps: hash -> count for each queue
    let mut b_hashes: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, msg) in messages_b.iter().enumerate() {
        b_hashes.entry(hash_body(&msg.body)).or_default().push(i);
    }

    let mut matched_b: HashSet<usize> = HashSet::new();
    let mut only_in_a = Vec::new();
    let mut in_both = 0usize;

    for msg in &messages_a {
        let h = hash_body(&msg.body);
        if let Some(indices) = b_hashes.get_mut(&h) {
            // Find an unmatched index in B
            if let Some(pos) = indices.iter().position(|&idx| !matched_b.contains(&idx)) {
                matched_b.insert(indices[pos]);
                in_both += 1;
            } else {
                only_in_a.push(msg.clone());
            }
        } else {
            only_in_a.push(msg.clone());
        }
    }

    let only_in_b: Vec<MessageInfo> = messages_b.iter().enumerate()
        .filter(|(i, _)| !matched_b.contains(i))
        .map(|(_, m)| m.clone())
        .collect();

    QueueComparisonResult {
        queue_a: queue_a.to_string(),
        queue_b: queue_b.to_string(),
        only_in_a,
        only_in_b,
        in_both,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MessageInfo;

    fn msg(body: &str) -> MessageInfo {
        MessageInfo {
            index: 1,
            routing_key: String::new(),
            exchange: String::new(),
            redelivered: false,
            timestamp: None,
            content_type: String::new(),
            headers: vec![],
            body: body.to_string(),
        }
    }

    #[test]
    fn identical_queues() {
        let a = vec![msg("hello"), msg("world")];
        let b = vec![msg("hello"), msg("world")];
        let result = compute_comparison("A", "B", a, b);
        assert_eq!(result.in_both, 2);
        assert!(result.only_in_a.is_empty());
        assert!(result.only_in_b.is_empty());
    }

    #[test]
    fn completely_disjoint() {
        let a = vec![msg("aaa"), msg("bbb")];
        let b = vec![msg("ccc"), msg("ddd")];
        let result = compute_comparison("A", "B", a, b);
        assert_eq!(result.in_both, 0);
        assert_eq!(result.only_in_a.len(), 2);
        assert_eq!(result.only_in_b.len(), 2);
    }

    #[test]
    fn partial_overlap() {
        let a = vec![msg("shared"), msg("only-a")];
        let b = vec![msg("shared"), msg("only-b")];
        let result = compute_comparison("A", "B", a, b);
        assert_eq!(result.in_both, 1);
        assert_eq!(result.only_in_a.len(), 1);
        assert_eq!(result.only_in_b.len(), 1);
        assert_eq!(result.only_in_a[0].body, "only-a");
        assert_eq!(result.only_in_b[0].body, "only-b");
    }

    #[test]
    fn empty_queues() {
        let result = compute_comparison("A", "B", vec![], vec![]);
        assert_eq!(result.in_both, 0);
        assert!(result.only_in_a.is_empty());
        assert!(result.only_in_b.is_empty());
    }

    #[test]
    fn one_empty() {
        let a = vec![msg("hello")];
        let result = compute_comparison("A", "B", a, vec![]);
        assert_eq!(result.in_both, 0);
        assert_eq!(result.only_in_a.len(), 1);
        assert!(result.only_in_b.is_empty());
    }

    #[test]
    fn duplicates_in_one_queue() {
        let a = vec![msg("dup"), msg("dup"), msg("unique")];
        let b = vec![msg("dup")];
        let result = compute_comparison("A", "B", a, b);
        assert_eq!(result.in_both, 1);
        assert_eq!(result.only_in_a.len(), 2); // one dup + unique
        assert!(result.only_in_b.is_empty());
    }

    #[test]
    fn queue_names_preserved() {
        let result = compute_comparison("queue-1", "queue-2", vec![], vec![]);
        assert_eq!(result.queue_a, "queue-1");
        assert_eq!(result.queue_b, "queue-2");
    }
}
