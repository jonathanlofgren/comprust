use std::collections::HashMap;

pub fn count_chars(source: &str) -> HashMap<char, i32> {
    source.chars().fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_chars() {
        assert_eq!(count_chars("mamma"), HashMap::from([('m', 3), ('a', 2)]));
        assert_eq!(count_chars(""), HashMap::new());
        assert_eq!(
            count_chars("abcd"),
            HashMap::from([('a', 1), ('b', 1), ('c', 1), ('d', 1)])
        );
    }
}
