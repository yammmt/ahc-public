use proconio::input;
use proconio::marker::Chars;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

// 盤面サイズ 50x50
const N: usize = 50;
// グループ数
const M: usize = 1000;
type Cell = (usize, usize);

struct AvailableComponents {
    sizes: Vec<Vec<usize>>,
    ids: Vec<Vec<usize>>,
    starts: Vec<Cell>,
}

fn calculate_available_component_sizes(
    grass: &[Vec<bool>],
    occupied: &[Vec<bool>],
    deadline: Instant,
) -> Option<AvailableComponents> {
    let n = grass.len();
    let mut component_sizes = vec![vec![0; n]; n];
    let mut component_ids = vec![vec![usize::MAX; n]; n];
    let mut component_starts = Vec::new();

    for start_x in 0..n {
        for start_y in 0..n {
            if Instant::now() >= deadline {
                return None;
            }
            if !grass[start_x][start_y]
                || occupied[start_x][start_y]
                || component_sizes[start_x][start_y] != 0
            {
                continue;
            }

            let mut cells = vec![(start_x, start_y)];
            let mut cursor = 0;
            let component_id = component_starts.len();
            component_sizes[start_x][start_y] = usize::MAX;
            component_ids[start_x][start_y] = component_id;
            component_starts.push((start_x, start_y));

            while cursor < cells.len() {
                if Instant::now() >= deadline {
                    return None;
                }
                let (x, y) = cells[cursor];
                cursor += 1;

                for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if grass[nx][ny] && !occupied[nx][ny] && component_sizes[nx][ny] == 0 {
                        component_sizes[nx][ny] = usize::MAX;
                        component_ids[nx][ny] = component_id;
                        cells.push((nx, ny));
                    }
                }
            }

            for (x, y) in cells {
                component_sizes[x][y] = cursor;
            }
        }
    }

    Some(AvailableComponents {
        sizes: component_sizes,
        ids: component_ids,
        starts: component_starts,
    })
}

fn available_size_after_release(
    region: &[Cell],
    occupied: &[Vec<bool>],
    components: &AvailableComponents,
) -> usize {
    let mut seen_components = vec![false; components.starts.len()];
    let mut available_size = region.len();

    for &(x, y) in region {
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || nx >= N as i32 || ny < 0 || ny >= N as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let component_id = components.ids[nx][ny];
            if !occupied[nx][ny] && component_id != usize::MAX && !seen_components[component_id] {
                seen_components[component_id] = true;
                available_size += components.sizes[nx][ny];
            }
        }
    }

    available_size
}

fn find_region_from_start(
    grass: &[Vec<bool>],
    occupied: &[Vec<bool>],
    start: (usize, usize),
    required_size: usize,
    deadline: Instant,
) -> Option<Vec<(usize, usize)>> {
    let n = grass.len();
    let (x, y) = start;
    let mut visited = vec![vec![false; n]; n];
    // 探索始点からのチェビシェフ距離が小さい候補を優先する。
    // Reverse により、距離・座標の昇順で取り出す min-heap として使う。
    let mut queue = BinaryHeap::from([Reverse((0_usize, x, y))]);
    let mut cells = Vec::with_capacity(required_size);
    visited[x][y] = true;

    while let Some(Reverse((_, cx, cy))) = queue.pop() {
        if Instant::now() >= deadline {
            return None;
        }

        cells.push((cx, cy));
        if cells.len() == required_size {
            return Some(cells);
        }

        // 追加済みマスに隣接する空きマスを追加候補とする。
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if grass[nx][ny] && !occupied[nx][ny] && !visited[nx][ny] {
                visited[nx][ny] = true;
                let distance = nx.abs_diff(x).max(ny.abs_diff(y));
                queue.push(Reverse((distance, nx, ny)));
            }
        }
    }

    None
}

fn boundary_len(cells: &[(usize, usize)], n: usize) -> usize {
    let mut in_region = vec![vec![false; n]; n];
    for &(x, y) in cells {
        in_region[x][y] = true;
    }

    let mut boundary = 0;
    for &(x, y) in cells {
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0
                || nx >= n as i32
                || ny < 0
                || ny >= n as i32
                || !in_region[nx as usize][ny as usize]
            {
                boundary += 1;
            }
        }
    }

    boundary
}

