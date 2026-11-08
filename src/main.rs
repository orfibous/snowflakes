//! Build: rustc --edition=2021 -O main.rs -o snowflakeAlgorithm
//! Creates ./dataset.txt on first run and reuses it on subsequent runs.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::time::{Duration, Instant};

type Snowflake = [u32; 6];
type Comparator = fn(&Snowflake, &Snowflake) -> bool;
const NUMBER_OF_SNOWFLAKES: usize = 100000;
const MAX_SNOWFLAKE_LENGTH: u64 = 255;
const NUMBER_OF_SNOWFLAKE_BRANCHES: usize = 6;

fn duration(d: Duration) -> String {
    let seconds = d.as_secs();
    let base = format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    );
    let micros = d.subsec_micros();
    if micros == 0 {
        base
    } else {
        format!("{}.{:06}", base, micros)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn create_dataset() -> io::Result<()> {
    let start = Instant::now();
    let mut random = BufReader::new(File::open("/dev/urandom")?);
    let mut f = BufWriter::new(File::create("dataset.txt")?);
    writeln!(f, "{}", NUMBER_OF_SNOWFLAKES)?;
    let bound = u64::from(MAX_SNOWFLAKE_LENGTH) + 1;
    let limit = (1u64 << 32) / bound * bound;
    for _ in 0..NUMBER_OF_SNOWFLAKES {
        for arm in 0..NUMBER_OF_SNOWFLAKE_BRANCHES {
            let value = loop {
                let mut bytes = [0; 4];
                random.read_exact(&mut bytes)?;
                let value = u64::from(u32::from_le_bytes(bytes));
                if value < limit {
                    break value % bound;
                }
            };
            write!(f, "{}{}", value, if arm == 5 { "\n" } else { " " })?;
        }
    }
    f.flush()?;
    println!("Dataset created in {}", duration(start.elapsed()));
    Ok(())
}

fn read_dataset() -> io::Result<Vec<Snowflake>> {
    let start = Instant::now();
    let mut lines = BufReader::new(File::open("dataset.txt")?).lines();
    let count: usize = lines
        .next()
        .ok_or_else(|| invalid("Missing count"))??
        .trim()
        .parse()
        .map_err(|_| invalid("Invalid count"))?;
    if count == 0 || count > 100000 {
        return Err(invalid("Count outside problem bounds"));
    }
    let mut ds = Vec::with_capacity(count);
    for _ in 0..count {
        let line = lines.next().ok_or_else(|| invalid("Missing snowflake"))??;
        let arms: Vec<u32> = line
            .split_whitespace()
            .map(|s| s.parse::<u32>().map_err(|_| invalid("Invalid arm")))
            .collect::<io::Result<_>>()?;
        let snowflake: Snowflake = arms.try_into().map_err(|_| invalid("Expected six arms"))?;
        if snowflake.iter().any(|&arm| arm >= 10000000) {
            return Err(invalid("Arm outside problem bounds"));
        }
        ds.push(snowflake);
    }
    println!("Dataset loaded in {}", duration(start.elapsed()));
    Ok(ds)
}

fn basic_comparator(snowflake: &Snowflake, next_snowflake: &Snowflake) -> bool {
    fn direction(left: &Snowflake, right: &Snowflake) -> bool {
        for x in 0..6 {
            for y in 0..6 {
                if (0..6).all(|z| left[(x + z) % 6] == right[(y + z) % 6]) {
                    return true;
                }
            }
        }
        false
    }
    let alike_flag = direction(snowflake, next_snowflake);
    let mut reverse = *next_snowflake;
    reverse.reverse();
    let rev_flag = direction(snowflake, &reverse);
    alike_flag || rev_flag
}

fn better_comparator(snowflake: &Snowflake, next_snowflake: &Snowflake) -> bool {
    let mut left = *snowflake;
    let mut right = *next_snowflake;
    left.sort_unstable();
    right.sort_unstable();
    left == right && basic_comparator(snowflake, next_snowflake)
}

fn advanced_comparator(left: &Snowflake, right: &Snowflake) -> bool {
    for start in 0..6 {
        if left[0] != right[start] {
            continue;
        }

        let clockwise =
            (1..6).all(|offset| left[offset] == right[(start + offset) % 6]);

        if clockwise {
            return true;
        }

        let counterclockwise =
            (1..6).all(|offset| left[offset] == right[(start + 6 - offset) % 6]);

        if counterclockwise {
            return true;
        }
    }

    false
}

fn sum(snowflake: &Snowflake) -> u64 {
    snowflake.iter().map(|&arm| u64::from(arm)).sum()
}

fn sum_better_comparator(snowflake: &Snowflake, next_snowflake: &Snowflake) -> bool {
    sum(snowflake) == sum(next_snowflake) && better_comparator(snowflake, next_snowflake)
}

fn report(flag: bool) {
    println!(
        "{}",
        if flag {
            "Twin snowflakes found."
        } else {
            "No two snowflakes are alike."
        }
    );
}

fn pairwise_search(snowflakes: &[Snowflake], comparator: Comparator, tick: Instant, stop_on_twin: bool) -> bool {
    let mut flag = false;
    let total = snowflakes.len();
    let progress_interval = (total / 10).max(1);
    for i in 0..total.saturating_sub(1) {
        if i != 0 && i % progress_interval == 0 {
            println!(
                "processed {}/{} in {}",
                i,
                total,
                duration(tick.elapsed())
            );
        }
        for j in i + 1..snowflakes.len() {
            if comparator(&snowflakes[i], &snowflakes[j]) {
                flag = true;
                if stop_on_twin {
                    report(true);
                    return true;
                }
            }
        }
    }
    report(flag);
    flag
}

fn find_duplicate_snowflakes(snowflakes: &[Snowflake], comparator: Comparator, tick: Instant) -> bool {
    pairwise_search(snowflakes, comparator, tick, false)
}

fn better_find_duplicate_snowflakes(snowflakes: &[Snowflake], comparator: Comparator, tick: Instant) -> bool {
    pairwise_search(snowflakes, comparator, tick, true)
}

// Six arms below 10^7 have a product below 2^140: five 32-bit limbs are exact.
fn product(snowflake: &Snowflake) -> [u32; 5] {
    let mut limbs = [1, 0, 0, 0, 0];
    for &arm in snowflake {
        let mut carry = 0u64;
        for limb in &mut limbs {
            let value = u64::from(*limb) * u64::from(arm) + carry;
            *limb = value as u32;
            carry = value >> 32;
        }
        debug_assert_eq!(carry, 0);
    }
    limbs
}

fn hashmap_search(
    snowflakes: &[Snowflake],
    comparator: Comparator,
    tick: Instant,
    with_product: bool,
) -> bool {
    let mut groups: HashMap<(u64, [u32; 5]), Vec<Snowflake>> = HashMap::new();
    for snowflake in snowflakes {
        let key = (
            sum(&snowflake),
            if with_product {
                product(&snowflake)
            } else {
                [0; 5]
            },
        );
        groups.entry(key).or_default().push(*snowflake);
    }
    let matching_groups = groups.values().filter(|group| group.len() > 1).count();
    println!(
        "{} groups contain more than one snowflake (grouped in {})",
        matching_groups,
        duration(tick.elapsed())
    );
    for group in groups.values() {
        if group.len() > 1 {
            for i in 0..group.len() - 1 {
                for j in i + 1..group.len() {
                    if comparator(&group[i], &group[j]) {
                        report(true);
                        return true;
                    }
                }
            }
        }
    }
    report(false);
    false
}

fn sum_hashmap_find_duplicate_snowflakes(
    snowflakes: &[Snowflake],
    comparator: Comparator,
    tick: Instant,
) -> bool {
    hashmap_search(snowflakes, comparator, tick, false)
}

fn sum_product_hashmap_find_duplicate_snowflakes(
    snowflakes: &[Snowflake],
    comparator: Comparator,
    tick: Instant,
) -> bool {
    hashmap_search(snowflakes, comparator, tick, true)
}

fn main() -> io::Result<()> {
    // # CCO '07 P2 - Snowflakes
    // -----
    // **Canadian Computing Competition: 2007 Stage 2, Day 1, Problem 2**
    //
    // You may have heard that no two snowflakes are alike. Your task is to write a program to determine whether this is really true. Your program will read information about a collection of snowflakes, and search for a pair that may be identical.
    //
    // Each snowflake has six arms. For each snowflake, your program will be provided with a measurement of the length of each of the six arms. Any pair of snowflakes which have the same lengths of corresponding arms should be flagged by your program as possibly identical.
    //
    // **Note: The original CCO data were weak and have been augmented with some custom test cases.**
    // ## Input Specification
    // ----
    // The first line of input will contain a single integer n, 0 < n ≤ 100 000 0 < n \le 100\,000 , the number of snowflakes to follow. This will be followed by n n lines, each describing a snowflake. Each snowflake will be described by a line containing six integers (each integer is at least 0 0 and less than 10 000 000 10\,000\,000 ), the lengths of the arms of the snowflake. The lengths of the arms will be given in order around the snowflake (either clockwise or counterclockwise), but they may begin with any of the six arms. For example, the same snowflake could be described as `1 2 3 4 5 6` or `4 3 2 1 6 5`.
    // ## Output Specification
    // ----
    // If all of the snowflakes are distinct, your program should print the message: `No two snowflakes are alike.`
    //
    // If there is a pair of possibly identical snowflakes, your program should print the message: `Twin snowflakes found.`
    //
    // ## Sample Input
    // ----
    // ```
    // 2
    // 1 2 3 4 5 6
    // 4 3 2 1 6 5
    // ```
    // ## Output for Sample Input
    // ----
    // ```
    // Twin snowflakes found.
    // ```
    //
    // ## Let's Create a dataset
    // Also let's make a function for loading the data from the file
    println!("LinuxUser Programming Exercise No 14");
    if !std::path::Path::new("dataset.txt").try_exists()? {
        create_dataset()?;
    }
    let snowflakes = read_dataset()?;
    let snowflakes_ref = &snowflakes;
    // ## Solution
    // ----
    // A simple solution is to just compare all snowflakes making all rotations.
    println!("Basic Solution: all rotations");
    println!("We won't run this as it is slow");
    //     let tick = Instant::now();
    //     find_duplicate_snowflakes(snowflakes_ref, basic_comparator, tick);
    //     println!("Basic Solution finished in {}", duration(tick.elapsed()));
    // ## A Better Algorithm
    // The Comparator function can be further improved, but we will keep it for now and try to improve on the algorithm. The current algorithm starts by choosing the first snowflake and comparing it with all the rest. Then it chooses the second snowflake and compares it with all the other snowflakes except the first, and so on.
    //
    // One simple improvement would be to stop comparing the moment we find at least a single pair of twin snowflakes. This will improve our best case scenario, but overall worst case time will remain the same.
    println!("Basic Solution: all rotations, stop on first twins found");
    println!("We also won't run this either");
    //     let tick = Instant::now();
    //     better_find_duplicate_snowflakes(snowflakes_ref, basic_comparator, tick);
    //     println!("Basic Solution finished in {}", duration(tick.elapsed()));
    // ## A Better Comparison
    // The simple solution may work, but it is too slow. In order to make it faster we can use a number of different tricks. One of them is to reverse the question. What if instead of searching for twin snowflakes we immediately stop processing two snowflakes when they are not twins. One easy way to do that is by checking if they contain the same elements (without caring about the order).
    println!("Better Comparator Solution: Do rotations only when snowflakes contain the same elements  (reversed criteria)");
    println!("We also won't run this also, since each Basic Run takes about 2 minutes and 30 seconds");
    //     let tick = Instant::now();
    //     better_find_duplicate_snowflakes(snowflakes_ref, better_comparator, tick);
    //     println!("Better Solution finished in {}", duration(tick.elapsed()));
    // An even faster comparison would be to use the sum of arm lengths of each snowflake. Comparing one number is faster than comparing the whole set of elements. If 2 sets have the same sum, we then compare the set and if the set has the same numbers we then do all the rotations for given snowflakes.
    println!("Sum Comparator (Better Comparator) Solution: Do rotations only when sum of arm lengths is equal (reversed criteria)");
    let tick = Instant::now();
    better_find_duplicate_snowflakes(snowflakes_ref, sum_better_comparator, tick);
    println!("Sum Comparator (Better Comparator) Solution finished in {}", duration(tick.elapsed()));
    // Another, idea would be to use a Sum HashMap. Instead of storing our snowflakes on a list, we will create a HashMap of vectors. Each vector will contain all snowflakes with the same sum of arm lengths. The sums will represent the keys of the HashMap. When our HashMap is complete, we will run our ```better_comparator``` only for the snowflakes in groups with equal sum. Instead of looping through the whole dataset we loop only through small groups.
    println!("Sum Hashmap Solution: Comparison only for snowflakes with same sum");
    let tick = Instant::now();
    sum_hashmap_find_duplicate_snowflakes(snowflakes_ref, better_comparator, tick);
    println!("Sum Hashmap Solution finished in {}", duration(tick.elapsed()));
    // ## Sum + Product HashMap
    //
    // We can improve the Sum HashMap by using both the sum and the product of arm lengths as the HashMap key. Instead of grouping snowflakes only by their sum, we group them by `(sum(snowflake), product(snowflake))`. Rotations and reflections preserve both values, so twin snowflakes will always belong to the same group.
    //
    // As in the previous solution, we load the dataset with `read_dataset()` and use `better_comparator` to compare snowflakes within each group. The product can separate snowflakes that have the same sum, reducing the number of comparisons. Matching keys do not prove that two snowflakes are twins, so we still need the comparator. This version inherits the earlier comparator's limitations; it changes the grouping, not the comparison logic.
    //
    // For six arms, creating the groups takes O(n) expected time and O(n) space. Searching the groups is close to linear when groups remain small, but can still take O(n²) time in the worst case. A zero arm makes the product zero, which reduces the benefit of the additional key.
    println!("Sum + Product Hashmap (Better Comparator) Solution: Comparison only for snowflakes with same sum and product");
    let tick = Instant::now();
    sum_product_hashmap_find_duplicate_snowflakes(snowflakes_ref, better_comparator, tick);
    println!("Sum + Product Hashmap (Better Comparator) Solution finished in {}", duration(tick.elapsed()));
    // ## Advanced Comparator
    //
    // We can improve the Comparator algorithm even more by applying a series of improvements.
    // 1. We fix the first arm of the left snowflake and only rotate the right snowflake.
    // 2. We skip start with a different first arm
    // 3. We use indices for the direction comparisons instead of reversed arrays. (We are starting to hack a bit into rust here)
    // 4. Return immediately at first match.
    // 5. We do not sort anything.
    println!("Sum + Product Hashmap (Advanced Comparator) Solution: Comparison only for snowflakes with same sum and product");
    let tick = Instant::now();
    sum_product_hashmap_find_duplicate_snowflakes(snowflakes_ref, advanced_comparator, tick);
    println!("Sum + Product Hashmap (Advanced Comparator) Solution finished in {}", duration(tick.elapsed()));
    Ok(())
}
