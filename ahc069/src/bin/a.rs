use proconio::input;
use proconio::marker::Chars;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

// 盤面サイズ 50x50
const N: usize = 50;
// グループ数
const M: usize = 1000;
// 各グループに対する探索の打ち切り時間 (microseconds)
const SEARCH_TIME_LIMIT_US: u64 = 1_600;
// 各グループに対する配置・移動処理の絶対時間制限 (microseconds)
const HARD_TIME_LIMIT_US: u64 = 1_850;
// 残された空き正方形領域の 1 tick あたりの評価重み λ
const PLACEMENT_SPACE_WEIGHT_PER_TICK: f64 = 0.7;
// 退去時刻が近いグループと接する共有辺 1 本あたりの評価重み μ
const DEPARTURE_AFFINITY_WEIGHT: f64 = 60.0;
// 将来の盤面価値を弱め始める、残りグループ数。この手前までは倍率 1.0。
const ENDGAME_FUTURE_VALUE_START_REMAINING_GROUPS: usize = 40;
// 最終グループ到着時における、将来の盤面価値の倍率。
const ENDGAME_FUTURE_VALUE_FINAL_MULTIPLIER: f64 = 0.0;
// 安価な一次評価から、盤面全体を走査する二次評価へ進める候補数。
const MAX_FULL_EVALUATION_CANDIDATES: usize = 64;
// 現在盤面での二次評価後、予測盤面で再評価する候補数。
const MAX_PROJECTED_EVALUATION_CANDIDATES: usize = 8;
// 探索時間のうち、一次評価候補の収集に使う割合。残りを二次評価用に予約する。
const CANDIDATE_GENERATION_TIME_RATIO: f64 = 0.7;
// 二次評価時間のうち、現在盤面での候補絞り込みに使う割合。
const CURRENT_FULL_EVALUATION_TIME_RATIO: f64 = 0.5;
// 人数の最大値 150 を収められる正方形の最小の一辺
const MAX_SQUARE_SIDE: usize = 13;
// 空き長方形として数える k x 2k の短辺の最大値。最大面積は 8 x 16 = 128。
const MAX_RECTANGLE_SHORT_SIDE: usize = 8;
// 空き正方形評価に加える、2:1 の空き長方形評価の相対重み。
const RECTANGLE_SPACE_RELATIVE_WEIGHT: f64 = 1.5;
// グループの生成に用いられる問題内時刻の上限
const TIME_HORIZON_TICKS: i64 = 100_000;
// 池がまとまっている盤面で、序盤に明確に効率が悪いとみなす利用料 / セル時間の上限
const EARLY_EFFICIENCY_THRESHOLD_BASE: f64 = 0.60;
// 池が最も散在している盤面における、残りターン比率を掛ける前の開始時点の効率閾値
const EARLY_EFFICIENCY_THRESHOLD_MIN: f64 = 0.20;
// 盤面が空いていても、通常の効率閾値に掛ける倍率をこの値より小さくしない。
const FILL_GATE_MIN_MULTIPLIER: f64 = 0.30;
// 盤面が混雑しているとき、効率閾値に掛ける倍率の上限。
const FILL_GATE_MAX_MULTIPLIER: f64 = 1.10;
// 候補受け入れ後の占有率がこの範囲にあるとき、効率閾値の倍率を下限から上限へ線形に強める。
const FILL_GATE_START_RATIO: f64 = 0.40;
const FILL_GATE_END_RATIO: f64 = 0.80;
// 効率閾値を残りグループ数に応じて緩め始める終盤の長さ。
const ENDGAME_EFFICIENCY_THRESHOLD_START_REMAINING_GROUPS: usize = 60;
// (グループ人数の上限, 効率閾値に掛ける倍率)
const GROUP_SIZE_EFFICIENCY_THRESHOLD_MULTIPLIERS: [(usize, f64); 4] =
    [(30, 1.00), (70, 1.0), (110, 1.00), (150, 0.95)];
type Cell = (usize, usize);

const BOARD_MASK: u64 = (1_u64 << N) - 1;

#[derive(Clone)]
struct BitBoard {
    rows: [u64; N],
}

impl BitBoard {
    fn empty() -> Self {
        Self { rows: [0; N] }
    }

