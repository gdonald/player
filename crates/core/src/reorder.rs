/// The ids with `id` moved to the 1-based `position`, which is clamped to the
/// list. An id that is not in the list leaves it unchanged.
pub fn move_to(ids: &[i64], id: i64, position: i64) -> Vec<i64> {
    let Some(from) = ids.iter().position(|other| *other == id) else {
        return ids.to_vec();
    };

    let mut reordered = ids.to_vec();
    reordered.remove(from);

    let index = usize::try_from(position - 1)
        .unwrap_or(0)
        .min(reordered.len());
    reordered.insert(index, id);

    reordered
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropEdge {
    Above,
    Below,
}

impl DropEdge {
    pub fn class(self) -> &'static str {
        match self {
            DropEdge::Above => "drop-above",
            DropEdge::Below => "drop-below",
        }
    }
}

/// The 1-based position that dropping on the target row moves an entry to.
pub fn drop_position(ids: &[i64], target: i64) -> Option<i64> {
    let index = ids.iter().position(|id| *id == target)?;

    i64::try_from(index + 1).ok()
}

/// Which edge of the target row the moving entry lands on: above when it moves
/// up, below when it moves down, and none when it stays put.
pub fn drop_edge(ids: &[i64], moving: i64, target: i64) -> Option<DropEdge> {
    let from = ids.iter().position(|id| *id == moving)?;
    let to = ids.iter().position(|id| *id == target)?;

    match from.cmp(&to) {
        std::cmp::Ordering::Greater => Some(DropEdge::Above),
        std::cmp::Ordering::Less => Some(DropEdge::Below),
        std::cmp::Ordering::Equal => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_position_is_the_target_rows_place_in_the_list() {
        assert_eq!(drop_position(&[4, 5, 6], 6), Some(3));
    }

    #[test]
    fn drop_position_is_none_for_a_missing_row() {
        assert_eq!(drop_position(&[4, 5, 6], 9), None);
    }

    #[test]
    fn moving_up_lands_above_the_target() {
        assert_eq!(drop_edge(&[1, 2, 3], 3, 1), Some(DropEdge::Above));
        assert_eq!(DropEdge::Above.class(), "drop-above");
    }

    #[test]
    fn moving_down_lands_below_the_target() {
        assert_eq!(drop_edge(&[1, 2, 3], 1, 3), Some(DropEdge::Below));
        assert_eq!(DropEdge::Below.class(), "drop-below");
    }

    #[test]
    fn dropping_on_itself_or_a_missing_row_has_no_edge() {
        assert_eq!(drop_edge(&[1, 2, 3], 2, 2), None);
        assert_eq!(drop_edge(&[1, 2, 3], 9, 2), None);
        assert_eq!(drop_edge(&[1, 2, 3], 2, 9), None);
    }

    #[test]
    fn moving_down_places_the_id_at_the_position() {
        assert_eq!(move_to(&[1, 2, 3, 4], 1, 3), vec![2, 3, 1, 4]);
    }

    #[test]
    fn moving_up_places_the_id_at_the_position() {
        assert_eq!(move_to(&[1, 2, 3, 4], 4, 2), vec![1, 4, 2, 3]);
    }

    #[test]
    fn moving_to_the_current_position_changes_nothing() {
        assert_eq!(move_to(&[1, 2, 3], 2, 2), vec![1, 2, 3]);
    }

    #[test]
    fn positions_outside_the_list_are_clamped_to_its_ends() {
        assert_eq!(move_to(&[1, 2, 3], 2, 0), vec![2, 1, 3]);
        assert_eq!(move_to(&[1, 2, 3], 2, 99), vec![1, 3, 2]);
    }

    #[test]
    fn an_id_not_in_the_list_changes_nothing() {
        assert_eq!(move_to(&[1, 2, 3], 9, 1), vec![1, 2, 3]);
    }
}
