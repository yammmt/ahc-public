use proconio::input;
use std::collections::VecDeque;
use std::io::{self, Write};

/// Return the cells in the order specified by the document: BFS from (3, 3),
/// visiting neighbours in up, down, left, right order.
fn bfs_order(n: usize, start: (usize, usize)) -> Vec<(usize, usize)> {
    let mut used = vec![vec![false; n]; n];
    let mut queue = VecDeque::new();
    let mut order = Vec::with_capacity(n * n);

    used[start.0][start.1] = true;
    queue.push_back(start);
    let directions = [(-1_isize, 0_isize), (1, 0), (0, -1), (0, 1)];

    while let Some((r, c)) = queue.pop_front() {
        order.push((r, c));
        for &(dr, dc) in &directions {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                continue;
            }
            let (nr, nc) = (nr as usize, nc as usize);
            if !used[nr][nc] {
                used[nr][nc] = true;
                queue.push_back((nr, nc));
            }
        }
    }
    order
}

fn complement_score(a: &[usize], b: &[usize]) -> usize {
    a.iter().zip(b).map(|(&av, &bv)| av.max(bv)).sum()
}

fn total_value(seed: &[usize]) -> usize {
    seed.iter().sum()
}

fn best_seed_for_component(
    seeds: &[Vec<usize>],
    component: usize,
    excluded: Option<usize>,
) -> usize {
    let mut best: Option<usize> = None;
    for candidate in 0..seeds.len() {
        if Some(candidate) == excluded {
            continue;
        }
        if best.is_none()
            || (seeds[candidate][component], total_value(&seeds[candidate]))
                > (
                    seeds[best.unwrap()][component],
                    total_value(&seeds[best.unwrap()]),
                )
        {
            best = Some(candidate);
        }
    }
    best.unwrap()
}

fn protected_seeds(seeds: &[Vec<usize>], capacity: usize) -> Vec<usize> {
    let mut protected = Vec::new();
    let mut backups = Vec::new();

    for component in 0..seeds[0].len() {
        let first = best_seed_for_component(seeds, component, None);
        if !protected.contains(&first) {
            protected.push(first);
        }

        let second = best_seed_for_component(seeds, component, Some(first));
        let risk = seeds[first][component] - seeds[second][component];
        backups.push((risk, component, second));
    }

    // Higher-risk criteria receive their second-best seed first.  The
    // component number makes equally risky choices deterministic.
    backups.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for (_, _, seed) in backups {
        if protected.len() == capacity {
            break;
        }
        if !protected.contains(&seed) {
            protected.push(seed);
        }
    }

    protected
}

fn placed_neighbours(
    layout: &[Vec<usize>],
    n: usize,
    r: usize,
    c: usize,
    directions: &[(isize, isize); 4],
) -> Vec<usize> {
    directions
        .iter()
        .filter_map(|&(dr, dc)| {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr < 0 || nr >= n as isize || nc < 0 || nc >= n as isize {
                return None;
            }
            let seed = layout[nr as usize][nc as usize];
            (seed != usize::MAX).then_some(seed)
        })
        .collect()
}

fn best_complementary_seed(
    seeds: &[Vec<usize>],
    used_seed: &[bool],
    neighbours: &[usize],
    allowed: &[bool],
) -> usize {
    let mut best_seed = None;
    let mut best_score = 0;
    for candidate in 0..seeds.len() {
        if used_seed[candidate] || !allowed[candidate] {
            continue;
        }
        let score: usize = neighbours
            .iter()
            .map(|&adjacent| complement_score(&seeds[adjacent], &seeds[candidate]))
            .sum();
        if best_seed.is_none() || score > best_score {
            best_seed = Some(candidate);
            best_score = score;
        }
    }
    best_seed.unwrap()
}

fn make_layout(seeds: &[Vec<usize>], n: usize) -> Vec<Vec<usize>> {
    let order = bfs_order(n, (n / 2, n / 2));
    let mut layout = vec![vec![usize::MAX; n]; n];
    let mut used_seed = vec![false; seeds.len()];
    let directions = [(-1_isize, 0_isize), (1, 0), (0, -1), (0, 1)];
    // Protect the best value of every criterion, then add second-best seeds
    // for the criteria where losing the maximum would hurt the most.
    let inner_order: Vec<(usize, usize)> = order
        .iter()
        .copied()
        .filter(|&(r, c)| r > 0 && r + 1 < n && c > 0 && c + 1 < n)
        .collect();
    let protected = protected_seeds(seeds, inner_order.len());
    let mut is_protected = vec![false; seeds.len()];
    for &seed in &protected {
        is_protected[seed] = true;
    }
    debug_assert!(protected.len() <= inner_order.len());

    // The first protected seed follows the base strategy's initial rule.
    let first = protected
        .iter()
        .copied()
        .max_by_key(|&k| {
            (
                seeds[k].iter().copied().max().unwrap(),
                total_value(&seeds[k]),
                usize::MAX - k,
            )
        })
        .unwrap();

    for (step, &(r, c)) in inner_order.iter().take(protected.len()).enumerate() {
        let chosen = if step == 0 {
            first
        } else {
            let neighbours = placed_neighbours(&layout, n, r, c, &directions);
            best_complementary_seed(seeds, &used_seed, &neighbours, &is_protected)
        };
        layout[r][c] = chosen;
        used_seed[chosen] = true;
    }

    // Fill every remaining cell in the original BFS order using all unused
    // seeds, as in the base strategy.
    let all_seeds = vec![true; seeds.len()];
    for &(r, c) in &order {
        if layout[r][c] != usize::MAX {
            continue;
        }
        let neighbours = placed_neighbours(&layout, n, r, c, &directions);
        let chosen = best_complementary_seed(seeds, &used_seed, &neighbours, &all_seeds);
        layout[r][c] = chosen;
        used_seed[chosen] = true;
    }

    layout
}

fn main() {
    input! {
        n: usize,
        m: usize,
        t: usize,
    }
    let seed_count = 2 * n * (n - 1);
    input! {
        mut seeds: [[usize; m]; seed_count],
    }

    for turn in 0..t {
        let layout = make_layout(&seeds, n);
        for row in &layout {
            println!(
                "{}",
                row.iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        io::stdout().flush().unwrap();

        if turn + 1 != t {
            input! {
                next_seeds: [[usize; m]; seed_count],
            }
            seeds = next_seeds;
        }
    }
}