    fn contains(&self, x: usize, y: usize) -> bool {
        self.rows[x] & (1_u64 << y) != 0
    }

    fn insert(&mut self, x: usize, y: usize) {
        self.rows[x] |= 1_u64 << y;
    }

    fn remove(&mut self, x: usize, y: usize) {
        self.rows[x] &= !(1_u64 << y);
    }

    fn count(&self) -> usize {
        self.rows.iter().map(|row| row.count_ones() as usize).sum()
    }

    fn from_cells(cells: &[Cell]) -> Self {
        let mut board = Self::empty();
        for &(x, y) in cells {
            board.insert(x, y);
        }
        board
    }
}

struct PlacementEvaluationContext {
    value: i64,
    duration_ticks: i64,
    departure_time_ticks: i64,
    estimated_arrival_rate_per_tick: f64,
    // 残りグループが少ない終盤では、将来の配置のための評価を弱める。
    // 今回の利用料はこの係数に依存させない。
    future_value_weight: f64,
}

struct RegionSearchResult {
    evaluation: f64,
    cells: Vec<Cell>,
    boundary: usize,
    usage_fee: i64,
    secondary_evaluation_timed_out: bool,
}

struct AvailableComponents {
    ids: [usize; N * N],
    sizes: Vec<usize>,
}

fn calculate_available_component_sizes(
    grass: &BitBoard,
    occupied: &BitBoard,
    deadline: Instant,
) -> Option<AvailableComponents> {
    let mut unvisited = [0_u64; N];
    for (x, row) in unvisited.iter_mut().enumerate() {
        *row = grass.rows[x] & !occupied.rows[x] & BOARD_MASK;
    }
    let mut component_ids = [usize::MAX; N * N];
    let mut component_sizes = Vec::new();
    let mut queue = [0_usize; N * N];

    for start_x in 0..N {
        while unvisited[start_x] != 0 {
            if Instant::now() >= deadline {
                return None;
            }
            let start_y = unvisited[start_x].trailing_zeros() as usize;
            let start_index = start_x * N + start_y;
            let component_id = component_sizes.len();
            let mut head = 0;
            let mut tail = 1;
            queue[0] = start_index;
            unvisited[start_x] &= !(1_u64 << start_y);
            component_ids[start_index] = component_id;

            while head < tail {
                if Instant::now() >= deadline {
                    return None;
                }
                let index = queue[head];
                head += 1;
                let x = index / N;
                let y = index % N;

                let mut push = |nx: usize, ny: usize| {
                    let bit = 1_u64 << ny;
                    if unvisited[nx] & bit != 0 {
                        unvisited[nx] &= !bit;
                        let next_index = nx * N + ny;
                        component_ids[next_index] = component_id;
                        queue[tail] = next_index;
                        tail += 1;
                    }
                };
                if y > 0 {
                    push(x, y - 1);
                }
                if y + 1 < N {
                    push(x, y + 1);
                }
                if x > 0 {
                    push(x - 1, y);
                }
                if x + 1 < N {
                    push(x + 1, y);
                }
            }
            component_sizes.push(tail);
        }
    }

    Some(AvailableComponents {
        ids: component_ids,
        sizes: component_sizes,
    })
}

fn available_size_after_release(
    region: &[Cell],
    occupied: &BitBoard,
    components: &AvailableComponents,
) -> usize {
    let mut seen_components = vec![false; components.sizes.len()];
    let mut available_size = region.len();

    for &(x, y) in region {
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || nx >= N as i32 || ny < 0 || ny >= N as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let component_id = components.ids[nx * N + ny];
            if !occupied.contains(nx, ny)
                && component_id != usize::MAX
                && !seen_components[component_id]
            {
                seen_components[component_id] = true;
                available_size += components.sizes[component_id];
            }
        }
    }

    available_size
}

