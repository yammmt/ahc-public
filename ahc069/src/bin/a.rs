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
// 各グループに対する探索の打ち切り時間
const SEARCH_TIME_LIMIT: Duration = Duration::from_micros(1_300);
// 各グループに対する配置・移動処理の絶対時間制限
const HARD_TIME_LIMIT: Duration = Duration::from_micros(1_500);
// 残された空き正方形領域を評価する重み λ
const REMAINING_AREA_WEIGHT: f64 = 0.02;
// 人数の最大値 150 を収められる正方形の最小の一辺
const MAX_SQUARE_SIDE: usize = 13;
// グループの生成に用いられる時刻の上限
const TIME_HORIZON: i64 = 100_000;
// P の生成分布から求めた平均人数
const AVERAGE_GROUP_SIZE: f64 = 59.5;
// 滞在時間の平均を推定する際の事前分布
const DURATION_PRIOR_MEAN: f64 = 5_000.0;
const DURATION_PRIOR_WEIGHT: f64 = 20.0;
// 序盤に明確に効率が悪いとみなす利用料 / セル時間の上限
const EARLY_EFFICIENCY_THRESHOLD: f64 = 0.50;
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

fn remaining_square_score(cells: &[Cell], grass: &[Vec<bool>], occupied: &[Vec<bool>]) -> f64 {
    let n = grass.len();
    let mut in_region = vec![vec![false; n]; n];
    for &(x, y) in cells {
        in_region[x][y] = true;
    }

    // largest_square[x][y] は (x, y) を左上とする空き正方形の最大の一辺。
    let mut largest_square = vec![vec![0_usize; n + 1]; n + 1];
    let mut side_histogram = [0_usize; MAX_SQUARE_SIDE + 1];
    for x in (0..n).rev() {
        for y in (0..n).rev() {
            if grass[x][y] && !occupied[x][y] && !in_region[x][y] {
                let side = 1 + largest_square[x + 1][y]
                    .min(largest_square[x][y + 1])
                    .min(largest_square[x + 1][y + 1]);
                largest_square[x][y] = side;
                side_histogram[side.min(MAX_SQUARE_SIDE)] += 1;
            }
        }
    }

    // Q_k を空いている k x k 正方形の配置位置数として、
    // F(B-R) = sum_{k=2}^{13} log(1 + Q_k) を計算する。
    let mut square_count = 0_usize;
    let mut score = 0.0;
    for side in (2..=MAX_SQUARE_SIDE).rev() {
        square_count += side_histogram[side];
        score += (square_count as f64).ln_1p();
    }
    score
}

fn evaluate_region(cells: &[Cell], grass: &[Vec<bool>], occupied: &[Vec<bool>]) -> f64 {
    let compactness = 4.0 * (cells.len() as f64).sqrt() / boundary_len(cells, grass.len()) as f64;
    compactness + REMAINING_AREA_WEIGHT * remaining_square_score(cells, grass, occupied)
}

