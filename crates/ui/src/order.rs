//! The user's own order of the contact list (issue #243): groups and the contacts of a group in
//! the order the user dragged them into, kept in the preferences. Names and addresses that are
//! not in the stored order keep the list's sorting and come after the ordered ones, so the order
//! survives contacts and groups being added or removed.

/// Most groups kept in an order (design: far more than a contact list has).
pub const MAX_GROUPS: usize = 500;
/// Most contacts kept in the orders of all groups together.
pub const MAX_CONTACTS: usize = 5000;

/// Sorts `items` by their position in `order` (stable); those not in it keep their present
/// order after the ordered ones.
pub fn apply<T>(items: &mut [T], order: &[String], key: impl Fn(&T) -> &str) {
    items.sort_by_key(|i| order.iter().position(|o| o == key(i)).unwrap_or(usize::MAX));
}

/// Moves `item` before `before` (to the end when `None` or `before` is not in it) in an order
/// that starts as `shown` (the list as the user sees it now); entries of the old order that are
/// not shown stay behind, so hidden or collapsed entries keep their place.
pub fn moved(
    old: &[String],
    shown: &[String],
    item: &str,
    before: Option<&str>,
    max: usize,
) -> Vec<String> {
    let mut out: Vec<String> = shown.to_vec();
    for o in old {
        if !out.contains(o) {
            out.push(o.clone());
        }
    }
    out.retain(|o| o != item);
    let at = before
        .and_then(|b| out.iter().position(|o| o == b))
        .unwrap_or(out.len());
    out.insert(at, item.to_string());
    out.truncate(max);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn ordered_first_others_keep_their_sorting() {
        let mut items = s(&["anna", "bert", "carl", "dora"]);
        apply(&mut items, &s(&["dora", "bert"]), |x| x.as_str());
        assert_eq!(items, s(&["dora", "bert", "anna", "carl"]));
        // A new contact (not in the order) comes after the ordered ones.
        let mut items = s(&["anna", "bert", "eve", "dora"]);
        apply(&mut items, &s(&["dora", "bert"]), |x| x.as_str());
        assert_eq!(items, s(&["dora", "bert", "anna", "eve"]));
    }

    #[test]
    fn moving_puts_the_item_before_the_target() {
        let shown = s(&["a", "b", "c"]);
        assert_eq!(moved(&[], &shown, "c", Some("a"), 10), s(&["c", "a", "b"]));
        assert_eq!(moved(&[], &shown, "a", None, 10), s(&["b", "c", "a"]));
        // Entries of the old order that are not shown stay behind.
        assert_eq!(
            moved(&s(&["x", "a"]), &shown, "b", Some("a"), 10),
            s(&["b", "a", "c", "x"])
        );
        // Bounded.
        assert_eq!(moved(&[], &shown, "c", Some("a"), 2), s(&["c", "a"]));
    }
}
