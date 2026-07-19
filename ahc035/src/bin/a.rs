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

fn make_layout(seeds: &[Vec<usize>], n: usize) -> Vec<Vec<usize>> {
    let m = seeds[0].len();
    let order = bfs_order(n, (n / 2, n / 2));
    let mut layout = vec![vec![usize::MAX; n]; n];
    let mut used_seed = vec![false; seeds.len()];
    let directions = [(-1_isize, 0_isize), (1, 0), (0, -1), (0, 1)];

    // The first seed maximizes its largest component.  Ties are broken by
    // total value, then by the seed number (the iteration order below).
    let mut first = 0;
    let mut first_key = (0, 0);
    for (k, seed) in seeds.iter().enumerate() {
        let key = (
            seed.iter().copied().max().unwrap(),
            seed.iter().sum::<usize>(),
        );
        if key > first_key {
            first = k;
            first_key = key;
        }
    }

    for (step, &(r, c)) in order.iter().enumerate() {
        let chosen = if step == 0 {
            first
        } else {
            let neighbours: Vec<usize> = directions
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
                .collect();

            // `max_by_key` keeps the later item on a tie, so use an explicit
            // comparison to retain the smallest seed number as required.
            let mut best_seed = None;
            let mut best_score = 0;
            for candidate in 0..seeds.len() {
                if used_seed[candidate] {
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
        };

        debug_assert_eq!(seeds[chosen].len(), m);
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