fn find_region_from_start(
    grass: &BitBoard,
    occupied: &BitBoard,
    start: (usize, usize),
    required_size: usize,
    deadline: Instant,
) -> Option<Vec<(usize, usize)>> {
    let (x, y) = start;
    let mut visited = BitBoard::empty();
    // 探索始点からのチェビシェフ距離が小さい候補を優先する。
    // 同じ距離では正方形の外周を左上から時計回りにたどり、端数を連続させる。
    // Reverse により、距離・外周上の順番・座標の昇順で取り出す min-heap として使う。
    let mut queue = BinaryHeap::from([Reverse((0_usize, 0_usize, x, y))]);
    let mut cells = Vec::with_capacity(required_size);
    visited.insert(x, y);

    while let Some(Reverse((_, _, cx, cy))) = queue.pop() {
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
            if nx < 0 || nx >= N as i32 || ny < 0 || ny >= N as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if grass.contains(nx, ny) && !occupied.contains(nx, ny) && !visited.contains(nx, ny) {
                visited.insert(nx, ny);
                let distance = nx.abs_diff(x).max(ny.abs_diff(y));
                let ring_order = chebyshev_ring_order(
                    nx as isize - x as isize,
                    ny as isize - y as isize,
                    distance,
                );
                queue.push(Reverse((distance, ring_order, nx, ny)));
            }
        }
    }

    None
}

fn chebyshev_ring_order(dx: isize, dy: isize, distance: usize) -> usize {
    if distance == 0 {
        return 0;
    }

    let d = distance as isize;
    if dx == -d {
        // 上辺を左から右へ進む。
        (dy + d) as usize
    } else if dy == d {
        // 右辺を上から下へ進む。右上は上辺に含める。
        (2 * d + dx + d) as usize
    } else if dx == d {
        // 下辺を右から左へ進む。右下は右辺に含める。
        (4 * d + d - dy) as usize
    } else {
        // 左辺を下から上へ進む。両端の角は上下辺に含める。
        debug_assert_eq!(dy, -d);
        (6 * d + d - dx) as usize
    }
}

fn boundary_len(cells: &[Cell]) -> usize {
    let region = BitBoard::from_cells(cells);
    let mut boundary = 0;
    for x in 0..N {
        let row = region.rows[x];
        boundary += (row & !(row << 1)).count_ones() as usize;
        boundary += (row & !(row >> 1)).count_ones() as usize;
        boundary +=
            (row & !region.rows.get(x.wrapping_sub(1)).copied().unwrap_or(0)).count_ones() as usize;
        boundary += (row & !region.rows.get(x + 1).copied().unwrap_or(0)).count_ones() as usize;
    }
    boundary
}

fn calculate_early_efficiency_threshold(grass: &BitBoard) -> f64 {
    let grass_count = grass.count();
    let pond_count = N * N - grass_count;
    if grass_count == 0 || pond_count == 0 {
        return EARLY_EFFICIENCY_THRESHOLD_BASE;
    }

    // 芝生と池が接する辺を数える。各辺は右と下だけを見て 1 回ずつ数える。
    let mut grass_pond_boundary = 0_usize;
    for x in 0..N {
        grass_pond_boundary += ((grass.rows[x] ^ (grass.rows[x] >> 1)) & ((1_u64 << (N - 1)) - 1))
            .count_ones() as usize;
        if x + 1 < N {
            grass_pond_boundary += (grass.rows[x] ^ grass.rows[x + 1]).count_ones() as usize;
        }
    }

    // 少ない方のセルがすべて反対種のセルに囲まれた場合を散在度 1 とする。
    let maximum_boundary = 4 * grass_count.min(pond_count);
    let fragmentation = grass_pond_boundary as f64 / maximum_boundary as f64;

    EARLY_EFFICIENCY_THRESHOLD_BASE
        - (EARLY_EFFICIENCY_THRESHOLD_BASE - EARLY_EFFICIENCY_THRESHOLD_MIN) * fragmentation
}

fn group_size_efficiency_threshold_multiplier(group_size: usize) -> f64 {
    for &(maximum_group_size, multiplier) in &GROUP_SIZE_EFFICIENCY_THRESHOLD_MULTIPLIERS {
        if group_size <= maximum_group_size {
            return multiplier;
        }
    }
    GROUP_SIZE_EFFICIENCY_THRESHOLD_MULTIPLIERS
        .last()
        .unwrap()
        .1
}

fn calculate_fill_gate(occupied_area: usize, group_size: usize, grass_area: usize) -> f64 {
    let fill_ratio = (occupied_area + group_size) as f64 / grass_area as f64;
    let progress = ((fill_ratio - FILL_GATE_START_RATIO)
        / (FILL_GATE_END_RATIO - FILL_GATE_START_RATIO))
        .clamp(0.0, 1.0);
    FILL_GATE_MIN_MULTIPLIER + (FILL_GATE_MAX_MULTIPLIER - FILL_GATE_MIN_MULTIPLIER) * progress
}

