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
