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

/// How the queue moves on when a track ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Play through once, removing each song as it finishes.
    Play,
    /// Play the same song over and over.
    LoopOne,
    /// Play the queue over and over, returning to the first song after the last.
    LoopAll,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Play => "play",
            Mode::LoopOne => "loop-one",
            Mode::LoopAll => "loop-all",
        }
    }

    /// Reads a stored mode. Anything else is `Play`.
    pub fn parse(stored: Option<&str>) -> Mode {
        match stored {
            Some("loop-one") => Mode::LoopOne,
            Some("loop-all") => Mode::LoopAll,
            _ => Mode::Play,
        }
    }
}

/// What to do next: remove an entry, play an entry, or start the current
/// song over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub remove: Option<i64>,
    pub play: Option<i64>,
    pub restart: bool,
}

const NOTHING: Step = Step {
    remove: None,
    play: None,
    restart: false,
};

/// The entry after the current one, wrapping to the first after the last.
/// The current entry is its own next when it is alone, which starts it over.
fn next_wrapping(queue: &[i64], current: i64) -> Step {
    let Some(index) = queue.iter().position(|id| *id == current) else {
        return NOTHING;
    };

    let next = queue[(index + 1) % queue.len()];

    Step {
        remove: None,
        play: (next != current).then_some(next),
        restart: next == current,
    }
}

fn play_through(queue: &[i64], current: Option<i64>) -> Step {
    let step = advance(queue, current);

    Step {
        remove: step.remove,
        play: step.play,
        restart: false,
    }
}

/// The current track ended. The loop modes keep every song in the queue.
pub fn finished(queue: &[i64], current: Option<i64>, mode: Mode) -> Step {
    let Some(current) = current else {
        return NOTHING;
    };

    match mode {
        Mode::Play => play_through(queue, Some(current)),
        Mode::LoopOne if queue.contains(&current) => Step {
            remove: None,
            play: None,
            restart: true,
        },
        Mode::LoopOne => NOTHING,
        Mode::LoopAll => next_wrapping(queue, current),
    }
}

/// The next button: in `Play` mode it finishes the current song early. In
/// the loop modes it moves to the following song, wrapping at the end, and
/// keeps every song in the queue.
pub fn skip(queue: &[i64], current: Option<i64>, mode: Mode) -> Step {
    let Some(current) = current else {
        return NOTHING;
    };

    match mode {
        Mode::Play => play_through(queue, Some(current)),
        Mode::LoopOne | Mode::LoopAll => next_wrapping(queue, current),
    }
}

/// The first entry in `after` that was not in `before`: the start of what
/// an enqueue just added.
pub fn first_added(before: &[i64], after: &[i64]) -> Option<i64> {
    after.iter().copied().find(|id| !before.contains(id))
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

    fn step(remove: Option<i64>, play: Option<i64>, restart: bool) -> Step {
        Step {
            remove,
            play,
            restart,
        }
    }

    #[test]
    fn modes_round_trip_through_their_names() {
        for mode in [Mode::Play, Mode::LoopOne, Mode::LoopAll] {
            assert_eq!(Mode::parse(Some(mode.as_str())), mode);
        }
    }

    #[test]
    fn missing_or_unknown_modes_are_play() {
        assert_eq!(Mode::parse(None), Mode::Play);
        assert_eq!(Mode::parse(Some("shuffle")), Mode::Play);
    }

    #[test]
    fn play_mode_removes_the_finished_song_and_plays_the_next() {
        assert_eq!(
            finished(&[1, 2, 3], Some(1), Mode::Play),
            step(Some(1), Some(2), false)
        );
    }

    #[test]
    fn play_mode_stops_after_the_last_song() {
        assert_eq!(
            finished(&[1, 2], Some(2), Mode::Play),
            step(Some(2), None, false)
        );
    }

    #[test]
    fn loop_one_starts_the_same_song_over_and_keeps_it() {
        assert_eq!(
            finished(&[1, 2], Some(1), Mode::LoopOne),
            step(None, None, true)
        );
    }

    #[test]
    fn loop_one_does_nothing_when_the_song_left_the_queue() {
        assert_eq!(finished(&[1, 2], Some(9), Mode::LoopOne), NOTHING);
    }

    #[test]
    fn loop_all_plays_the_next_song_and_keeps_the_finished_one() {
        assert_eq!(
            finished(&[1, 2, 3], Some(2), Mode::LoopAll),
            step(None, Some(3), false)
        );
    }

    #[test]
    fn loop_all_returns_to_the_first_song_after_the_last() {
        assert_eq!(
            finished(&[1, 2, 3], Some(3), Mode::LoopAll),
            step(None, Some(1), false)
        );
    }

    #[test]
    fn loop_all_with_one_song_starts_it_over() {
        assert_eq!(
            finished(&[1], Some(1), Mode::LoopAll),
            step(None, None, true)
        );
    }

    #[test]
    fn loop_all_does_nothing_when_the_song_left_the_queue() {
        assert_eq!(finished(&[1, 2], Some(9), Mode::LoopAll), NOTHING);
    }

    #[test]
    fn nothing_happens_when_nothing_is_current() {
        assert_eq!(finished(&[1], None, Mode::LoopAll), NOTHING);
        assert_eq!(skip(&[1], None, Mode::Play), NOTHING);
    }

    #[test]
    fn skip_in_play_mode_removes_the_current_song() {
        assert_eq!(
            skip(&[1, 2], Some(1), Mode::Play),
            step(Some(1), Some(2), false)
        );
    }

    #[test]
    fn skip_in_loop_one_moves_to_the_next_song_and_keeps_both() {
        assert_eq!(
            skip(&[1, 2], Some(2), Mode::LoopOne),
            step(None, Some(1), false)
        );
    }

    #[test]
    fn skip_in_loop_all_wraps_to_the_first_song() {
        assert_eq!(
            skip(&[4, 5], Some(5), Mode::LoopAll),
            step(None, Some(4), false)
        );
    }

    #[test]
    fn first_added_is_the_first_new_entry() {
        assert_eq!(first_added(&[1, 2], &[1, 2, 7, 8]), Some(7));
    }

    #[test]
    fn first_added_is_none_when_nothing_was_added() {
        assert_eq!(first_added(&[1, 2], &[1, 2]), None);
        assert_eq!(first_added(&[1, 2], &[2]), None);
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