fn calculate_efficiency_threshold_endgame_multiplier(remaining_group_count: usize) -> f64 {
    if ENDGAME_EFFICIENCY_THRESHOLD_START_REMAINING_GROUPS == 0
        || remaining_group_count >= ENDGAME_EFFICIENCY_THRESHOLD_START_REMAINING_GROUPS
    {
        return 1.0;
    }

    remaining_group_count as f64 / ENDGAME_EFFICIENCY_THRESHOLD_START_REMAINING_GROUPS as f64
}

fn calculate_future_value_weight(remaining_group_count: usize) -> f64 {
    if ENDGAME_FUTURE_VALUE_START_REMAINING_GROUPS == 0
        || remaining_group_count >= ENDGAME_FUTURE_VALUE_START_REMAINING_GROUPS
    {
        return 1.0;
    }

    let progress =
        remaining_group_count as f64 / ENDGAME_FUTURE_VALUE_START_REMAINING_GROUPS as f64;
    ENDGAME_FUTURE_VALUE_FINAL_MULTIPLIER + (1.0 - ENDGAME_FUTURE_VALUE_FINAL_MULTIPLIER) * progress
}

fn rectangle_placement_count(available_rows: &[u64; N], height: usize, width: usize) -> usize {
    let mut count = 0;
    for top_x in 0..=N - height {
        let mut common_columns = BOARD_MASK;
        for row in &available_rows[top_x..top_x + height] {
            common_columns &= row;
        }

        let mut start_columns = common_columns;
        for offset in 1..width {
            start_columns &= common_columns >> offset;
        }
        count += start_columns.count_ones() as usize;
    }
    count
}

fn remaining_rectangle_score(available_rows: &[u64; N]) -> f64 {
    let mut score = 0.0;
    for short_side in 2..=MAX_RECTANGLE_SHORT_SIDE {
        let long_side = 2 * short_side;
        let horizontal_count = rectangle_placement_count(available_rows, short_side, long_side);
        let vertical_count = rectangle_placement_count(available_rows, long_side, short_side);
        score += (horizontal_count as f64).ln_1p() + (vertical_count as f64).ln_1p();
    }
    score
}

