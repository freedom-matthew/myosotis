use std::collections::HashSet;

use crate::card::CardId;

/// Share of meaningful words two cards must have in common to count as similar.
const SIMILAR_ENOUGH: f64 = 0.6;

const STOP_WORDS: [&str; 34] = [
    "the", "and", "for", "that", "this", "with", "from", "are", "was", "were", "what", "which",
    "who", "whom", "why", "how", "when", "where", "does", "did", "can", "could", "should", "would",
    "will", "has", "have", "had", "not", "but", "its", "their", "than", "then",
];

fn meaningful_words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() > 2 && !STOP_WORDS.contains(word))
        .map(|word| word.strip_suffix('s').unwrap_or(word).to_string())
        .collect()
}

/// Word overlap between two texts, from 0 (nothing shared) to 1 (the same words).
pub fn similarity(first: &str, second: &str) -> f64 {
    let first: HashSet<String> = meaningful_words(first);
    let second: HashSet<String> = meaningful_words(second);
    if first.is_empty() || second.is_empty() {
        return 0.0;
    }
    let shared: usize = first.intersection(&second).count();
    shared as f64 / ((first.len() * second.len()) as f64).sqrt()
}

/// The existing card most like this text, if any is similar enough.
pub fn find_duplicate(text: &str, existing: &[(CardId, String)]) -> Option<CardId> {
    existing
        .iter()
        .map(|(id, other)| (*id, similarity(text, other)))
        .filter(|(_, score)| *score >= SIMILAR_ENOUGH)
        .max_by(|first, second| first.1.total_cmp(&second.1))
        .map(|(id, _)| id)
}

/// Groups cards that resemble each other, leaving out pairs known to be different.
pub fn similar_groups(cards: &[(CardId, String)], apart: &[(CardId, CardId)]) -> Vec<Vec<CardId>> {
    let mut group_of: Vec<usize> = (0..cards.len()).collect();
    for first in 0..cards.len() {
        for second in (first + 1)..cards.len() {
            let pair: (CardId, CardId) = (cards[first].0, cards[second].0);
            let known_apart: bool = apart.contains(&pair) || apart.contains(&(pair.1, pair.0));
            if !known_apart && similarity(&cards[first].1, &cards[second].1) >= SIMILAR_ENOUGH {
                let (from, to): (usize, usize) = (group_of[second], group_of[first]);
                for group in &mut group_of {
                    if *group == from {
                        *group = to;
                    }
                }
            }
        }
    }

    let mut groups: Vec<Vec<CardId>> = Vec::new();
    for label in 0..cards.len() {
        let members: Vec<CardId> = (0..cards.len())
            .filter(|index| group_of[*index] == label)
            .map(|index| cards[index].0)
            .collect();
        if members.len() > 1 {
            groups.push(members);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewordings_of_one_idea_are_similar() {
        let score: f64 = similarity("What triggers a habit? A cue", "Which cue triggers habits?");
        assert!(score >= SIMILAR_ENOUGH, "{score}");
    }

    #[test]
    fn different_ideas_are_not_similar() {
        let score: f64 = similarity(
            "What triggers a habit? A cue",
            "Who wrote Hamlet? Shakespeare",
        );
        assert!(score < SIMILAR_ENOUGH, "{score}");
    }

    #[test]
    fn text_with_only_small_words_matches_nothing() {
        assert_eq!(similarity("is it", "is it"), 0.0);
    }

    #[test]
    fn groups_join_cards_linked_through_a_shared_neighbour() {
        let cards: Vec<(CardId, String)> = vec![
            (1, "habit cue routine reward".to_string()),
            (2, "habit cue routine loop".to_string()),
            (3, "planet orbit gravity star".to_string()),
        ];
        assert_eq!(similar_groups(&cards, &[]), vec![vec![1, 2]]);
    }

    #[test]
    fn pairs_known_to_differ_stay_apart() {
        let cards: Vec<(CardId, String)> = vec![
            (1, "habit cue routine reward".to_string()),
            (2, "habit cue routine loop".to_string()),
        ];
        assert!(similar_groups(&cards, &[(2, 1)]).is_empty());
    }
}
