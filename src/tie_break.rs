use crate::search::{filter_results, SearchResult};
use jiff::{SpanRound, Timestamp, Unit};
use nucleo::{self, Matcher};
use std::cmp::Reverse;

pub fn break_a_tie(mut participants: Vec<SearchResult>, _matcher: &Matcher) -> Vec<SearchResult> {
    // Capture "now" once so every element uses the same baseline
    let now = Timestamp::now();

    // Sort in‐place by descending freshness
    participants.sort_by_key(|participant| {
        // Compute time since last visit (clamped to ≥ 0)
        let days_since = now - participant.last_visit.unwrap();

        // We're operating on hours, not seconds or miliseconds, becuase it's more authentic to the browsing experience. People don't think of their sessions as activities that happen over the course of seconds, but rather hours or days. If freshness was too precise, it might actually conflict with memory.
        let days: u32 = days_since.get_hours() as u32;

        // How many times they’ve visited
        let visits: u32 = participant.visit_count.unwrap();

        // freshness = days × visits; Reverse to get descending order
        Reverse(days * visits)
    });

    participants
}
