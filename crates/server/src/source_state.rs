#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Unscanned,
    Scanning,
    Scanned,
    Errored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEvent {
    StartScan,
    Done,
    Error,
}

impl SourceState {
    pub const ALL: [SourceState; 4] = [
        SourceState::Unscanned,
        SourceState::Scanning,
        SourceState::Scanned,
        SourceState::Errored,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SourceState::Unscanned => "unscanned",
            SourceState::Scanning => "scanning",
            SourceState::Scanned => "scanned",
            SourceState::Errored => "errored",
        }
    }

    pub fn parse(text: &str) -> Option<SourceState> {
        SourceState::ALL
            .into_iter()
            .find(|state| state.as_str() == text)
    }

    pub fn apply(self, event: SourceEvent) -> Option<SourceState> {
        match (self, event) {
            (
                SourceState::Unscanned | SourceState::Scanned | SourceState::Errored,
                SourceEvent::StartScan,
            ) => Some(SourceState::Scanning),
            (SourceState::Scanning, SourceEvent::Done) => Some(SourceState::Scanned),
            (SourceState::Scanning, SourceEvent::Error) => Some(SourceState::Errored),
            _ => None,
        }
    }
}

impl SourceEvent {
    /// The states this event may leave, for use in a guarded `UPDATE`.
    pub fn from_states(self) -> Vec<String> {
        SourceState::ALL
            .into_iter()
            .filter(|state| state.apply(self).is_some())
            .map(|state| state.as_str().to_string())
            .collect()
    }

    pub fn target(self) -> SourceState {
        match self {
            SourceEvent::StartScan => SourceState::Scanning,
            SourceEvent::Done => SourceState::Scanned,
            SourceEvent::Error => SourceState::Errored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_scan_leaves_unscanned_scanned_and_errored() {
        for state in [
            SourceState::Unscanned,
            SourceState::Scanned,
            SourceState::Errored,
        ] {
            assert_eq!(
                state.apply(SourceEvent::StartScan),
                Some(SourceState::Scanning)
            );
        }
    }

    #[test]
    fn start_scan_is_rejected_while_scanning() {
        assert_eq!(SourceState::Scanning.apply(SourceEvent::StartScan), None);
    }

    #[test]
    fn done_moves_scanning_to_scanned() {
        assert_eq!(
            SourceState::Scanning.apply(SourceEvent::Done),
            Some(SourceState::Scanned)
        );
    }

    #[test]
    fn error_moves_scanning_to_errored() {
        assert_eq!(
            SourceState::Scanning.apply(SourceEvent::Error),
            Some(SourceState::Errored)
        );
    }

    #[test]
    fn done_and_error_are_rejected_outside_scanning() {
        for state in [
            SourceState::Unscanned,
            SourceState::Scanned,
            SourceState::Errored,
        ] {
            assert_eq!(state.apply(SourceEvent::Done), None);
            assert_eq!(state.apply(SourceEvent::Error), None);
        }
    }

    #[test]
    fn from_states_lists_the_states_an_event_leaves() {
        assert_eq!(
            SourceEvent::StartScan.from_states(),
            vec!["unscanned", "scanned", "errored"]
        );
        assert_eq!(SourceEvent::Done.from_states(), vec!["scanning"]);
    }

    #[test]
    fn target_is_the_state_an_event_enters() {
        assert_eq!(SourceEvent::StartScan.target(), SourceState::Scanning);
        assert_eq!(SourceEvent::Done.target(), SourceState::Scanned);
        assert_eq!(SourceEvent::Error.target(), SourceState::Errored);
    }

    #[test]
    fn states_round_trip_through_their_names() {
        for state in SourceState::ALL {
            assert_eq!(SourceState::parse(state.as_str()), Some(state));
        }

        assert_eq!(SourceState::parse("paused"), None);
    }
}