fn find_best_region(
    grass: &[Vec<bool>],
    components: &AvailableComponents,
    occupied: &[Vec<bool>],
    required_size: usize,
    deadline: Instant,
) -> Option<Vec<(usize, usize)>> {
    let n = grass.len();
    let mut included_in_candidates = vec![vec![false; n]; n];
    let mut next_start_index = 0;
    let mut best_candidate: Option<(f64, Vec<Cell>)> = None;

    loop {
        if Instant::now() >= deadline {
            break;
        }

        let mut start = None;
        while next_start_index < n * n {
            let x = next_start_index / n;
            let y = next_start_index % n;
            next_start_index += 1;
            if components.sizes[x][y] >= required_size && !included_in_candidates[x][y] {
                start = Some((x, y));
                break;
            }
        }
        let Some(start) = start else {
            break;
        };

        let Some(cells) = find_region_from_start(grass, occupied, start, required_size, deadline)
        else {
            break;
        };

        for &(x, y) in &cells {
            included_in_candidates[x][y] = true;
        }
        let evaluation = evaluate_region(&cells, grass, occupied);
        if best_candidate
            .as_ref()
            .is_none_or(|(best_evaluation, _)| evaluation > *best_evaluation)
        {
            best_candidate = Some((evaluation, cells));
        }
    }

    best_candidate.map(|(_, cells)| cells)
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

fn should_reject_by_efficiency(
    value: i64,
    group_size: usize,
    duration: i64,
    boundary: usize,
    estimated_load: f64,
    efficiency_threshold: f64,
) -> bool {
    if estimated_load <= 1.0 {
        return false;
    }
    let efficiency =
        usage_fee(value, group_size, boundary) as f64 / (group_size as f64 * duration as f64);
    efficiency < efficiency_threshold
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
    let grass_area = grass.iter().flatten().filter(|&&cell| cell).count();
    let mut occupied = vec![vec![false; N]; N];
    let mut regions: Vec<Vec<(usize, usize)>> = vec![Vec::new(); M];
    let mut departure_times = vec![0_i64; M];
    let mut group_ids = vec![0_usize; M];
    let mut group_sizes = vec![0_usize; M];
    let mut values = vec![0_i64; M];
    let mut maximum_boundaries = vec![0_usize; M];
    let mut active = vec![false; M];
    let mut observed_duration_sum = 0_i64;
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

        let duration = t - s;
        observed_duration_sum += duration;
        let estimated_mean_duration = (DURATION_PRIOR_WEIGHT * DURATION_PRIOR_MEAN
            + observed_duration_sum as f64)
            / (DURATION_PRIOR_WEIGHT + i as f64 + 1.0);
        let remaining_group_count = M - i - 1;
        let remaining_time = TIME_HORIZON - s;
        let estimated_load =
            remaining_group_count as f64 * AVERAGE_GROUP_SIZE * estimated_mean_duration
                / (remaining_time as f64 * grass_area as f64);
        let remaining_turn_ratio = remaining_group_count as f64 / (M - 1) as f64;
        let efficiency_threshold = EARLY_EFFICIENCY_THRESHOLD * remaining_turn_ratio;

        let started_at = Instant::now();
        let search_deadline = started_at + SEARCH_TIME_LIMIT;
        let hard_deadline = started_at + HARD_TIME_LIMIT;
        let available_components =
            calculate_available_component_sizes(&grass, &occupied, search_deadline);
        let normal_region = available_components.as_ref().and_then(|components| {
            find_best_region(&grass, components, &occupied, p, search_deadline)
        });
        let normal_boundary = normal_region.as_ref().map(|cells| boundary_len(cells, N));
        let reject_normal_region = normal_boundary.is_some_and(|boundary| {
            should_reject_by_efficiency(
                v,
                p,
                duration,
                boundary,
                estimated_load,
                efficiency_threshold,
            )
        });

        if reject_normal_region {
            writeln!(out, "0").unwrap();
            writeln!(out, "No").unwrap();
        } else if let Some(cells) = normal_region
            && Instant::now() < hard_deadline
        {
            writeln!(out, "0").unwrap();
            writeln!(out, "Yes").unwrap();
            for &(x, y) in &cells {
                writeln!(out, "{x} {y}").unwrap();
                occupied[x][y] = true;
            }
            regions[i] = cells;
            maximum_boundaries[i] = normal_boundary.unwrap();
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
                                find_best_region(&grass, &components, &occupied, p, search_deadline)
                            });

                    let mut candidate = None;
                    if let Some(arriving_cells) = arriving_region
                        && Instant::now() < hard_deadline
                    {
                        let arriving_boundary = boundary_len(&arriving_cells, N);
                        let reject_arriving_region = should_reject_by_efficiency(
                            v,
                            p,
                            duration,
                            arriving_boundary,
                            estimated_load,
                            efficiency_threshold,
                        );

                        if !reject_arriving_region {
                            for &(x, y) in &arriving_cells {
                                occupied[x][y] = true;
                            }

                            let moved_region = calculate_available_component_sizes(
                                &grass,
                                &occupied,
                                search_deadline,
                            )
                            .and_then(|components| {
                                find_best_region(
                                    &grass,
                                    &components,
                                    &occupied,
                                    group_sizes[j],
                                    search_deadline,
                                )
                            });

                            let mut moved_candidate = None;
                            if let Some(moved_cells) = moved_region
                                && Instant::now() < hard_deadline
                            {
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
                                candidate =
                                    Some((arriving_cells, moved_cells, moved_maximum_boundary));
                            }
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