fn remaining_space_score(cells: &[Cell], grass: &BitBoard, occupied: &BitBoard) -> f64 {
    let in_region = BitBoard::from_cells(cells);
    let mut available_rows = [0_u64; N];
    for (x, available) in available_rows.iter_mut().enumerate() {
        *available = grass.rows[x] & !occupied.rows[x] & !in_region.rows[x] & BOARD_MASK;
    }

    // largest_square[x][y] は (x, y) を左上とする空き正方形の最大の一辺。
    let mut largest_square = [0_u8; (N + 1) * (N + 1)];
    let mut side_histogram = [0_usize; MAX_SQUARE_SIDE + 1];
    for x in (0..N).rev() {
        let available = available_rows[x];
        for y in (0..N).rev() {
            if available & (1_u64 << y) != 0 {
                let index = x * (N + 1) + y;
                let side = 1 + largest_square[index + N + 1]
                    .min(largest_square[index + 1])
                    .min(largest_square[index + N + 2]);
                largest_square[index] = side;
                side_histogram[(side as usize).min(MAX_SQUARE_SIDE)] += 1;
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
    score + RECTANGLE_SPACE_RELATIVE_WEIGHT * remaining_rectangle_score(&available_rows)
}

fn projected_occupied_board(occupied_until_ticks: &[Vec<i64>], time_ticks: i64) -> BitBoard {
    let mut occupied = BitBoard::empty();
    for (x, row) in occupied_until_ticks.iter().enumerate() {
        for (y, &departure_time_ticks) in row.iter().enumerate() {
            if departure_time_ticks > time_ticks {
                occupied.insert(x, y);
            }
        }
    }
    occupied
}

fn departure_affinity_score(
    cells: &[Cell],
    occupied_until_ticks: &[Vec<i64>],
    departure_time_ticks: i64,
    estimated_arrival_rate_per_tick: f64,
) -> f64 {
    if estimated_arrival_rate_per_tick == 0.0 {
        return 0.0;
    }

    let n = occupied_until_ticks.len();
    let mut score = 0.0;
    for &(x, y) in cells {
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                continue;
            }
            let adjacent_departure_time_ticks = occupied_until_ticks[nx as usize][ny as usize];
            if adjacent_departure_time_ticks == 0 {
                continue;
            }
            let difference_ticks =
                departure_time_ticks.abs_diff(adjacent_departure_time_ticks) as f64;
            let expected_arrival_count = difference_ticks * estimated_arrival_rate_per_tick;
            score += (-expected_arrival_count).exp();
        }
    }
    score
}

fn evaluate_region_cheap(
    cells: &[Cell],
    occupied_until_ticks: &[Vec<i64>],
    context: &PlacementEvaluationContext,
) -> (f64, usize, i64) {
    let boundary = boundary_len(cells);
    let fee = usage_fee(context.value, cells.len(), boundary);
    let evaluation = fee as f64
        + context.future_value_weight
            * DEPARTURE_AFFINITY_WEIGHT
            * departure_affinity_score(
                cells,
                occupied_until_ticks,
                context.departure_time_ticks,
                context.estimated_arrival_rate_per_tick,
            );
    (evaluation, boundary, fee)
}

fn evaluate_region_with_space_score(
    cheap_evaluation: f64,
    space_score: f64,
    context: &PlacementEvaluationContext,
) -> f64 {
    cheap_evaluation
        + context.future_value_weight
            * PLACEMENT_SPACE_WEIGHT_PER_TICK
            * context.duration_ticks as f64
            * space_score
}

fn retain_cheap_candidate(
    candidates: &mut Vec<(f64, usize, i64, Vec<Cell>)>,
    evaluation: f64,
    boundary: usize,
    usage_fee: i64,
    cells: Vec<Cell>,
) {
    if candidates.len() < MAX_FULL_EVALUATION_CANDIDATES {
        candidates.push((evaluation, boundary, usage_fee, cells));
        return;
    }

    let (worst_index, worst_evaluation) = candidates
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| left.0.total_cmp(&right.0))
        .map(|(index, candidate)| (index, candidate.0))
        .unwrap();
    if evaluation > worst_evaluation {
        candidates[worst_index] = (evaluation, boundary, usage_fee, cells);
    }
}

