pub fn path_component_left(path: &str) -> (&str, &str) {
    if let Some((left, right)) = path.split_once('/') {
        (left, right.trim_start_matches('/'))
    } else {
        (path, "")
    }
}

pub fn path_component_right(path: &str) -> (&str, &str) {
    if let Some((left, right)) = path.trim_end_matches('/').rsplit_once('/') {
        (left.trim_end_matches('/'), right)
    } else {
        ("", path)
    }
}
