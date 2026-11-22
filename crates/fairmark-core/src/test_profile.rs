//! Test profiles of fairmark-core (issue #368).
//!
//! The tests that sweep the golden inputs (every file, or several files per method with
//! perturbed fields) run in full in normal CI. Under mutation testing each mutant runs the
//! whole test suite, so those sweeps dominate its cost. cargo-mutants sets
//! `FAIRMARK_TEST_PROFILE=mutants` (`.cargo/mutants.toml`, through `cargo --config env.*`), and
//! in that profile each sweep checks a fixed sample of its inputs per method instead.
//!
//! The sample is deterministic: no random choice, no dependence on the machine. Without the
//! variable, or with the value `full`, every test runs as written. Any other value is refused,
//! so that a typing error never silently selects a profile.
//!
//! This file is compiled only into tests: `lib.rs` declares it under `#[cfg(test)]` and the
//! integration tests include it with `#[path]`, so one definition serves both. Its own test is
//! in `lib.rs`, so that it runs once, in the library.

/// The variable that selects the profile.
pub const VARIABLE: &str = "FAIRMARK_TEST_PROFILE";

/// The test profile of the current process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Every test as written: normal CI and local runs.
    Full,
    /// The sweeps over golden inputs check a fixed sample per method: mutation testing.
    Mutants,
}

/// The profile named by `FAIRMARK_TEST_PROFILE`; `Full` when the variable is not set.
///
/// # Panics
/// On a value other than `full` or `mutants`.
pub fn profile() -> Profile {
    match std::env::var(VARIABLE) {
        Err(std::env::VarError::NotPresent) => Profile::Full,
        Ok(v) if v == "full" => Profile::Full,
        Ok(v) if v == "mutants" => Profile::Mutants,
        other => panic!("{VARIABLE} must be `full` or `mutants`, found {other:?}"),
    }
}

/// `full` in the full profile, `mutants` in the mutants profile.
pub fn pick<T>(full: T, mutants: T) -> T {
    match profile() {
        Profile::Full => full,
        Profile::Mutants => mutants,
    }
}

/// A size setting of a sweep: the integer in the variable `name` when it is set (to widen a
/// search by hand), otherwise [`pick`]`(full, mutants)`.
///
/// # Panics
/// On a value of `name` that is not a non-negative integer.
pub fn count(name: &str, full: usize, mutants: usize) -> usize {
    match std::env::var(name) {
        Ok(v) => v
            .parse()
            .unwrap_or_else(|_| panic!("{name} must be a non-negative integer, found {v:?}")),
        Err(_) => pick(full, mutants),
    }
}

/// Golden inputs per method that a sweep over every golden file checks in the mutants profile.
pub const MUTANTS_PER_METHOD: usize = 2;

/// The items of a sweep over every golden file: all of them in the full profile; in the
/// mutants profile, for each group (the method, or the directory of the file), at most
/// [`MUTANTS_PER_METHOD`] items spread evenly over the group in its order: positions
/// `j × n / k` for `j` in `0..k`, with `n` items in the group and `k` kept. The order of the
/// items is preserved.
pub fn sample<T, K: PartialEq>(items: Vec<T>, group: impl Fn(&T) -> K) -> Vec<T> {
    if profile() == Profile::Full {
        return items;
    }
    // The position of each item within its group, and the size of each group.
    let mut groups: Vec<(K, usize)> = Vec::new();
    let mut position = Vec::with_capacity(items.len());
    for item in &items {
        let key = group(item);
        match groups.iter_mut().position(|(k, _)| *k == key) {
            Some(g) => {
                position.push((g, groups[g].1));
                groups[g].1 += 1;
            }
            None => {
                position.push((groups.len(), 0));
                groups.push((key, 1));
            }
        }
    }
    let kept = |g: usize, i: usize| {
        let n = groups[g].1;
        let k = n.min(MUTANTS_PER_METHOD);
        (0..k).any(|j| j * n / k == i)
    };
    items
        .into_iter()
        .zip(position)
        .filter(|(_, (g, i))| kept(*g, *i))
        .map(|(item, _)| item)
        .collect()
}