fn find_best_region(
    grass: &BitBoard,
    components: &AvailableComponents,
    occupied: &BitBoard,
    occupied_until_ticks: &[Vec<i64>],
    required_size: usize,
    context: &PlacementEvaluationContext,
    deadline: Instant,
) -> Option<RegionSearchResult> {
    let candidate_generation_started_at = Instant::now();
    let remaining_search_time = deadline.saturating_duration_since(candidate_generation_started_at);
    let candidate_generation_deadline = candidate_generation_started_at
        + remaining_search_time.mul_f64(CANDIDATE_GENERATION_TIME_RATIO);
    let mut included_in_candidates = BitBoard::empty();
    let mut used_as_start = BitBoard::empty();
    let mut next_start_index = 0;
    let mut cheap_candidates = Vec::with_capacity(MAX_FULL_EVALUATION_CANDIDATES);
    let mut exhausted_ordered_starts = false;

    loop {
        if Instant::now() >= candidate_generation_deadline {
            break;
        }

        let mut start = None;
        while next_start_index < N * N {
            let x = next_start_index / N;
            let y = next_start_index % N;
            next_start_index += 1;
            let component_id = components.ids[x * N + y];
            if component_id != usize::MAX
                && components.sizes[component_id] >= required_size
                && !included_in_candidates.contains(x, y)
            {
                start = Some((x, y));
                break;
            }
        }
        let Some(start) = start else {
            exhausted_ordered_starts = true;
            break;
        };
        used_as_start.insert(start.0, start.1);

        let Some(cells) = find_region_from_start(
            grass,
            occupied,
            start,
            required_size,
            candidate_generation_deadline,
        ) else {
            break;
        };

        for &(x, y) in &cells {
            included_in_candidates.insert(x, y);
        }
        let (evaluation, boundary, usage_fee) =
            evaluate_region_cheap(&cells, occupied_until_ticks, context);
        retain_cheap_candidate(
            &mut cheap_candidates,
            evaluation,
            boundary,
            usage_fee,
            cells,
        );
    }

    if exhausted_ordered_starts && Instant::now() < candidate_generation_deadline {
        let mut unused_starts = Vec::new();
        for x in 0..N {
            if Instant::now() >= candidate_generation_deadline {
                break;
            }
            for y in 0..N {
                let component_id = components.ids[x * N + y];
                if component_id != usize::MAX
                    && components.sizes[component_id] >= required_size
                    && !used_as_start.contains(x, y)
                {
                    unused_starts.push((x, y));
                }
            }
        }
        let mut rng = SmallRng::seed_from_u64(0);
        unused_starts.shuffle(&mut rng);

        for start in unused_starts {
            if Instant::now() >= candidate_generation_deadline {
                break;
            }

            let Some(cells) = find_region_from_start(
                grass,
                occupied,
                start,
                required_size,
                candidate_generation_deadline,
            ) else {
                break;
            };
            let (evaluation, boundary, usage_fee) =
                evaluate_region_cheap(&cells, occupied_until_ticks, context);
            retain_cheap_candidate(
                &mut cheap_candidates,
                evaluation,
                boundary,
                usage_fee,
                cells,
            );
        }
    }

    cheap_candidates.sort_unstable_by(|left, right| right.0.total_cmp(&left.0));
    let current_full_evaluation_started_at = Instant::now();
    let current_full_evaluation_deadline = current_full_evaluation_started_at
        + deadline
            .saturating_duration_since(current_full_evaluation_started_at)
            .mul_f64(CURRENT_FULL_EVALUATION_TIME_RATIO);
    let mut current_full_candidates = Vec::new();
    for (cheap_evaluation, boundary, usage_fee, cells) in cheap_candidates {
        if !current_full_candidates.is_empty() && Instant::now() >= current_full_evaluation_deadline
        {
            break;
        }
        let space_score = remaining_space_score(cells.as_slice(), grass, occupied);
        let evaluation = evaluate_region_with_space_score(cheap_evaluation, space_score, context);
        current_full_candidates.push((
            evaluation,
            cheap_evaluation,
            space_score,
            boundary,
            usage_fee,
            cells,
        ));
    }

    current_full_candidates.sort_unstable_by(|left, right| right.0.total_cmp(&left.0));
    current_full_candidates.truncate(MAX_PROJECTED_EVALUATION_CANDIDATES);

    let mut best_candidate = current_full_candidates
        .first()
        .map(|candidate| (candidate.0, candidate.3, candidate.4, candidate.5.clone()));
    if context.future_value_weight > 0.0 && Instant::now() < deadline {
        let arrival_time_ticks = context.departure_time_ticks - context.duration_ticks;
        let one_third_time_ticks = arrival_time_ticks + context.duration_ticks / 3;
        let two_thirds_time_ticks = arrival_time_ticks + context.duration_ticks * 2 / 3;
        let one_third_occupied =
            projected_occupied_board(occupied_until_ticks, one_third_time_ticks);
        let two_thirds_occupied =
            projected_occupied_board(occupied_until_ticks, two_thirds_time_ticks);

        let mut projected_best_candidate = None;
        for (_, cheap_evaluation, current_space_score, boundary, usage_fee, cells) in
            current_full_candidates
        {
            if projected_best_candidate.is_some() && Instant::now() >= deadline {
                break;
            }
            let one_third_space_score =
                remaining_space_score(cells.as_slice(), grass, &one_third_occupied);
            let two_thirds_space_score =
                remaining_space_score(cells.as_slice(), grass, &two_thirds_occupied);
            let average_space_score =
                (current_space_score + one_third_space_score + two_thirds_space_score) / 3.0;
            let evaluation =
                evaluate_region_with_space_score(cheap_evaluation, average_space_score, context);
            if projected_best_candidate
                .as_ref()
                .is_none_or(|(best_evaluation, _, _, _)| evaluation > *best_evaluation)
            {
                projected_best_candidate = Some((evaluation, boundary, usage_fee, cells));
            }
        }
        if projected_best_candidate.is_some() {
            best_candidate = projected_best_candidate;
        }
    }

    let secondary_evaluation_timed_out = best_candidate.is_some() && Instant::now() >= deadline;
    best_candidate.map(
        |(evaluation, boundary, usage_fee, cells)| RegionSearchResult {
            evaluation,
            cells,
            boundary,
            usage_fee,
            secondary_evaluation_timed_out,
        },
    )
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
    usage_fee: i64,
    group_size: usize,
    duration_ticks: i64,
    efficiency_threshold: f64,
) -> bool {
    let efficiency = usage_fee as f64 / (group_size as f64 * duration_ticks as f64);
    efficiency < efficiency_threshold
}

