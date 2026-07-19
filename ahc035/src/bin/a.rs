use proconio::input;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::Instant;

const HC_TURNS: usize = 2;
const HC_LIMIT_MS: u128 = 1800;
type Cell = (usize, usize);

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

fn grid_edges(n: usize) -> Vec<(Cell, Cell)> {
    let mut edges = Vec::with_capacity(2 * n * (n - 1));
    for r in 0..n {
        for c in 0..n {
            if r + 1 < n {
                edges.push(((r, c), (r + 1, c)));
            }
            if c + 1 < n {
                edges.push(((r, c), (r, c + 1)));
            }
        }
    }
    edges
}

fn child_cdf(a: &[usize], b: &[usize], max_value: usize) -> Vec<f64> {
    let mut probability = vec![0.0; max_value + 1];
    probability[0] = 1.0;
    let mut current_max = 0;

    for (&av, &bv) in a.iter().zip(b) {
        let mut next = vec![0.0; max_value + 1];
        if av == bv {
            for sum in 0..=current_max {
                next[sum + av] += probability[sum];
            }
        } else {
            for sum in 0..=current_max {
                next[sum + av] += probability[sum] * 0.5;
                next[sum + bv] += probability[sum] * 0.5;
            }
        }
        current_max += av.max(bv);
        probability = next;
    }

    let mut cumulative = 0.0;
    for value in &mut probability {
        cumulative += *value;
        *value = cumulative;
    }
    probability
}

fn edge_cdfs(
    layout: &[Vec<usize>],
    seeds: &[Vec<usize>],
    edges: &[(Cell, Cell)],
    max_value: usize,
) -> Vec<Vec<f64>> {
    edges
        .iter()
        .map(|&((r1, c1), (r2, c2))| {
            child_cdf(&seeds[layout[r1][c1]], &seeds[layout[r2][c2]], max_value)
        })
        .collect()
}

fn cdf_product_metadata(cdfs: &[Vec<f64>], max_value: usize) -> (Vec<usize>, Vec<f64>) {
    let mut zero_count = vec![0; max_value + 1];
    let mut nonzero_product = vec![1.0; max_value + 1];
    for cdf in cdfs {
        for value in 0..=max_value {
            if cdf[value] == 0.0 {
                zero_count[value] += 1;
            } else {
                nonzero_product[value] *= cdf[value];
            }
        }
    }
    (zero_count, nonzero_product)
}

fn expected_maximum(zero_count: &[usize], nonzero_product: &[f64], max_value: usize) -> f64 {
    (0..max_value)
        .map(|value| {
            if zero_count[value] == 0 {
                1.0 - nonzero_product[value]
            } else {
                1.0
            }
        })
        .sum()
}

fn seed_after_swap(layout: &[Vec<usize>], cell: Cell, first: Cell, second: Cell) -> usize {
    if cell == first {
        layout[second.0][second.1]
    } else if cell == second {
        layout[first.0][first.1]
    } else {
        layout[cell.0][cell.1]
    }
}

fn affected_edges(edges: &[(Cell, Cell)], first: Cell, second: Cell) -> Vec<usize> {
    edges
        .iter()
        .enumerate()
        .filter_map(|(index, &(a, b))| {
            (a == first || b == first || a == second || b == second).then_some(index)
        })
        .collect()
}

fn swapped_expected_maximum(
    layout: &[Vec<usize>],
    seeds: &[Vec<usize>],
    edges: &[(Cell, Cell)],
    cdfs: &[Vec<f64>],
    zero_count: &[usize],
    nonzero_product: &[f64],
    max_value: usize,
    first: Cell,
    second: Cell,
) -> f64 {
    let affected = affected_edges(edges, first, second);
    let new_cdfs: Vec<Vec<f64>> = affected
        .iter()
        .map(|&edge_index| {
            let (a, b) = edges[edge_index];
            let first_seed = seed_after_swap(layout, a, first, second);
            let second_seed = seed_after_swap(layout, b, first, second);
            child_cdf(&seeds[first_seed], &seeds[second_seed], max_value)
        })
        .collect();

    let mut expectation = 0.0;
    for value in 0..max_value {
        let mut zeros = zero_count[value];
        let mut product = nonzero_product[value];
        for (position, &edge_index) in affected.iter().enumerate() {
            let old = cdfs[edge_index][value];
            if old == 0.0 {
                zeros -= 1;
            } else {
                product /= old;
            }

            let new = new_cdfs[position][value];
            if new == 0.0 {
                zeros += 1;
            } else {
                product *= new;
            }
        }
        expectation += if zeros == 0 { 1.0 - product } else { 1.0 };
    }
    expectation
}

fn hill_climb(layout: &mut [Vec<usize>], seeds: &[Vec<usize>], n: usize, start: &Instant) {
    let edges = grid_edges(n);
    let max_value: usize = (0..seeds[0].len())
        .map(|component| seeds.iter().map(|seed| seed[component]).max().unwrap())
        .sum();
    let inner: Vec<Cell> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .filter(|&(r, c)| r > 0 && r + 1 < n && c > 0 && c + 1 < n)
        .collect();
    let outer: Vec<Cell> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .filter(|cell| !inner.contains(cell))
        .collect();

    let mut cdfs = edge_cdfs(layout, seeds, &edges, max_value);
    let (mut zero_count, mut nonzero_product) = cdf_product_metadata(&cdfs, max_value);
    let mut current = expected_maximum(&zero_count, &nonzero_product, max_value);

    loop {
        let mut best = None;
        for cells in [&inner, &outer] {
            for i in 0..cells.len() {
                for j in i + 1..cells.len() {
                    if start.elapsed().as_millis() >= HC_LIMIT_MS {
                        return;
                    }
                    let score = swapped_expected_maximum(
                        layout,
                        seeds,
                        &edges,
                        &cdfs,
                        &zero_count,
                        &nonzero_product,
                        max_value,
                        cells[i],
                        cells[j],
                    );
                    if score > current + 1e-9
                        && best
                            .as_ref()
                            .is_none_or(|&(best_score, _, _)| score > best_score)
                    {
                        best = Some((score, cells[i], cells[j]));
                    }
                }
            }
        }

        let Some((score, first, second)) = best else {
            return;
        };
        let temporary = layout[first.0][first.1];
        layout[first.0][first.1] = layout[second.0][second.1];
        layout[second.0][second.1] = temporary;
        cdfs = edge_cdfs(layout, seeds, &edges, max_value);
        (zero_count, nonzero_product) = cdf_product_metadata(&cdfs, max_value);
        current = score;
    }
}

fn make_layout(
    seeds: &[Vec<usize>],
    n: usize,
    apply_hill_climb: bool,
    start: &Instant,
) -> Vec<Vec<usize>> {
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

    if apply_hill_climb && start.elapsed().as_millis() < HC_LIMIT_MS {
        hill_climb(&mut layout, seeds, n, start);
    }
    layout
}

fn main() {
    let start = Instant::now();
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
        let layout = make_layout(&seeds, n, turn + HC_TURNS >= t, &start);
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
