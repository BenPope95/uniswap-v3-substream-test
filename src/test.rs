fn find_name_symbol_pair(storage_changes: Vec<StorageChange>) -> Option<(String, String)> {
    storage_changes.iter().enumerate().reduce(|(_, prev_storage_change), (i, storage_change)| {
        if let Some(current_str) = from_utf8(&prev_storage_change.new_value).ok().map(|s| s.to_owned()) {
            if is_typical_string(&current_str) {
                if let Some(next_str) = from_utf8(&storage_change.new_value).ok().map(|s| s.to_owned()) {
                    if is_symbol(&next_str) {
                        Some((current_str, next_str))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn main() {
    let storage_changes: Vec<StorageChange> = vec![
        // Fill this vector with your actual storage changes.
    ];

    if let Some((name, symbol)) = find_name_symbol_pair(storage_changes) {
        println!("Found name: '{}', symbol: '{}'", name, symbol);
    } else {
        println!("No matching name-symbol pair found.");
    }
}