fn find_best_region(
    grass: &[Vec<bool>],
    component_sizes: &[Vec<usize>],
    component_starts: &[(usize, usize)],
    occupied: &[Vec<bool>],
    required_size: usize,
    deadline: Instant,
) -> Option<Vec<(usize, usize)>> {
    if Instant::now() >= deadline {
        return None;
    }
    let mut starts = component_starts
        .iter()
        .copied()
        .filter(|&(x, y)| component_sizes[x][y] >= required_size);
    let first_start = starts.next()?;
    let mut best_cells =
        find_region_from_start(grass, occupied, first_start, required_size, deadline)?;
    if Instant::now() >= deadline {
        return Some(best_cells);
    }
    let mut best_boundary = boundary_len(&best_cells, grass.len());

    for start in starts {
        if Instant::now() >= deadline {
            break;
        }

        if let Some(cells) = find_region_from_start(grass, occupied, start, required_size, deadline)
            && Instant::now() < deadline
        {
            let boundary = boundary_len(&cells, grass.len());
            if boundary < best_boundary {
                best_cells = cells;
                best_boundary = boundary;
            }
        }
    }

    Some(best_cells)
}

fn usage_fee(value: i64, group_size: usize, maximum_boundary: usize) -> i64 {
    let squared = 64_i128 * value as i128 * value as i128 * group_size as i128;
    let boundary = maximum_boundary as i128;
    let lower_ok = |fee: i64| {
        let threshold = (2 * fee as i128 - 1) * boundary;
        threshold <= 0 || threshold * threshold <= squared
    };
    let upper_ok = |fee: i64| {
        let threshold = (2 * fee as i128 + 1) * boundary;
        squared < threshold * threshold
    };
    let mut fee =
        (value as f64 * 4.0 * (group_size as f64).sqrt() / maximum_boundary as f64).round() as i64;
    while !lower_ok(fee) {
        fee -= 1;
    }
    while !upper_ok(fee) {
        fee += 1;
    }
    fee
}

fn move_cost(value: i64, move_cost_rate_milli: i64) -> i64 {
    ((2 * value as i128 * move_cost_rate_milli as i128 + 1000) / 2000).max(1) as i64
}

