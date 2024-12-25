use regex::Regex;
use std::cmp::Ordering;

pub fn compare_versions(version_a: &str, version_b: &str) -> Ordering {
    let re = Regex::new(r"(\d+(?:\.\d+)*)(?:-([^+]+))?(?:\+.*)?").unwrap();

    let matcher_a = re.captures(version_a);
    let matcher_b = re.captures(version_b);

    if matcher_a.is_none() || matcher_b.is_none() {
        return version_a.cmp(version_b);
    }

    let matcher_a = matcher_a.unwrap();
    let matcher_b = matcher_b.unwrap();

    let core_a = matcher_a.get(1).map(|m| m.as_str()).unwrap_or("");
    let core_b = matcher_b.get(1).map(|m| m.as_str()).unwrap_or("");
    let cmp = compare_version_groups(core_a, core_b); // Compare version core
    if cmp != Ordering::Equal {
        return cmp;
    }

    let a_has_pre_release = matcher_a.get(2).is_some();
    let b_has_pre_release = matcher_b.get(2).is_some();

    if a_has_pre_release != b_has_pre_release {
        return if a_has_pre_release {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    if a_has_pre_release {
        let pre_a = matcher_a.get(2).map(|m| m.as_str()).unwrap_or("");
        let pre_b = matcher_b.get(2).map(|m| m.as_str()).unwrap_or("");
        let cmp = compare_version_groups(pre_a, pre_b); // Compare pre-release
        if cmp != Ordering::Equal {
            return cmp;
        }
    }

    Ordering::Equal
}

fn compare_version_groups(group_a: &str, group_b: &str) -> Ordering {
    let parts_a: Vec<&str> = group_a.split('.').collect();
    let parts_b: Vec<&str> = group_b.split('.').collect();

    for (part_a, part_b) in parts_a.iter().zip(parts_b.iter()) {
        match (part_a.parse::<u32>(), part_b.parse::<u32>()) {
            (Ok(a), Ok(b)) => {
                let cmp = a.cmp(&b);
                if cmp != Ordering::Equal {
                    return cmp; // Both numeric
                }
            }
            (Ok(_), Err(_)) => return Ordering::Less, // Only A numeric
            (Err(_), Ok(_)) => return Ordering::Greater, // Only B numeric
            (Err(_), Err(_)) => {
                let cmp = part_a.cmp(part_b); // Neither numeric
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }

    parts_a.len().cmp(&parts_b.len()) // Compare part count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn compare() {
        assert_eq!(compare_versions("0.1", "0.2"), Ordering::Less);
        assert_eq!(compare_versions("0.2", "0.1"), Ordering::Greater);
        assert_eq!(compare_versions("0.1", "0.1"), Ordering::Equal);

        assert_eq!(compare_versions("0.1", "0.1.1"), Ordering::Less);
        assert_eq!(compare_versions("0.1.1", "0.1"), Ordering::Greater);
        assert_eq!(compare_versions("0.1.1", "0.1.1"), Ordering::Equal);

        assert_eq!(compare_versions("0.1.1", "0.1.2"), Ordering::Less);
        assert_eq!(compare_versions("0.1.2", "0.1.1"), Ordering::Greater);

        assert_eq!(compare_versions("0.1.1-alpha", "0.1.1"), Ordering::Less);
        assert_eq!(compare_versions("0.1.1", "0.1.1-alpha"), Ordering::Greater);
    }
}
