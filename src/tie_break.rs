use crate::search::{filter_results, SearchResult};
use jiff::{SpanRound, Timestamp, Unit};
use nucleo::{self, Matcher};

pub fn break_a_tie(participants: Vec<SearchResult>, matcher: &Matcher) -> Vec<SearchResult> {
    // We can assume this tie only exists because they share a similar title, score, so we're falling back to the "freshness" score

    // A highest rank tuple with the freshness score and the indice of the value that scored it
    let mut highest: (u32, SearchResult) = (
        0,
        participants
            .first()
            .expect("Wouldn't get to this point without one")
            .clone(),
    );

    for participant in &participants {
        // We're operating on hours, not seconds or miliseconds, becuase it's more authentic to the browsing experience. People don't think of their sessions as activities that happen over the course of seconds, but rather hours or days. If freshness was too precise, it might actually conflict with memory.
        let days_since = Timestamp::now() - participant.last_visit.unwrap();

        // Bounding to catch negative days (unwelcome)
        let days: u32 = days_since.get_days().max(0) as u32;
        let visits: u32 = participant.visit_count.unwrap();
        let freshness: u32 = days * visits;

        if highest.0 < freshness {
            highest.1 = participant.clone()
        }
    }

    participants
}