fn main() {
    input! {
        _n: usize,
        _m: usize,
        r: String,
        rows: [Chars; N],
    }
    let grass: Vec<Vec<bool>> = rows
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell == '.').collect())
        .collect();
    let mut occupied = vec![vec![false; N]; N];
    let mut regions: Vec<Vec<(usize, usize)>> = vec![Vec::new(); M];
    let mut departure_times = vec![0_i64; M];
    let mut group_ids = vec![0_usize; M];
    let mut group_sizes = vec![0_usize; M];
    let mut values = vec![0_i64; M];
    let mut maximum_boundaries = vec![0_usize; M];
    let mut active = vec![false; M];
    let (integer_part, fractional_part) = r.split_once('.').unwrap();
    let move_cost_rate_milli =
        integer_part.parse::<i64>().unwrap() * 1000 + fractional_part.parse::<i64>().unwrap();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for i in 0..M {
        input! {
            group_id: usize,
            s: i64,
            t: i64,
            p: usize,
            v: i64,
        }

        for j in 0..i {
            if active[j] && departure_times[j] < s {
                for &(x, y) in &regions[j] {
                    occupied[x][y] = false;
                }
                active[j] = false;
            }
        }

        group_ids[i] = group_id;
        group_sizes[i] = p;
        values[i] = v;
        departure_times[i] = t;

        let started_at = Instant::now();
        let search_deadline = started_at + Duration::from_micros(1_300);
        let hard_deadline = started_at + Duration::from_micros(1_500);
        let available_components =
            calculate_available_component_sizes(&grass, &occupied, search_deadline);
        let normal_region = available_components.as_ref().and_then(|components| {
            find_best_region(
                &grass,
                &components.sizes,
                &components.starts,
                &occupied,
                p,
                search_deadline,
            )
        });

        if let Some(cells) = normal_region
            && Instant::now() < hard_deadline
        {
            writeln!(out, "0").unwrap();
            writeln!(out, "Yes").unwrap();
            for &(x, y) in &cells {
                writeln!(out, "{x} {y}").unwrap();
                occupied[x][y] = true;
            }
            regions[i] = cells;
            maximum_boundaries[i] = boundary_len(&regions[i], N);
            active[i] = true;
        } else {
            let mut move_candidates = Vec::new();
            if let Some(components) = &available_components {
                for j in 0..i {
                    if Instant::now() >= search_deadline {
                        break;
                    }
                    if active[j]
                        && available_size_after_release(&regions[j], &occupied, components) >= p
                    {
                        move_candidates.push((
                            usage_fee(values[j], group_sizes[j], maximum_boundaries[j]),
                            j,
                        ));
                    }
                }
            }
            move_candidates.sort_unstable();

            let mut accepted_move = None;
            if Instant::now() < search_deadline {
                for (_, j) in move_candidates {
                    if Instant::now() >= search_deadline {
                        break;
                    }

                    for &(x, y) in &regions[j] {
                        occupied[x][y] = false;
                    }

                    let arriving_region =
                        calculate_available_component_sizes(&grass, &occupied, search_deadline)
                            .and_then(|components| {
                                find_best_region(
                                    &grass,
                                    &components.sizes,
                                    &components.starts,
                                    &occupied,
                                    p,
                                    search_deadline,
                                )
                            });

                    let mut candidate = None;
                    if let Some(arriving_cells) = arriving_region
                        && Instant::now() < hard_deadline
                    {
                        for &(x, y) in &arriving_cells {
                            occupied[x][y] = true;
                        }

                        let moved_region =
                            calculate_available_component_sizes(&grass, &occupied, search_deadline)
                                .and_then(|components| {
                                    find_best_region(
                                        &grass,
                                        &components.sizes,
                                        &components.starts,
                                        &occupied,
                                        group_sizes[j],
                                        search_deadline,
                                    )
                                });

                        let mut moved_candidate = None;
                        if let Some(moved_cells) = moved_region
                            && Instant::now() < hard_deadline
                        {
                            let arriving_boundary = boundary_len(&arriving_cells, N);
                            let moved_maximum_boundary =
                                maximum_boundaries[j].max(boundary_len(&moved_cells, N));
                            let score_difference = usage_fee(v, p, arriving_boundary)
                                - move_cost(values[j], move_cost_rate_milli)
                                + usage_fee(values[j], group_sizes[j], moved_maximum_boundary)
                                - usage_fee(values[j], group_sizes[j], maximum_boundaries[j]);
                            if score_difference > 0 && Instant::now() < hard_deadline {
                                moved_candidate = Some((moved_cells, moved_maximum_boundary));
                            }
                        }

                        for &(x, y) in &arriving_cells {
                            occupied[x][y] = false;
                        }
                        if let Some((moved_cells, moved_maximum_boundary)) = moved_candidate {
                            candidate = Some((arriving_cells, moved_cells, moved_maximum_boundary));
                        }
                    }

                    if Instant::now() >= hard_deadline {
                        candidate = None;
                    }

                    if let Some((arriving_cells, moved_cells, moved_maximum_boundary)) = candidate {
                        for &(x, y) in &arriving_cells {
                            occupied[x][y] = true;
                        }
                        for &(x, y) in &moved_cells {
                            occupied[x][y] = true;
                        }
                        accepted_move =
                            Some((j, arriving_cells, moved_cells, moved_maximum_boundary));
                        break;
                    }

                    for &(x, y) in &regions[j] {
                        occupied[x][y] = true;
                    }
                }
            }

            if let Some((j, arriving_cells, moved_cells, moved_maximum_boundary)) = accepted_move {
                writeln!(out, "1").unwrap();
                writeln!(out, "{}", group_ids[j]).unwrap();
                for &(x, y) in &moved_cells {
                    writeln!(out, "{x} {y}").unwrap();
                }
                writeln!(out, "Yes").unwrap();
                for &(x, y) in &arriving_cells {
                    writeln!(out, "{x} {y}").unwrap();
                }

                regions[j] = moved_cells;
                maximum_boundaries[j] = moved_maximum_boundary;
                regions[i] = arriving_cells;
                maximum_boundaries[i] = boundary_len(&regions[i], N);
                active[i] = true;
            } else {
                writeln!(out, "0").unwrap();
                writeln!(out, "No").unwrap();
            }
        }
        out.flush().unwrap();
    }
}
