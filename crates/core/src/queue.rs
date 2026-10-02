#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Advance {
    pub remove: Option<i64>,
    pub play: Option<i64>,
}

/// When a track ends (or the current entry is deleted): remove the current
/// queue entry and play the entry after it.
pub fn advance(queue: &[i64], current: Option<i64>) -> Advance {
    let Some(current) = current else {
        return Advance {
            remove: None,
            play: None,
        };
    };

    let Some(index) = queue.iter().position(|id| *id == current) else {
        return Advance {
            remove: None,
            play: None,
        };
    };

    Advance {
        remove: Some(current),
        play: queue.get(index + 1).copied(),
    }
}

/// The entry to start when nothing is playing.
pub fn start(queue: &[i64], current: Option<i64>) -> Option<i64> {
    if current.is_some() {
        return None;
    }

    queue.first().copied()
}

pub fn shows_play_button(queue: &[i64], current: Option<i64>) -> bool {
    start(queue, current).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advancing_with_nothing_current_does_nothing() {
        assert_eq!(
            advance(&[1, 2], None),
            Advance {
                remove: None,
                play: None
            }
        );
    }

    #[test]
    fn advancing_when_the_current_entry_is_gone_does_nothing() {
        assert_eq!(
            advance(&[1, 2], Some(9)),
            Advance {
                remove: None,
                play: None
            }
        );
    }

    #[test]
    fn advancing_the_only_entry_removes_it_and_stops() {
        assert_eq!(
            advance(&[1], Some(1)),
            Advance {
                remove: Some(1),
                play: None
            }
        );
    }

    #[test]
    fn advancing_removes_the_current_entry_and_plays_the_next() {
        assert_eq!(
            advance(&[1, 2, 3], Some(1)),
            Advance {
                remove: Some(1),
                play: Some(2)
            }
        );
    }

    #[test]
    fn advancing_from_the_middle_plays_the_following_entry() {
        assert_eq!(
            advance(&[1, 2, 3], Some(2)),
            Advance {
                remove: Some(2),
                play: Some(3)
            }
        );
    }

    #[test]
    fn start_plays_the_first_entry_when_nothing_is_current() {
        assert_eq!(start(&[4, 5], None), Some(4));
    }

    #[test]
    fn start_does_nothing_while_an_entry_is_current() {
        assert_eq!(start(&[4, 5], Some(4)), None);
    }

    #[test]
    fn start_does_nothing_on_an_empty_queue() {
        assert_eq!(start(&[], None), None);
    }

    #[test]
    fn play_button_shows_for_a_stopped_queue_with_entries() {
        assert!(shows_play_button(&[1], None));
    }

    #[test]
    fn play_button_hides_while_playing() {
        assert!(!shows_play_button(&[1], Some(1)));
    }
}
