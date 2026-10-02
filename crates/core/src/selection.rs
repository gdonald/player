/// Adds the id when absent and removes it when present.
pub fn toggle(selected: &[i64], id: i64) -> Vec<i64> {
    if selected.contains(&id) {
        selected
            .iter()
            .copied()
            .filter(|other| *other != id)
            .collect()
    } else {
        let mut next = selected.to_vec();
        next.push(id);
        next
    }
}

/// The select-all box is checked when every visible row is.
pub fn all_selected(selected: &[i64], visible: &[i64]) -> bool {
    !visible.is_empty() && visible.iter().all(|id| selected.contains(id))
}

/// Checking the select-all box selects every row. Unchecking clears them.
pub fn select_all(checked: bool, visible: &[i64]) -> Vec<i64> {
    if checked {
        visible.to_vec()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_adds_a_missing_id() {
        assert_eq!(toggle(&[1], 2), vec![1, 2]);
    }

    #[test]
    fn toggle_removes_a_present_id() {
        assert_eq!(toggle(&[1, 2], 1), vec![2]);
    }

    #[test]
    fn all_selected_needs_every_visible_row() {
        assert!(all_selected(&[2, 1], &[1, 2]));
        assert!(!all_selected(&[1], &[1, 2]));
    }

    #[test]
    fn all_selected_is_false_with_no_rows() {
        assert!(!all_selected(&[], &[]));
    }

    #[test]
    fn select_all_checks_or_clears_every_row() {
        assert_eq!(select_all(true, &[3, 4]), vec![3, 4]);
        assert!(select_all(false, &[3, 4]).is_empty());
    }
}