fn main() {
    input! {
        _n: usize,
        _m: usize,
        r: String,
        rows: [Chars; N],
    }
    let mut grass = BitBoard::empty();
    for (x, row) in rows.into_iter().enumerate() {
        for (y, cell) in row.into_iter().enumerate() {
            if cell == '.' {
                grass.insert(x, y);
            }
        }
    }
    let grass_area = grass.count();
    let early_efficiency_threshold = calculate_early_efficiency_threshold(&grass);
    let mut occupied = BitBoard::empty();
    let mut occupied_until_ticks = vec![vec![0_i64; N]; N];
    let mut regions: Vec<Vec<(usize, usize)>> = vec![Vec::new(); M];
    let mut departure_time_ticks = vec![0_i64; M];
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
            arrival_time_ticks: i64,
            departure_time_ticks_for_group: i64,
            p: usize,
            v: i64,
        }

        for j in 0..i {
            if active[j] && departure_time_ticks[j] < arrival_time_ticks {
                for &(x, y) in &regions[j] {
                    occupied.remove(x, y);
                    occupied_until_ticks[x][y] = 0;
                }
                active[j] = false;
            }
        }

        group_ids[i] = group_id;
        group_sizes[i] = p;
        values[i] = v;
        departure_time_ticks[i] = departure_time_ticks_for_group;

        let duration_ticks = departure_time_ticks_for_group - arrival_time_ticks;
        let remaining_group_count = M - i - 1;
        let remaining_time_ticks = TIME_HORIZON_TICKS - arrival_time_ticks;
        let estimated_arrival_rate_per_tick =
            remaining_group_count as f64 / remaining_time_ticks as f64;
        let future_value_weight = calculate_future_value_weight(remaining_group_count);
        let group_size_multiplier = group_size_efficiency_threshold_multiplier(p);
        let fill_gate = calculate_fill_gate(occupied.count(), p, grass_area);
        let efficiency_threshold_endgame_multiplier =
            calculate_efficiency_threshold_endgame_multiplier(remaining_group_count);
        let efficiency_threshold = early_efficiency_threshold
            * group_size_multiplier
            * fill_gate
            * efficiency_threshold_endgame_multiplier;
        let placement_context = PlacementEvaluationContext {
            value: v,
            duration_ticks,
            departure_time_ticks: departure_time_ticks_for_group,
            estimated_arrival_rate_per_tick,
            future_value_weight,
        };

        let started_at = Instant::now();
        let search_deadline = started_at + Duration::from_micros(SEARCH_TIME_LIMIT_US);
        let hard_deadline = started_at + Duration::from_micros(HARD_TIME_LIMIT_US);
        let available_components =
            calculate_available_component_sizes(&grass, &occupied, search_deadline);
        let normal_region = available_components.as_ref().and_then(|components| {
            find_best_region(
                &grass,
                components,
                &occupied,
                &occupied_until_ticks,
                p,
                &placement_context,
                search_deadline,
            )
        });
        let reject_normal_region = normal_region.as_ref().is_some_and(|result| {
            should_reject_by_efficiency(result.usage_fee, p, duration_ticks, efficiency_threshold)
        });

        if reject_normal_region {
            writeln!(out, "0").unwrap();
            writeln!(out, "No").unwrap();
        } else if let Some(result) = normal_region
            && (Instant::now() < hard_deadline
                || (result.secondary_evaluation_timed_out && result.evaluation > 0.0))
        {
            let cells = result.cells;
            writeln!(out, "0").unwrap();
            writeln!(out, "Yes").unwrap();
            for &(x, y) in &cells {
                writeln!(out, "{x} {y}").unwrap();
                occupied.insert(x, y);
                occupied_until_ticks[x][y] = departure_time_ticks_for_group;
            }
            regions[i] = cells;
            maximum_boundaries[i] = result.boundary;
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
                        occupied.remove(x, y);
                        occupied_until_ticks[x][y] = 0;
                    }

                    let arriving_region =
                        calculate_available_component_sizes(&grass, &occupied, search_deadline)
                            .and_then(|components| {
                                find_best_region(
                                    &grass,
                                    &components,
                                    &occupied,
                                    &occupied_until_ticks,
                                    p,
                                    &placement_context,
                                    search_deadline,
                                )
                            })
                            .map(|result| (result.cells, result.boundary, result.usage_fee));

                    let mut candidate = None;
                    if let Some((arriving_cells, arriving_boundary, arriving_usage_fee)) =
                        arriving_region
                        && Instant::now() < hard_deadline
                    {
                        let reject_arriving_region = should_reject_by_efficiency(
                            arriving_usage_fee,
                            p,
                            duration_ticks,
                            efficiency_threshold,
                        );

                        if !reject_arriving_region {
                            for &(x, y) in &arriving_cells {
                                occupied.insert(x, y);
                                occupied_until_ticks[x][y] = departure_time_ticks_for_group;
                            }

                            let moved_placement_context = PlacementEvaluationContext {
                                value: values[j],
                                duration_ticks: departure_time_ticks[j] - arrival_time_ticks,
                                departure_time_ticks: departure_time_ticks[j],
                                estimated_arrival_rate_per_tick,
                                future_value_weight,
                            };
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
                                    &occupied_until_ticks,
                                    group_sizes[j],
                                    &moved_placement_context,
                                    search_deadline,
                                )
                            })
                            .map(|result| (result.cells, result.boundary, result.usage_fee));

                            let mut moved_candidate = None;
                            if let Some((moved_cells, moved_boundary, moved_usage_fee)) =
                                moved_region
                                && Instant::now() < hard_deadline
                            {
                                let moved_maximum_boundary =
                                    maximum_boundaries[j].max(moved_boundary);
                                let moved_maximum_usage_fee =
                                    if moved_maximum_boundary == moved_boundary {
                                        moved_usage_fee
                                    } else {
                                        usage_fee(values[j], group_sizes[j], moved_maximum_boundary)
                                    };
                                let score_difference = arriving_usage_fee
                                    - move_cost(values[j], move_cost_rate_milli)
                                    + moved_maximum_usage_fee
                                    - usage_fee(values[j], group_sizes[j], maximum_boundaries[j]);
                                if score_difference > 0 && Instant::now() < hard_deadline {
                                    moved_candidate = Some((moved_cells, moved_maximum_boundary));
                                }
                            }

                            for &(x, y) in &arriving_cells {
                                occupied.remove(x, y);
                                occupied_until_ticks[x][y] = 0;
                            }
                            if let Some((moved_cells, moved_maximum_boundary)) = moved_candidate {
                                candidate = Some((
                                    arriving_cells,
                                    arriving_boundary,
                                    moved_cells,
                                    moved_maximum_boundary,
                                ));
                            }
                        }
                    }

                    if Instant::now() >= hard_deadline {
                        candidate = None;
                    }

                    if let Some((
                        arriving_cells,
                        arriving_boundary,
                        moved_cells,
                        moved_maximum_boundary,
                    )) = candidate
                    {
                        for &(x, y) in &arriving_cells {
                            occupied.insert(x, y);
                            occupied_until_ticks[x][y] = departure_time_ticks_for_group;
                        }
                        for &(x, y) in &moved_cells {
                            occupied.insert(x, y);
                            occupied_until_ticks[x][y] = departure_time_ticks[j];
                        }
                        accepted_move = Some((
                            j,
                            arriving_cells,
                            arriving_boundary,
                            moved_cells,
                            moved_maximum_boundary,
                        ));
                        break;
                    }

                    for &(x, y) in &regions[j] {
                        occupied.insert(x, y);
                        occupied_until_ticks[x][y] = departure_time_ticks[j];
                    }
                }
            }

            if let Some((
                j,
                arriving_cells,
                arriving_boundary,
                moved_cells,
                moved_maximum_boundary,
            )) = accepted_move
            {
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
                maximum_boundaries[i] = arriving_boundary;
                active[i] = true;
            } else {
                writeln!(out, "0").unwrap();
                writeln!(out, "No").unwrap();
            }
        }
        out.flush().unwrap();
    }
}
