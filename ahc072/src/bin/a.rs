use proconio::input;
use proconio::marker::Bytes;
use std::collections::{HashSet, VecDeque};
use std::fmt::Write;
use std::time::{Duration, Instant};

const DIRECTIONS: [(isize, isize, char); 4] =
    [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const MAX_OPERATIONS: usize = 100_000;
const MAX_HEIGHT: usize = 8;
const MAX_PAIR_STEPS: usize = 3;
const MAX_REPLANS: usize = 512;
const MAX_TARGET_CANDIDATES: usize = 4;
const TARGET_SAMPLE_SEED: u64 = 0x0A72_2026_0930_0009;
const MAX_CANDIDATE_ROLLOUTS: usize = 3;
const MAX_MERGE_PARTNERS: usize = 16;
const MAX_MIXED_REPLANS: usize = 64;
const MIXED_REPLAN_INTERVAL: usize = 4;
const MAX_MIXED_PARTNERS: usize = 4;
const MAX_MIXED_TRANSPORT_STEPS: usize = 24;
const MAX_PICKUP_REPLANS: usize = 32;
const PICKUP_REPLAN_INTERVAL: usize = 8;
const MAX_PICKUP_PARTNERS: usize = 2;
const MAX_PICKUP_THIRDS: usize = 3;
const MAX_PICKUP_APPROACH_STEPS: usize = 12;
const MAX_PICKUP_FINISH_STEPS: usize = 24;
const MAX_PICKUP_STATES: usize = 4000;
const MAX_PICKUP_ROLLOUTS: usize = 8;
const SEARCH_DEADLINE: Duration = Duration::from_millis(1650);

#[derive(Clone, Copy, Debug, Default)]
struct Stack {
    colors: [u8; MAX_HEIGHT], // Bottom to top in colors[..len].
    len: usize,
}

impl PartialEq for Stack {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.colors[..self.len] == other.colors[..other.len]
    }
}

impl Eq for Stack {}

impl Stack {
    fn len(&self) -> usize {
        self.len
    }

    fn last(&self) -> Option<u8> {
        (self.len > 0).then(|| self.colors[self.len - 1])
    }

    fn push(&mut self, color: u8) {
        assert!(self.len < MAX_HEIGHT);
        self.colors[self.len] = color;
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.colors[self.len])
    }

    fn top_run_len(&self) -> usize {
        let Some(color) = self.last() else {
            return 0;
        };
        let mut count = 0;
        while count < self.len && self.colors[self.len - 1 - count] == color {
            count += 1;
        }
        count
    }
}

#[derive(Clone)]
struct Board {
    n: usize,
    walls: Vec<bool>,
    nests: Vec<Option<u8>>,
    stacks: Vec<Stack>,
}

impl Board {
    fn adjacent(&self, cell: usize, direction: usize) -> Option<usize> {
        let (di, dj, _) = DIRECTIONS[direction];
        let i = (cell / self.n).checked_add_signed(di)?;
        let j = (cell % self.n).checked_add_signed(dj)?;
        if i >= self.n || j >= self.n {
            return None;
        }
        let next = i * self.n + j;
        (!self.walls[next]).then_some(next)
    }

    fn return_home(&mut self, cell: usize) {
        if let Some(color) = self.nests[cell] {
            while self.stacks[cell].last() == Some(color) {
                self.stacks[cell].pop();
            }
        }
    }

    fn apply(&mut self, action: Action) {
        let height = self.stacks[action.from].len();
        assert!(action.k < height);
        assert!((1..=action.k + 1).contains(&action.length));

        let mut to = action.from;
        for _ in 0..action.length {
            to = self.adjacent(to, action.direction).unwrap();
        }
        let jumping_count = height - action.k;
        assert!(self.stacks[to].len() + jumping_count <= MAX_HEIGHT);

        let mut jumping = [0; MAX_HEIGHT];
        for (index, slot) in jumping.iter_mut().take(jumping_count).enumerate() {
            *slot = self.stacks[action.from].colors[height - 1 - index];
        }
        self.stacks[action.from].len = action.k;
        for &color in jumping.iter().take(jumping_count) {
            self.stacks[to].push(color);
        }
        self.return_home(action.from);
        self.return_home(to);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Action {
    from: usize,
    k: usize,
    direction: usize,
    length: usize,
}

type Plan = VecDeque<Vec<Action>>;

fn plan_length(plan: &Plan) -> usize {
    plan.iter().map(Vec::len).sum()
}

struct RelayPlan {
    actions: [Action; 2],
    saving: usize,
}

type MovePriority = (bool, usize, usize, usize, usize);
type MoveCandidate = (MovePriority, Action, usize);

fn distances_from(board: &Board, start: usize) -> Vec<usize> {
    let mut distances = vec![usize::MAX; board.n * board.n];
    let mut queue = VecDeque::new();
    distances[start] = 0;
    queue.push_back(start);

    while let Some(cell) = queue.pop_front() {
        for direction in 0..DIRECTIONS.len() {
            let Some(next) = board.adjacent(cell, direction) else {
                continue;
            };
            if distances[next] == usize::MAX {
                distances[next] = distances[cell] + 1;
                queue.push_back(next);
            }
        }
    }
    distances
}

fn choose_target(board: &Board, distances: &[Vec<usize>]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, usize, usize)> = None;
    for (cell, stack) in board.stacks.iter().enumerate() {
        if let Some(color) = stack.last() {
            let color = usize::from(color);
            let distance = distances[color][cell];
            // Clear full and mixed stacks before selecting the farthest slime freely.
            // With every stack homogeneous and shorter than MAX_HEIGHT, any top
            // slime can jump to a neighboring cell that is closer to its nest.
            let cleanup_priority = if stack.len() == MAX_HEIGHT {
                2
            } else if stack.top_run_len() != stack.len() {
                1
            } else {
                0
            };
            if best
                .as_ref()
                .is_none_or(|(_, _, best_priority, best_distance)| {
                    (cleanup_priority, distance) > (*best_priority, *best_distance)
                })
            {
                best = Some((cell, color, cleanup_priority, distance));
            }
        }
    }
    best.map(|(cell, color, _, _)| (cell, color))
}

// SplitMix64 with a fixed seed. Only target candidate extraction consumes it.
struct TargetRng {
    state: u64,
}

impl TargetRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, size: usize) -> usize {
        let modulus = size as u64;
        let threshold = modulus.wrapping_neg() % modulus;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return (value % modulus) as usize;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetSlot {
    Fourth,
    OtherColor,
    Overall,
    Fill,
}

impl TargetSlot {
    fn index(self) -> usize {
        match self {
            Self::Fourth => 0,
            Self::OtherColor => 1,
            Self::Overall => 2,
            Self::Fill => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TargetCandidate {
    cell: usize,
    color: usize,
    rank: usize,
    slot: TargetSlot,
}

struct TargetChoices {
    first: (usize, usize),
    additional: Vec<TargetCandidate>,
}

#[derive(Default)]
struct TargetCounts {
    rank: [usize; 3],  // 2-4, 5-8, 9+
    color: [usize; 2], // Same as first, different from first
    slot: [usize; 4],  // Fourth, other color, overall, fill
}

impl TargetCounts {
    fn record(&mut self, candidate: TargetCandidate, first_color: usize) {
        let rank_bucket = if candidate.rank <= 4 {
            0
        } else if candidate.rank <= 8 {
            1
        } else {
            2
        };
        self.rank[rank_bucket] += 1;
        self.color[usize::from(candidate.color != first_color)] += 1;
        self.slot[candidate.slot.index()] += 1;
    }
}

fn sample_target_index(
    groups: &[(usize, usize)],
    chosen: &[(usize, TargetSlot)],
    different_color: bool,
    rng: &mut TargetRng,
) -> Option<usize> {
    let available: Vec<_> = (1..groups.len())
        .filter(|&index| {
            !chosen.iter().any(|&(selected, _)| selected == index)
                && (!different_color || groups[index].1 != groups[0].1)
        })
        .collect();
    (!available.is_empty()).then(|| available[rng.index(available.len())])
}

// Return only ordinary homogeneous groups. A full or mixed stack must keep
// the legacy cleanup priority, so it suppresses target comparison entirely.
fn ordinary_target_candidates(
    board: &Board,
    distances: &[Vec<usize>],
    rng: &mut TargetRng,
) -> Option<TargetChoices> {
    let mut candidates = Vec::new();
    for (cell, stack) in board.stacks.iter().enumerate() {
        let Some(color) = stack.last() else {
            continue;
        };
        if stack.len() == MAX_HEIGHT || stack.top_run_len() != stack.len() {
            return None;
        }
        candidates.push((
            distances[usize::from(color)][cell],
            cell,
            usize::from(color),
        ));
    }
    candidates.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let groups: Vec<_> = candidates
        .into_iter()
        .map(|(_, cell, color)| (cell, color))
        .collect();
    let &first = groups.first()?;
    let mut chosen = Vec::new();
    if groups.len() >= MAX_TARGET_CANDIDATES {
        chosen.push((MAX_TARGET_CANDIDATES - 1, TargetSlot::Fourth));
    }
    if let Some(index) = sample_target_index(&groups, &chosen, true, rng) {
        chosen.push((index, TargetSlot::OtherColor));
    }
    if chosen.len() < MAX_TARGET_CANDIDATES - 1
        && let Some(index) = sample_target_index(&groups, &chosen, false, rng)
    {
        chosen.push((index, TargetSlot::Overall));
    }
    while chosen.len() < MAX_TARGET_CANDIDATES - 1 {
        let Some(index) = sample_target_index(&groups, &chosen, false, rng) else {
            break;
        };
        chosen.push((index, TargetSlot::Fill));
    }
    chosen.sort_unstable_by_key(|&(index, _)| index);
    Some(TargetChoices {
        first,
        additional: chosen
            .into_iter()
            .map(|(index, slot)| TargetCandidate {
                cell: groups[index].0,
                color: groups[index].1,
                rank: index + 1,
                slot,
            })
            .collect(),
    })
}

fn springboard_gain(board: &Board, cell: usize, moving: usize, distances: &[usize]) -> usize {
    let base_height = board.stacks[cell].len();
    let current_distance = distances[cell];
    let mut best_gain = 0;
    for direction in 0..DIRECTIONS.len() {
        let mut to = cell;
        for _ in 1..=base_height + 1 {
            let Some(next) = board.adjacent(to, direction) else {
                break;
            };
            to = next;
            if board.stacks[to].len() + moving <= MAX_HEIGHT {
                best_gain = best_gain.max(current_distance.saturating_sub(distances[to]));
            }
        }
    }
    best_gain.saturating_sub(1)
}

fn choose_move(
    board: &Board,
    cell: usize,
    color: usize,
    moving: usize,
    distances: &[usize],
) -> Option<(Action, usize)> {
    let height = board.stacks[cell].len();
    let k = height - moving;
    let current_distance = distances[cell];
    let mut best: Option<MoveCandidate> = None;

    for direction in 0..DIRECTIONS.len() {
        let mut to = cell;
        for length in 1..=k + 1 {
            let Some(next) = board.adjacent(to, direction) else {
                break;
            };
            to = next;
            let new_distance = distances[to];
            if new_distance >= current_distance || board.stacks[to].len() + moving > MAX_HEIGHT {
                continue;
            }
            let collected = if board.stacks[to].last() == Some(color as u8) {
                board.stacks[to].top_run_len()
            } else {
                0
            };
            let relay_gain = if board.stacks[to].len() > 0 && collected == 0 {
                springboard_gain(board, to, moving, distances)
            } else {
                0
            };
            let priority = (
                collected > 0,
                current_distance - new_distance,
                relay_gain,
                collected,
                length,
            );
            if best
                .as_ref()
                .is_none_or(|(best_priority, _, _)| priority > *best_priority)
            {
                best = Some((
                    priority,
                    Action {
                        from: cell,
                        k,
                        direction,
                        length,
                    },
                    to,
                ));
            }
        }
    }
    best.map(|(_, action, to)| (action, to))
}

fn choose_group_move(
    board: &Board,
    cell: usize,
    color: usize,
    distances: &[usize],
) -> Option<(Action, usize)> {
    // Prefer moving the whole top run, then the largest legal portion of it.
    // Keep the existing destination ranking for each possible group size.
    (1..=board.stacks[cell].top_run_len())
        .rev()
        .find_map(|moving| choose_move(board, cell, color, moving, distances))
}

fn choose_relay(
    board: &Board,
    cell: usize,
    color: usize,
    distances: &[usize],
) -> Option<RelayPlan> {
    let moving = board.stacks[cell].top_run_len();
    let (ordinary, ordinary_to) = choose_group_move(board, cell, color, distances)?;
    // Preserve the existing preference for collecting and transporting groups.
    if board.stacks[ordinary_to].last() == Some(color as u8)
        || board.nests[ordinary_to] == Some(color as u8)
    {
        return None;
    }
    let mut after_first = board.clone();
    after_first.apply(ordinary);
    let (_, ordinary_end) = choose_group_move(&after_first, ordinary_to, color, distances)?;
    if after_first.stacks[ordinary_end].last() == Some(color as u8) {
        return None;
    }
    // Compare two operations plus remaining walking distance against the
    // ordinary two moves. Also require a strict saving over walking from here;
    // every completed relay then decreases the moving group's nest distance.
    let cost_limit = distances[cell].min(2 + distances[ordinary_end]);
    let k = board.stacks[cell].len() - moving;
    let mut best: Option<RelayPlan> = None;
    for direction in 0..DIRECTIONS.len() {
        let mut via = cell;
        for length in 1..=k + 1 {
            let Some(next) = board.adjacent(via, direction) else {
                break;
            };
            via = next;
            let base = &board.stacks[via];
            if base.len() == 0
                || base.len() != base.top_run_len()
                || base.last() == Some(color as u8)
                || base.len() + moving > MAX_HEIGHT
                || board.nests[via] == Some(color as u8)
            {
                continue;
            }
            for exit_direction in 0..DIRECTIONS.len() {
                let mut to = via;
                for exit_length in 1..=base.len() + 1 {
                    let Some(next) = board.adjacent(to, exit_direction) else {
                        break;
                    };
                    to = next;
                    // Finish on an empty cell or at home, so the group does
                    // not remain on a mixed tower when target selection resumes.
                    if to == cell
                        || (board.stacks[to].len() > 0 && board.nests[to] != Some(color as u8))
                        || board.stacks[to].len() + moving > MAX_HEIGHT
                    {
                        continue;
                    }
                    let cost = 2 + distances[to];
                    if cost >= cost_limit {
                        continue;
                    }
                    let saving = cost_limit - cost;
                    if best.as_ref().is_none_or(|plan| saving > plan.saving) {
                        best = Some(RelayPlan {
                            actions: [
                                Action {
                                    from: cell,
                                    k,
                                    direction,
                                    length,
                                },
                                Action {
                                    from: via,
                                    k: base.len(),
                                    direction: exit_direction,
                                    length: exit_length,
                                },
                            ],
                            saving,
                        });
                    }
                }
            }
        }
    }
    best
}

// Estimate transport operations while keeping other towers fixed. The moving
// group is absent from `removed`, including both original cells after pairing.
// This deliberately does not predict later collections or tower movements.
fn transport_cost(
    board: &Board,
    start: usize,
    color: usize,
    moving: usize,
    removed: &[usize],
) -> Option<usize> {
    let height = |cell| {
        if removed.contains(&cell) {
            0
        } else {
            board.stacks[cell].len()
        }
    };
    let mut costs = vec![usize::MAX; board.stacks.len()];
    let mut queue = VecDeque::from([start]);
    costs[start] = 0;
    while let Some(cell) = queue.pop_front() {
        if board.nests[cell] == Some(color as u8) {
            return Some(costs[cell]);
        }
        for direction in 0..DIRECTIONS.len() {
            let mut to = cell;
            for _ in 0..=height(cell) {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                if height(to) + moving <= MAX_HEIGHT && costs[to] == usize::MAX {
                    costs[to] = costs[cell] + 1;
                    queue.push_back(to);
                }
            }
        }
    }
    None
}

fn lost_singleton_support(board: &Board, cell: usize, distances: &[Vec<usize>]) -> usize {
    let mut loss = 0;
    for direction in 0..DIRECTIONS.len() {
        let Some(user) = board.adjacent(cell, direction) else {
            continue;
        };
        let stack = &board.stacks[user];
        let Some(color) = stack.last() else {
            continue;
        };
        let moving = stack.top_run_len();
        if Some(color) == board.stacks[cell].last()
            || board.nests[cell] == Some(color)
            || moving + 1 > MAX_HEIGHT
        {
            continue;
        }
        let distance = &distances[usize::from(color)];
        // One operation of potential saving per neighboring group that could
        // enter this singleton and then jump two cells toward its own nest.
        // No credit is given for the new pair becoming a taller springboard.
        if distance[cell] < distance[user] && springboard_gain(board, cell, moving, distance) > 0 {
            loss += 1;
        }
    }
    loss
}

fn choose_pair(
    board: &Board,
    cell: usize,
    color: usize,
    distances: &[Vec<usize>],
) -> Option<Vec<Action>> {
    if board.stacks[cell].len() != 1 {
        return None;
    }
    let (_, ordinary_to) = choose_move(board, cell, color, 1, &distances[color])?;
    // Keep immediate collection and returning home as in the original solver.
    if board.stacks[ordinary_to].last() == Some(color as u8)
        || board.nests[ordinary_to] == Some(color as u8)
    {
        return None;
    }
    let mut steps = vec![usize::MAX; board.stacks.len()];
    let mut previous = vec![None; board.stacks.len()];
    let mut queue = VecDeque::from([cell]);
    let mut candidates = Vec::new();
    steps[cell] = 0;
    while let Some(from) = queue.pop_front() {
        if steps[from] == MAX_PAIR_STEPS {
            continue;
        }
        for direction in 0..DIRECTIONS.len() {
            let Some(to) = board.adjacent(from, direction) else {
                continue;
            };
            if steps[to] != usize::MAX || board.nests[to] == Some(color as u8) {
                continue;
            }
            let target = &board.stacks[to];
            let is_partner = target.len() == 1 && target.last() == Some(color as u8);
            if target.len() > 0 && !is_partner {
                continue;
            }
            steps[to] = steps[from] + 1;
            previous[to] = Some(Action {
                from,
                k: 0,
                direction,
                length: 1,
            });
            if is_partner {
                candidates.push(to);
            } else {
                queue.push_back(to);
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let source_cost = transport_cost(board, cell, color, 1, &[cell])?;
    let support_loss = lost_singleton_support(board, cell, distances);
    let mut best = None;
    for partner in candidates {
        let Some(partner_cost) = transport_cost(board, partner, color, 1, &[partner]) else {
            continue;
        };
        let Some(pair_cost) = transport_cost(board, partner, color, 2, &[cell, partner]) else {
            continue;
        };
        let separate_cost = source_cost + partner_cost;
        let merged_cost = steps[partner] + pair_cost + support_loss;
        if merged_cost >= separate_cost {
            continue;
        }
        let priority = (separate_cost - merged_cost, MAX_PAIR_STEPS - steps[partner]);
        if best
            .as_ref()
            .is_none_or(|&(best_priority, _)| priority > best_priority)
        {
            best = Some((priority, partner));
        }
    }
    let (_, mut to) = best?;
    let mut actions = Vec::new();
    while to != cell {
        let action = previous[to]?;
        actions.push(action);
        to = action.from;
    }
    actions.reverse();
    Some(actions)
}

// A legacy decision is one unit: a short pair, a relay, or one ordinary move.
// Budget checks belong to the caller and never change this choice.
fn legacy_unit(
    board: &Board,
    cell: usize,
    color: usize,
    distances: &[Vec<usize>],
) -> Option<Vec<Action>> {
    if let Some(pair) = choose_pair(board, cell, color, distances) {
        Some(pair)
    } else if let Some(relay) = choose_relay(board, cell, color, &distances[color]) {
        Some(relay.actions.to_vec())
    } else {
        Some(vec![
            choose_group_move(board, cell, color, &distances[color])?.0,
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RolloutStop {
    Incumbent,
    Deadline,
    OperationLimit,
    NoMove,
}

#[derive(Clone, Copy, Default)]
struct RolloutStats {
    attempts: usize,
    completed: usize,
    pruned: usize,
    timed_out: usize,
    operation_limit: usize,
    no_move: usize,
}

impl RolloutStats {
    fn record(&mut self, result: &Result<Plan, RolloutStop>) {
        self.attempts += 1;
        match result {
            Ok(_) => self.completed += 1,
            Err(RolloutStop::Incumbent) => self.pruned += 1,
            Err(RolloutStop::Deadline) => self.timed_out += 1,
            Err(RolloutStop::OperationLimit) => self.operation_limit += 1,
            Err(RolloutStop::NoMove) => self.no_move += 1,
        }
    }

    fn log(&self, kind: &str) {
        eprintln!(
            "rollout_pruning kind={kind} attempts={} completed={} pruned={} timed_out={} operation_limit={} no_move={}",
            self.attempts,
            self.completed,
            self.pruned,
            self.timed_out,
            self.operation_limit,
            self.no_move,
        );
    }
}

// Units are kept intact when a saved continuation is resumed.
// The problem limit is inclusive; the incumbent bound is exclusive.
fn legacy_rollout(
    initial: &Board,
    distances: &[Vec<usize>],
    limit: usize,
    deadline: Option<Instant>,
    incumbent_bound: Option<usize>,
) -> Result<Plan, RolloutStop> {
    if incumbent_bound == Some(0) {
        return Err(RolloutStop::Incumbent);
    }
    let mut board = initial.clone();
    let mut plan = Plan::new();
    let mut count = 0;
    while let Some((cell, color)) = choose_target(&board, distances) {
        if count >= limit {
            return Err(RolloutStop::OperationLimit);
        }
        if deadline.is_some_and(|time| Instant::now() >= time) {
            return Err(RolloutStop::Deadline);
        }
        let unit = legacy_unit(&board, cell, color, distances).ok_or(RolloutStop::NoMove)?;
        let next_count = count + unit.len();
        if incumbent_bound.is_some_and(|bound| next_count >= bound) {
            return Err(RolloutStop::Incumbent);
        }
        if next_count > limit {
            return Err(RolloutStop::OperationLimit);
        }
        for &action in &unit {
            board.apply(action);
        }
        count = next_count;
        plan.push_back(unit);
    }
    Ok(plan)
}

// Re-read the current saved length at every call site. No partial plan escapes.
fn candidate_rollout(
    initial: &Board,
    distances: &[Vec<usize>],
    prefix: &[Action],
    remaining: usize,
    saved_length: usize,
    deadline: Option<Instant>,
    stats: &mut RolloutStats,
) -> Result<Plan, RolloutStop> {
    let result = if prefix.len() >= saved_length {
        Err(RolloutStop::Incumbent)
    } else if prefix.len() > remaining {
        Err(RolloutStop::OperationLimit)
    } else {
        let mut after = initial.clone();
        for &action in prefix {
            after.apply(action);
        }
        legacy_rollout(
            &after,
            distances,
            remaining - prefix.len(),
            deadline,
            Some(saved_length - prefix.len()),
        )
    };
    stats.record(&result);
    result
}

// The source group is removed from the fixed background. Only empty cells and
// homogeneous towers are visited. A homogeneous tower cannot lose residents
// through automatic homecoming: such a tower cannot exist on its own nest.
fn merge_candidates(board: &Board, source: usize, color: usize) -> Vec<Vec<Action>> {
    let q = board.stacks[source].len();
    if q == 0 || q == MAX_HEIGHT || board.stacks[source].top_run_len() != q {
        return Vec::new();
    }
    let mut steps = vec![usize::MAX; board.stacks.len()];
    let mut previous = vec![None; board.stacks.len()];
    let mut queue = VecDeque::from([source]);
    let mut partners = Vec::new();
    steps[source] = 0;
    while let Some(from) = queue.pop_front() {
        let support = if from == source {
            0
        } else {
            board.stacks[from].len()
        };
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                if steps[to] != usize::MAX || board.nests[to] == Some(color as u8) {
                    continue;
                }
                let stack = &board.stacks[to];
                if stack.len() + q > MAX_HEIGHT || stack.top_run_len() != stack.len() {
                    continue;
                }
                steps[to] = steps[from] + 1;
                previous[to] = Some(Action {
                    from,
                    k: support,
                    direction,
                    length,
                });
                if stack.len() > 0 && stack.last() == Some(color as u8) {
                    partners.push(to);
                } else {
                    queue.push_back(to);
                }
            }
        }
    }

    let source_cost = transport_cost(board, source, color, q, &[source]);
    let mut ranked = Vec::new();
    for partner in partners.into_iter().take(MAX_MERGE_PARTNERS) {
        let p = board.stacks[partner].len();
        let separate = source_cost.zip(transport_cost(board, partner, color, p, &[partner]));
        let merged = transport_cost(board, partner, color, q + p, &[source, partner]);
        let estimate = match (separate, merged) {
            (Some((a, b)), Some(c)) => a as i64 + b as i64 - steps[partner] as i64 - c as i64,
            _ => i64::MIN / 2,
        };
        let mut path = Vec::new();
        let mut at = partner;
        while at != source {
            let action = previous[at].expect("BFS predecessor");
            path.push(action);
            at = action.from;
        }
        path.reverse();
        // Simulate the entire candidate, including the game's color reversal
        // and automatic homecoming, before admitting it to rollout.
        let mut actual = board.clone();
        let mut valid = true;
        for (index, &action) in path.iter().enumerate() {
            if actual.stacks[action.from].len() != action.k + q
                || actual.stacks[action.from].top_run_len() < q
            {
                valid = false;
                break;
            }
            actual.apply(action);
            let next = if index + 1 == path.len() {
                partner
            } else {
                path[index + 1].from
            };
            let expected = if next == partner {
                q + p
            } else {
                q + board.stacks[next].len()
            };
            if actual.stacks[next].len() != expected || actual.stacks[next].top_run_len() < q {
                valid = false;
                break;
            }
        }
        if valid && actual.stacks[partner].len() == q + p {
            ranked.push((estimate, partner, path));
        }
    }
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.2.len().cmp(&b.2.len()))
            .then_with(|| a.1.cmp(&b.1))
    });
    ranked.into_iter().map(|(_, _, path)| path).collect()
}

// Move the selected homogeneous group without changing any background tower.
// Other homogeneous colors can serve as temporary springboards and as partners.
fn mixed_merge_paths(
    board: &Board,
    source: usize,
    color: usize,
    distances: &[Vec<usize>],
) -> Vec<(usize, Vec<Action>)> {
    let q = board.stacks[source].len();
    if q == 0 || q == MAX_HEIGHT || board.stacks[source].top_run_len() != q {
        return Vec::new();
    }
    let mut steps = vec![usize::MAX; board.stacks.len()];
    let mut previous = vec![None; board.stacks.len()];
    let mut queue = VecDeque::from([source]);
    let mut partners = Vec::new();
    steps[source] = 0;
    while let Some(from) = queue.pop_front() {
        let support = if from == source {
            0
        } else {
            board.stacks[from].len()
        };
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                if steps[to] != usize::MAX || board.nests[to] == Some(color as u8) {
                    continue;
                }
                let stack = &board.stacks[to];
                if stack.len() + q > MAX_HEIGHT
                    || stack.top_run_len() != stack.len()
                    || stack.last() == Some(color as u8)
                {
                    continue;
                }
                steps[to] = steps[from] + 1;
                previous[to] = Some(Action {
                    from,
                    k: support,
                    direction,
                    length,
                });
                if stack.len() > 0 {
                    partners.push(to);
                }
                queue.push_back(to);
            }
        }
    }
    let mut ranked = Vec::new();
    for partner in partners {
        let partner_color = usize::from(board.stacks[partner].last().unwrap());
        let estimate = steps[partner]
            .saturating_add(distances[color][partner].min(distances[partner_color][partner]));
        let mut path = Vec::new();
        let mut at = partner;
        while at != source {
            let action = previous[at].expect("BFS predecessor");
            path.push(action);
            at = action.from;
        }
        path.reverse();
        ranked.push((estimate, partner, path));
    }
    ranked.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.2.len().cmp(&b.2.len()))
            .then_with(|| a.1.cmp(&b.1))
    });
    ranked
        .into_iter()
        .take(MAX_MIXED_PARTNERS)
        .map(|(_, partner, path)| (partner, path))
        .collect()
}

// The background excludes both groups. State parity records which block is
// on top; every whole-tower jump reverses it. A terminal must leave the other
// block alone on an empty cell after automatic homecoming.
fn mixed_transport_plans(
    board: &Board,
    source: usize,
    partner: usize,
    merge_path: &[Action],
) -> Vec<Vec<Action>> {
    let source_color = board.stacks[source].last().unwrap();
    let partner_color = board.stacks[partner].last().unwrap();
    let source_count = board.stacks[source].len();
    let partner_count = board.stacks[partner].len();
    let moving = source_count + partner_count;
    let mut background = board.clone();
    background.stacks[source] = Stack::default();
    background.stacks[partner] = Stack::default();
    let mut merged = board.clone();
    for &action in merge_path {
        if merged.stacks[action.from].len() != action.k + source_count
            || merged.stacks[action.from].top_run_len() < source_count
        {
            return Vec::new();
        }
        merged.apply(action);
    }
    if merged.stacks[partner].len() != moving
        || merged.stacks[partner].top_run_len() != source_count
        || merged.stacks[partner].last() != Some(source_color)
        || merged
            .stacks
            .iter()
            .enumerate()
            .any(|(cell, stack)| cell != partner && *stack != background.stacks[cell])
    {
        return Vec::new();
    }

    let start = partner * 2;
    let mut steps = vec![usize::MAX; board.stacks.len() * 2];
    let mut previous: Vec<Option<(usize, Action)>> = vec![None; steps.len()];
    let mut queue = VecDeque::from([start]);
    let mut found = [false; 2];
    let mut plans = Vec::new();
    steps[start] = 0;
    while let Some(state) = queue.pop_front() {
        if steps[state] >= MAX_MIXED_TRANSPORT_STEPS {
            continue;
        }
        let from = state / 2;
        let parity = state % 2;
        let support = background.stacks[from].len();
        let top_after_jump = if parity == 0 {
            partner_color
        } else {
            source_color
        };
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let landing_height = background.stacks[to].len();
                if landing_height + moving > MAX_HEIGHT {
                    continue;
                }
                let action = Action {
                    from,
                    k: support,
                    direction,
                    length,
                };
                if board.nests[to] == Some(top_after_jump) {
                    if landing_height != 0 {
                        continue;
                    }
                    let returned = usize::from(top_after_jump == partner_color);
                    if found[returned] {
                        continue;
                    }
                    found[returned] = true;
                    let mut transport = vec![action];
                    let mut at = state;
                    while at != start {
                        let (before, prior_action) = previous[at].expect("BFS predecessor");
                        transport.push(prior_action);
                        at = before;
                    }
                    transport.reverse();
                    let mut actual = merged.clone();
                    for &step in &transport {
                        actual.apply(step);
                    }
                    let remaining_color = if top_after_jump == source_color {
                        partner_color
                    } else {
                        source_color
                    };
                    let remaining_count = if top_after_jump == source_color {
                        partner_count
                    } else {
                        source_count
                    };
                    if actual.stacks[to].len() == remaining_count
                        && actual.stacks[to].top_run_len() == remaining_count
                        && actual.stacks[to].last() == Some(remaining_color)
                        && actual
                            .stacks
                            .iter()
                            .enumerate()
                            .all(|(cell, stack)| cell == to || *stack == background.stacks[cell])
                    {
                        let mut whole = merge_path.to_vec();
                        whole.extend(transport);
                        plans.push(whole);
                    }
                } else {
                    let next_state = to * 2 + (1 - parity);
                    if steps[next_state] == usize::MAX {
                        steps[next_state] = steps[state] + 1;
                        previous[next_state] = Some((state, action));
                        queue.push_back(next_state);
                    }
                }
            }
        }
        if found.iter().all(|&done| done) {
            break;
        }
    }
    plans
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TwoColorStack {
    // Bottom to top. Bit 0 is the source color; bit 1 is the partner color.
    bits: u16,
    len: u8,
}

impl TwoColorStack {
    fn mask(len: u8) -> u16 {
        (1u16 << len) - 1
    }

    fn new(source_count: usize, partner_count: usize) -> Self {
        Self {
            bits: Self::mask(partner_count as u8),
            len: (source_count + partner_count) as u8,
        }
    }

    fn top_bit(self) -> u16 {
        (self.bits >> (self.len - 1)) & 1
    }

    fn reversed(self) -> Self {
        let mut bits = 0;
        for index in 0..self.len {
            bits = (bits << 1) | ((self.bits >> index) & 1);
        }
        Self {
            bits,
            len: self.len,
        }
    }

    fn with_lower_group(self, color_bit: u16, count: usize) -> Self {
        let count = count as u8;
        Self {
            bits: (if color_bit == 0 { 0 } else { Self::mask(count) }) | (self.bits << count),
            len: self.len + count,
        }
    }

    fn after_home(mut self, nest: Option<u8>, source_color: u8, partner_color: u8) -> Self {
        let nest_bit = if nest == Some(source_color) {
            Some(0)
        } else if nest == Some(partner_color) {
            Some(1)
        } else {
            None
        };
        if let Some(bit) = nest_bit {
            while self.len > 0 && self.top_bit() == bit {
                self.len -= 1;
            }
            self.bits &= Self::mask(self.len);
        }
        self
    }

    fn is_single_color(self) -> bool {
        self.len == 0 || self.bits == 0 || self.bits == Self::mask(self.len)
    }

    fn matches_stack(self, stack: &Stack, source_color: u8, partner_color: u8) -> bool {
        stack.len() == usize::from(self.len)
            && (0..usize::from(self.len)).all(|index| {
                stack.colors[index]
                    == if (self.bits >> index) & 1 == 0 {
                        source_color
                    } else {
                        partner_color
                    }
            })
    }
}

fn matches_background(actual: &Board, background: &Board, moving_cell: usize) -> bool {
    actual
        .stacks
        .iter()
        .enumerate()
        .all(|(cell, stack)| cell == moving_cell || *stack == background.stacks[cell])
}

#[derive(Default)]
struct PickupApproachStats {
    requests: usize,
    initialized: usize,
    expanded: usize,
    deadline_hits: usize,
    elapsed: Duration,
}

impl PickupApproachStats {
    fn log(&self) {
        eprintln!(
            "pickup_approach_bfs requests={} initialized={} expanded={} time_us={} deadline_hits={}",
            self.requests,
            self.initialized,
            self.expanded,
            self.elapsed.as_micros(),
            self.deadline_hits,
        );
    }
}

type PickupApproach = (Vec<Action>, TwoColorStack);

#[derive(Default)]
struct PickupArrival {
    found: [bool; 2],
    approaches: Vec<PickupApproach>,
}

// One fixed background and FIFO traversal per source/partner pair.
// Third groups remain in the background, including after recording an arrival.
struct PickupApproachBfs {
    background: Board,
    source_color: u8,
    partner_color: u8,
    moving: usize,
    initial: TwoColorStack,
    start: usize,
    steps: Vec<usize>,
    previous: Vec<Option<(usize, Action)>>,
    queue: VecDeque<usize>,
    third_index: Vec<Option<usize>>,
    arrivals: Vec<PickupArrival>,
}

impl PickupApproachBfs {
    fn new(
        board: &Board,
        source: usize,
        partner: usize,
        thirds: &[usize],
        stats: &mut PickupApproachStats,
    ) -> Self {
        let started = Instant::now();
        let mut background = board.clone();
        background.stacks[source] = Stack::default();
        background.stacks[partner] = Stack::default();
        let start = partner * 2;
        let mut steps = vec![usize::MAX; board.stacks.len() * 2];
        steps[start] = 0;
        let mut third_index = vec![None; board.stacks.len()];
        for (index, &third) in thirds.iter().enumerate() {
            third_index[third] = Some(index);
        }
        let search = Self {
            background,
            source_color: board.stacks[source].last().unwrap(),
            partner_color: board.stacks[partner].last().unwrap(),
            moving: board.stacks[source].len() + board.stacks[partner].len(),
            initial: TwoColorStack::new(board.stacks[source].len(), board.stacks[partner].len()),
            start,
            previous: vec![None; steps.len()],
            steps,
            queue: VecDeque::from([start]),
            third_index,
            arrivals: thirds.iter().map(|_| PickupArrival::default()).collect(),
        };
        stats.initialized += 1;
        stats.elapsed += started.elapsed();
        search
    }

    // Pause only after every transition of a state has been processed.
    // A deadline leaves the queue intact, unlike exhausting reachable states.
    fn request(
        &mut self,
        third: usize,
        deadline: Instant,
        stats: &mut PickupApproachStats,
    ) -> Vec<PickupApproach> {
        let started = Instant::now();
        stats.requests += 1;
        let index = self.third_index[third].expect("Selected pickup group");
        while !self.arrivals[index].found.iter().all(|&done| done) && !self.queue.is_empty() {
            if Instant::now() >= deadline {
                stats.deadline_hits += 1;
                break;
            }
            let state = self.queue.pop_front().unwrap();
            if self.steps[state] >= MAX_PICKUP_APPROACH_STEPS {
                continue;
            }
            stats.expanded += 1;
            self.expand(state);
        }
        // Preserve discovery order, including when another request found these.
        let approaches = self.arrivals[index].approaches.clone();
        stats.elapsed += started.elapsed();
        approaches
    }

    fn expand(&mut self, state: usize) {
        let from = state / 2;
        let parity = state % 2;
        let moving_stack = if parity == 0 {
            self.initial
        } else {
            self.initial.reversed()
        };
        let incoming = moving_stack.reversed();
        let incoming_color = if incoming.top_bit() == 0 {
            self.source_color
        } else {
            self.partner_color
        };
        let support = self.background.stacks[from].len();
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = self.background.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let target = &self.background.stacks[to];
                if target.top_run_len() != target.len()
                    || target.len() + self.moving > MAX_HEIGHT
                    || self.background.nests[to] == Some(incoming_color)
                {
                    continue;
                }
                let action = Action {
                    from,
                    k: support,
                    direction,
                    length,
                };
                let next_parity = 1 - parity;
                // Arrival detection precedes the unvisited-state check, as before.
                if let Some(index) = self.third_index[to]
                    && !self.arrivals[index].found[next_parity]
                {
                    self.arrivals[index].found[next_parity] = true;
                    let mut path = vec![action];
                    let mut at = state;
                    while at != self.start {
                        let (before, prior) = self.previous[at].expect("BFS predecessor");
                        path.push(prior);
                        at = before;
                    }
                    path.reverse();
                    let third_bit = u16::from(target.last() == Some(self.partner_color));
                    self.arrivals[index]
                        .approaches
                        .push((path, incoming.with_lower_group(third_bit, target.len())));
                }
                let next_state = to * 2 + next_parity;
                if self.steps[next_state] == usize::MAX {
                    self.steps[next_state] = self.steps[state] + 1;
                    self.previous[next_state] = Some((state, action));
                    self.queue.push_back(next_state);
                }
            }
        }
    }
}

// Single-target convenience used only by the existing unit tests.
#[cfg(test)]
fn pickup_approaches(
    board: &Board,
    source: usize,
    partner: usize,
    third: usize,
    deadline: Instant,
) -> Vec<PickupApproach> {
    let mut stats = PickupApproachStats::default();
    PickupApproachBfs::new(board, source, partner, &[third], &mut stats)
        .request(third, deadline, &mut stats)
}

#[cfg(test)]
fn reference_pickup_approaches(
    board: &Board,
    source: usize,
    partner: usize,
    third: usize,
    deadline: Instant,
) -> Vec<(Vec<Action>, TwoColorStack)> {
    let source_color = board.stacks[source].last().unwrap();
    let partner_color = board.stacks[partner].last().unwrap();
    let moving = board.stacks[source].len() + board.stacks[partner].len();
    let initial = TwoColorStack::new(board.stacks[source].len(), board.stacks[partner].len());
    let mut background = board.clone();
    background.stacks[source] = Stack::default();
    background.stacks[partner] = Stack::default();
    let start = partner * 2;
    let mut steps = vec![usize::MAX; board.stacks.len() * 2];
    let mut previous: Vec<Option<(usize, Action)>> = vec![None; steps.len()];
    let mut queue = VecDeque::from([start]);
    let mut found = [false; 2];
    let mut approaches = Vec::new();
    steps[start] = 0;
    while let Some(state) = queue.pop_front() {
        if Instant::now() >= deadline {
            break;
        }
        if steps[state] >= MAX_PICKUP_APPROACH_STEPS {
            continue;
        }
        let from = state / 2;
        let parity = state % 2;
        let moving_stack = if parity == 0 {
            initial
        } else {
            initial.reversed()
        };
        let incoming = moving_stack.reversed();
        let incoming_color = if incoming.top_bit() == 0 {
            source_color
        } else {
            partner_color
        };
        let support = background.stacks[from].len();
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let target = &background.stacks[to];
                if target.top_run_len() != target.len()
                    || target.len() + moving > MAX_HEIGHT
                    || board.nests[to] == Some(incoming_color)
                {
                    continue;
                }
                let action = Action {
                    from,
                    k: support,
                    direction,
                    length,
                };
                let next_parity = 1 - parity;
                if to == third && !found[next_parity] {
                    found[next_parity] = true;
                    let mut path = vec![action];
                    let mut at = state;
                    while at != start {
                        let (before, prior) = previous[at].expect("BFS predecessor");
                        path.push(prior);
                        at = before;
                    }
                    path.reverse();
                    let third_bit = u16::from(target.last() == Some(partner_color));
                    approaches.push((path, incoming.with_lower_group(third_bit, target.len())));
                }
                let next_state = to * 2 + next_parity;
                if steps[next_state] == usize::MAX {
                    steps[next_state] = steps[state] + 1;
                    previous[next_state] = Some((state, action));
                    queue.push_back(next_state);
                }
            }
        }
        if found.iter().all(|&done| done) {
            break;
        }
    }
    approaches
}

struct PickupNode {
    cell: usize,
    colors: TwoColorStack,
    depth: usize,
    previous: Option<usize>,
    action: Option<Action>,
}

// Search after the third group has joined. Partial automatic homecoming is
// reflected in the packed stack; only a genuinely single-color state ends it.
fn pickup_finish_plans(
    board: &Board,
    groups: [usize; 3],
    merge_path: &[Action],
    approach: &[Action],
    picked_colors: TwoColorStack,
    deadline: Instant,
) -> Vec<Vec<Action>> {
    let [source, partner, third] = groups;
    let source_color = board.stacks[source].last().unwrap();
    let partner_color = board.stacks[partner].last().unwrap();
    let mut background = board.clone();
    for cell in [source, partner, third] {
        background.stacks[cell] = Stack::default();
    }
    let mut picked_board = board.clone();
    for &action in merge_path.iter().chain(approach) {
        picked_board.apply(action);
    }
    if !picked_colors.matches_stack(&picked_board.stacks[third], source_color, partner_color)
        || !matches_background(&picked_board, &background, third)
    {
        return Vec::new();
    }

    let mut nodes = vec![PickupNode {
        cell: third,
        colors: picked_colors,
        depth: 0,
        previous: None,
        action: None,
    }];
    let mut seen = HashSet::from([(third, picked_colors)]);
    let mut seen_terminals = HashSet::new();
    let mut plans = Vec::new();
    let mut head = 0;
    while head < nodes.len() {
        if Instant::now() >= deadline {
            break;
        }
        let state = head;
        head += 1;
        if nodes[state].depth >= MAX_PICKUP_FINISH_STEPS {
            continue;
        }
        let from = nodes[state].cell;
        let colors = nodes[state].colors;
        let support = background.stacks[from].len();
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for length in 1..=support + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let target = &background.stacks[to];
                if target.top_run_len() != target.len()
                    || target.len() + usize::from(colors.len) > MAX_HEIGHT
                {
                    continue;
                }
                let landed =
                    colors
                        .reversed()
                        .after_home(board.nests[to], source_color, partner_color);
                let action = Action {
                    from,
                    k: support,
                    direction,
                    length,
                };
                if landed.is_single_color() {
                    if landed.len > 0 && target.len() > 0 {
                        continue;
                    }
                    if !seen_terminals.insert((to, landed)) {
                        continue;
                    }
                    let mut tail = vec![action];
                    let mut at = state;
                    while let Some(previous) = nodes[at].previous {
                        tail.push(nodes[at].action.expect("BFS action"));
                        at = previous;
                    }
                    tail.reverse();
                    let mut actual = picked_board.clone();
                    for &step in &tail {
                        actual.apply(step);
                    }
                    let valid_end = if landed.len == 0 {
                        actual.stacks[to] == background.stacks[to]
                    } else {
                        landed.matches_stack(&actual.stacks[to], source_color, partner_color)
                    };
                    if valid_end && matches_background(&actual, &background, to) {
                        let mut whole = merge_path.to_vec();
                        whole.extend_from_slice(approach);
                        whole.extend(tail);
                        plans.push(whole);
                    }
                } else if nodes.len() < MAX_PICKUP_STATES && seen.insert((to, landed)) {
                    nodes.push(PickupNode {
                        cell: to,
                        colors: landed,
                        depth: nodes[state].depth + 1,
                        previous: Some(state),
                        action: Some(action),
                    });
                }
            }
        }
        if plans.len() >= 4 {
            break;
        }
    }
    plans
}

struct PickupCandidates {
    triples: usize,
    approaches: usize,
    plans: Vec<Vec<Action>>,
}

fn pickup_candidates(
    board: &Board,
    source: usize,
    color: usize,
    distances: &[Vec<usize>],
    deadline: Instant,
    stats: &mut PickupApproachStats,
) -> PickupCandidates {
    let mut result = PickupCandidates {
        triples: 0,
        approaches: 0,
        plans: Vec::new(),
    };
    for (partner, merge_path) in mixed_merge_paths(board, source, color, distances)
        .into_iter()
        .take(MAX_PICKUP_PARTNERS)
    {
        if Instant::now() >= deadline {
            break;
        }
        let partner_color = board.stacks[partner].last().unwrap();
        let total = board.stacks[source].len() + board.stacks[partner].len();
        let partner_distances = distances_from(board, partner);
        let mut thirds: Vec<_> = board
            .stacks
            .iter()
            .enumerate()
            .filter_map(|(third, stack)| {
                (third != source
                    && third != partner
                    && stack.len() > 0
                    && stack.len() == stack.top_run_len()
                    && (stack.last() == Some(color as u8) || stack.last() == Some(partner_color))
                    && total + stack.len() <= MAX_HEIGHT)
                    .then_some((partner_distances[third], third))
            })
            .collect();
        thirds.sort_unstable();
        let thirds: Vec<_> = thirds
            .into_iter()
            .take(MAX_PICKUP_THIRDS)
            .map(|(_, third)| third)
            .collect();
        let mut search = None;
        for &third in &thirds {
            if Instant::now() >= deadline {
                break;
            }
            result.triples += 1;
            let search = search.get_or_insert_with(|| {
                PickupApproachBfs::new(board, source, partner, &thirds, stats)
            });
            let approaches = search.request(third, deadline, stats);
            result.approaches += approaches.len();
            for (approach, picked_colors) in approaches {
                if Instant::now() >= deadline {
                    break;
                }
                result.plans.extend(pickup_finish_plans(
                    board,
                    [source, partner, third],
                    &merge_path,
                    &approach,
                    picked_colors,
                    deadline,
                ));
            }
        }
    }
    result.plans.sort_by_key(Vec::len);
    result
}

const MAX_EXISTING_MIXED_PICKUPS: usize = 3;
const MAX_EXISTING_MIXED_DEPTH: usize = 24;
const MAX_EXISTING_MIXED_STATES: usize = 4000;
const MAX_EXISTING_MIXED_ROLLOUTS: usize = 8;
const MAX_EXISTING_MIXED_CALLS: usize = 64;
const EXISTING_MIXED_CALL_TIME: Duration = Duration::from_millis(10);
const EXISTING_MIXED_CASE_TIME: Duration = Duration::from_millis(200);

fn existing_mixed_colors(stack: &Stack) -> Option<[u8; 2]> {
    let mut present = [false; 12];
    for &color in &stack.colors[..stack.len()] {
        present[usize::from(color)] = true;
    }
    let colors: Vec<_> = present
        .iter()
        .enumerate()
        .filter_map(|(c, &yes)| yes.then_some(c as u8))
        .collect();
    (colors.len() == 2).then(|| [colors[0], colors[1]])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ExistingMixedState {
    cell: usize,
    colors: TwoColorStack,
    collected: u8,
}

struct ExistingMixedTransition {
    state: ExistingMixedState,
    action: Action,
    split: bool,
}

struct ExistingMixedModel {
    background: Board,
    colors: [u8; 2],
    pickups: Vec<usize>,
    pickup_limit: bool,
}

impl ExistingMixedModel {
    fn new(board: &Board, source: usize, colors: [u8; 2]) -> (Self, ExistingMixedState) {
        let mut packed = TwoColorStack {
            bits: 0,
            len: board.stacks[source].len() as u8,
        };
        for (i, &color) in board.stacks[source].colors[..board.stacks[source].len()]
            .iter()
            .enumerate()
        {
            packed.bits |= u16::from(color == colors[1]) << i;
        }
        let distances = distances_from(board, source);
        let mut ranked: Vec<_> = board
            .stacks
            .iter()
            .enumerate()
            .filter_map(|(cell, stack)| {
                (cell != source
                    && stack.len() > 0
                    && stack.top_run_len() == stack.len()
                    && stack.last().is_some_and(|c| colors.contains(&c)))
                .then_some((distances[cell], cell))
            })
            .collect();
        ranked.sort_unstable();
        let pickup_limit = ranked.len() > MAX_EXISTING_MIXED_PICKUPS;
        let pickups = ranked
            .into_iter()
            .take(MAX_EXISTING_MIXED_PICKUPS)
            .map(|(_, cell)| cell)
            .collect();
        let mut background = board.clone();
        background.stacks[source] = Stack::default();
        (
            Self {
                background,
                colors,
                pickups,
                pickup_limit,
            },
            ExistingMixedState {
                cell: source,
                colors: packed,
                collected: 0,
            },
        )
    }

    fn stack(&self, cell: usize, collected: u8) -> Stack {
        if self
            .pickups
            .iter()
            .enumerate()
            .any(|(i, &p)| p == cell && collected & (1 << i) != 0)
        {
            Stack::default()
        } else {
            self.background.stacks[cell]
        }
    }

    fn terminal(&self, state: ExistingMixedState) -> bool {
        state.colors.len == 0
            || (state.colors.is_single_color()
                && self.stack(state.cell, state.collected).len() == 0)
    }

    // Pass a candidate as a springboard first; then consider collecting it.
    fn transitions(&self, state: ExistingMixedState) -> Vec<ExistingMixedTransition> {
        let mut result = Vec::new();
        if state.colors.len == 0 {
            return result;
        }
        let support = self.stack(state.cell, state.collected).len();
        let incoming = state.colors.reversed();
        for direction in 0..DIRECTIONS.len() {
            let mut to = state.cell;
            for length in 1..=support + 1 {
                let Some(next) = self.background.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let target = self.stack(to, state.collected);
                if target.top_run_len() != target.len()
                    || target.len() + usize::from(incoming.len) > MAX_HEIGHT
                {
                    continue;
                }
                let action = Action {
                    from: state.cell,
                    k: support,
                    direction,
                    length,
                };
                result.push(ExistingMixedTransition {
                    state: ExistingMixedState {
                        cell: to,
                        colors: incoming.after_home(
                            self.background.nests[to],
                            self.colors[0],
                            self.colors[1],
                        ),
                        collected: state.collected,
                    },
                    action,
                    split: false,
                });
                if !state.colors.is_single_color()
                    && let Some(index) = self.pickups.iter().position(|&cell| cell == to)
                    && state.collected & (1 << index) == 0
                {
                    let colors = incoming
                        .with_lower_group(
                            u16::from(target.last() == Some(self.colors[1])),
                            target.len(),
                        )
                        .after_home(self.background.nests[to], self.colors[0], self.colors[1]);
                    result.push(ExistingMixedTransition {
                        state: ExistingMixedState {
                            cell: to,
                            colors,
                            collected: state.collected | (1 << index),
                        },
                        action,
                        split: false,
                    });
                }
            }
        }
        // Only a nest with no background may unload its lower block.
        if support == 0
            && let Some(nest) = self.background.nests[state.cell]
            && let Some(bit) = self.colors.iter().position(|&c| c == nest)
        {
            let bottom_bit = bit as u16;
            let mut lower = 0;
            while lower < usize::from(state.colors.len)
                && (state.colors.bits >> lower) & 1 == bottom_bit
            {
                lower += 1;
            }
            let upper = usize::from(state.colors.len) - lower;
            if lower > 0
                && upper > 0
                && (lower..usize::from(state.colors.len))
                    .all(|i| (state.colors.bits >> i) & 1 != bottom_bit)
            {
                let colors = TwoColorStack {
                    bits: if bottom_bit == 0 {
                        TwoColorStack::mask(upper as u8)
                    } else {
                        0
                    },
                    len: upper as u8,
                };
                for direction in 0..DIRECTIONS.len() {
                    let mut to = state.cell;
                    for length in 1..=lower + 1 {
                        let Some(next) = self.background.adjacent(to, direction) else {
                            break;
                        };
                        to = next;
                        if self.stack(to, state.collected).len() != 0 {
                            continue;
                        }
                        result.push(ExistingMixedTransition {
                            state: ExistingMixedState {
                                cell: to,
                                colors: colors.after_home(
                                    self.background.nests[to],
                                    self.colors[0],
                                    self.colors[1],
                                ),
                                collected: state.collected,
                            },
                            action: Action {
                                from: state.cell,
                                k: lower,
                                direction,
                                length,
                            },
                            split: true,
                        });
                    }
                }
            }
        }
        result
    }

    fn board_at(&self, state: ExistingMixedState) -> Board {
        let mut board = self.background.clone();
        for (i, &cell) in self.pickups.iter().enumerate() {
            if state.collected & (1 << i) != 0 {
                board.stacks[cell] = Stack::default();
            }
        }
        for i in 0..state.colors.len {
            board.stacks[state.cell].push(self.colors[usize::from((state.colors.bits >> i) & 1)]);
        }
        board
    }
}

struct ExistingMixedNode {
    state: ExistingMixedState,
    depth: usize,
    previous: Option<(usize, Action)>,
    split: bool,
}

struct ExistingMixedPlan {
    prefix: Vec<Action>,
    collected: u8,
    end: ExistingMixedState,
    split: bool,
    returned: [usize; 2],
}

#[derive(Default)]
struct ExistingMixedSearch {
    plans: Vec<ExistingMixedPlan>,
    pickups: Vec<usize>,
    registered: usize,
    expanded: usize,
    terminals: [usize; 8],
    validated: usize,
    invalid: usize,
    duplicates: usize,
    depth_limit: bool,
    state_limit: bool,
    terminal_limit: bool,
    timed_out: bool,
    pickup_limit: bool,
    rollout_limit: bool,
}

fn checked_existing_action(board: &mut Board, action: Action) -> bool {
    let height = board.stacks[action.from].len();
    if action.k >= height || action.length == 0 || action.length > action.k + 1 {
        return false;
    }
    let mut to = action.from;
    for _ in 0..action.length {
        let Some(next) = board.adjacent(to, action.direction) else {
            return false;
        };
        to = next;
    }
    if board.stacks[to].len() + height - action.k > MAX_HEIGHT {
        return false;
    }
    board.apply(action);
    true
}

fn existing_mixed_search(
    board: &Board,
    source: usize,
    colors: [u8; 2],
    deadline: Instant,
) -> ExistingMixedSearch {
    let (model, initial) = ExistingMixedModel::new(board, source, colors);
    let mut result = ExistingMixedSearch {
        pickups: model.pickups.clone(),
        pickup_limit: model.pickup_limit,
        registered: 1,
        ..Default::default()
    };
    let mut nodes = vec![ExistingMixedNode {
        state: initial,
        depth: 0,
        previous: None,
        split: false,
    }];
    let mut seen = HashSet::from([initial]);
    let mut terminal_boards = HashSet::new();
    let mut terminals: Vec<(usize, ExistingMixedPlan)> = Vec::new();
    let mut head = 0;
    'search: while head < nodes.len() {
        if Instant::now() >= deadline {
            result.timed_out = true;
            break;
        }
        let index = head;
        head += 1;
        if model.terminal(nodes[index].state) {
            continue;
        }
        if nodes[index].depth >= MAX_EXISTING_MIXED_DEPTH {
            result.depth_limit = true;
            continue;
        }
        result.expanded += 1;
        for transition in model.transitions(nodes[index].state) {
            if seen.contains(&transition.state) {
                continue;
            }
            if nodes.len() >= MAX_EXISTING_MIXED_STATES {
                result.state_limit = true;
                continue;
            }
            seen.insert(transition.state);
            let depth = nodes[index].depth + 1;
            let state = transition.state;
            let at = nodes.len();
            nodes.push(ExistingMixedNode {
                state,
                depth,
                previous: Some((index, transition.action)),
                split: transition.split,
            });
            result.registered += 1;
            if !model.terminal(state) {
                continue;
            }
            result.terminals[usize::from(state.collected)] += 1;
            if terminals.len() >= MAX_EXISTING_MIXED_STATES {
                result.terminal_limit = true;
                continue;
            }
            let mut path = Vec::with_capacity(depth);
            let mut prior = at;
            while let Some((before, action)) = nodes[prior].previous {
                path.push(action);
                prior = before;
            }
            path.reverse();
            let mut actual = board.clone();
            for &action in &path {
                if Instant::now() >= deadline {
                    result.timed_out = true;
                    break 'search;
                }
                if !checked_existing_action(&mut actual, action) {
                    result.invalid += 1;
                    continue 'search;
                }
            }
            let expected = model.board_at(state);
            if actual.stacks != expected.stacks {
                result.invalid += 1;
                continue;
            }
            result.validated += 1;
            let key: Vec<_> = actual
                .stacks
                .iter()
                .map(|s| s.colors[..s.len()].to_vec())
                .collect();
            if !terminal_boards.insert(key) {
                result.duplicates += 1;
                continue;
            }
            let mut returned = [0; 2];
            for (i, &color) in colors.iter().enumerate() {
                let before = board
                    .stacks
                    .iter()
                    .map(|s| s.colors[..s.len()].iter().filter(|&&c| c == color).count())
                    .sum::<usize>();
                let after = actual
                    .stacks
                    .iter()
                    .map(|s| s.colors[..s.len()].iter().filter(|&&c| c == color).count())
                    .sum::<usize>();
                returned[i] = before - after;
            }
            terminals.push((
                at,
                ExistingMixedPlan {
                    prefix: path,
                    collected: state.collected,
                    end: state,
                    split: nodes[at].split,
                    returned,
                },
            ));
        }
    }
    // BFS discovery order is depth order. Keep one shortest terminal per mask.
    result.rollout_limit = terminals.len() > MAX_EXISTING_MIXED_ROLLOUTS;
    let mut selected = vec![false; terminals.len()];
    let mut represented = [false; 8];
    let mut order = Vec::new();
    for (i, (_, plan)) in terminals.iter().enumerate() {
        let mask = usize::from(plan.collected);
        if !represented[mask] && order.len() < MAX_EXISTING_MIXED_ROLLOUTS {
            represented[mask] = true;
            selected[i] = true;
            order.push(i);
        }
    }
    for (i, &yes) in selected.iter().enumerate() {
        if order.len() >= MAX_EXISTING_MIXED_ROLLOUTS {
            break;
        }
        if !yes {
            order.push(i);
        }
    }
    let mut terminals: Vec<_> = terminals.into_iter().map(|(_, plan)| Some(plan)).collect();
    result.plans = order
        .into_iter()
        .map(|i| terminals[i].take().unwrap())
        .collect();
    result
}

#[derive(Default)]
struct ExistingMixedStats {
    attempts: usize,
    not_mixed: usize,
    too_many_colors: usize,
    call_skips: usize,
    case_time_skips: usize,
    global_time_skips: usize,
    expanded: usize,
    registered: usize,
    terminals: [usize; 8],
    validated: usize,
    invalid: usize,
    duplicates: usize,
    depth_hits: usize,
    state_hits: usize,
    terminal_hits: usize,
    pickup_hits: usize,
    rollout_limit_hits: usize,
    call_time_hits: usize,
    case_time_hits: usize,
    global_time_hits: usize,
    generated: usize,
    no_candidates: usize,
    saved: usize,
    executed: usize,
    savings: usize,
    elapsed: Duration,
    rollout: RolloutStats,
    events: String,
}

impl ExistingMixedStats {
    fn record_search(&mut self, result: &ExistingMixedSearch) {
        self.expanded += result.expanded;
        self.registered += result.registered;
        for (total, count) in self.terminals.iter_mut().zip(result.terminals) {
            *total += count;
        }
        self.validated += result.validated;
        self.invalid += result.invalid;
        self.duplicates += result.duplicates;
        self.depth_hits += usize::from(result.depth_limit);
        self.state_hits += usize::from(result.state_limit);
        self.terminal_hits += usize::from(result.terminal_limit);
        self.pickup_hits += usize::from(result.pickup_limit);
        self.rollout_limit_hits += usize::from(result.rollout_limit);
        self.generated += result.plans.len();
        self.no_candidates += usize::from(result.plans.is_empty());
    }
    fn log(&self) {
        eprintln!(
            "existing_mixed attempts={} not_mixed={} too_many_colors={} call_skips={} case_time_skips={} global_time_skips={} expanded={} registered={} terminals={:?} validated={} invalid={} duplicates={} depth_hits={} state_hits={} terminal_hits={} call_time_hits={} case_time_hits={} global_time_hits={} generated={} no_candidates={} saved={} executed={} savings={} time_us={}",
            self.attempts,
            self.not_mixed,
            self.too_many_colors,
            self.call_skips,
            self.case_time_skips,
            self.global_time_skips,
            self.expanded,
            self.registered,
            self.terminals,
            self.validated,
            self.invalid,
            self.duplicates,
            self.depth_hits,
            self.state_hits,
            self.terminal_hits,
            self.call_time_hits,
            self.case_time_hits,
            self.global_time_hits,
            self.generated,
            self.no_candidates,
            self.saved,
            self.executed,
            self.savings,
            self.elapsed.as_micros()
        );
        eprintln!(
            "existing_mixed_caps pickup_hits={} rollout_limit_hits={}",
            self.pickup_hits, self.rollout_limit_hits
        );
        self.rollout.log("existing_mixed");
        eprint!("{}", self.events);
    }
}

fn main() {
    input! {
        n: usize,
        k: usize,
        rows: [Bytes; n],
    }

    let mut board = Board {
        n,
        walls: vec![false; n * n],
        nests: vec![None; n * n],
        stacks: vec![Stack::default(); n * n],
    };
    let mut nest_cells = vec![usize::MAX; k];
    for (i, row) in rows.iter().enumerate() {
        for (j, &symbol) in row.iter().enumerate() {
            let cell = i * n + j;
            match symbol {
                b'#' => board.walls[cell] = true,
                b'A'..=b'L' => {
                    let color = usize::from(symbol - b'A');
                    board.nests[cell] = Some(color as u8);
                    nest_cells[color] = cell;
                }
                b'a'..=b'l' => {
                    let color = usize::from(symbol - b'a');
                    board.stacks[cell].push(color as u8);
                }
                b'.' => {}
                _ => unreachable!("Unexpected input cell"),
            }
        }
    }

    let distances: Vec<_> = nest_cells
        .iter()
        .map(|&nest| distances_from(&board, nest))
        .collect();

    let started = Instant::now();
    let deadline = started + SEARCH_DEADLINE;
    let mut target_rng = TargetRng::new(TARGET_SAMPLE_SEED);
    let mut saved = legacy_rollout(&board, &distances, MAX_OPERATIONS, None, None)
        .expect("Legacy solver must produce a complete baseline");
    let mut actions = Vec::new();
    let mut existing_mixed_stats = ExistingMixedStats::default();
    let mut replans = 0;
    let mut candidates = 0;
    let mut rollouts = 1;
    let mut rollout_stats = [RolloutStats::default(); 5];
    let mut accepted = 0;
    let mut mixed_attempts = 0;
    let mut mixed_partners = 0;
    let mut mixed_candidates = 0;
    let mut mixed_rollouts = 0;
    let mut mixed_accepted = 0;
    let mut mixed_actions = 0;
    let mut pickup_attempts = 0;
    let mut pickup_approach_stats = PickupApproachStats::default();
    let mut pickup_triples = 0;
    let mut pickup_approaches_count = 0;
    let mut pickup_candidates_count = 0;
    let mut pickup_rollouts = 0;
    let mut pickup_saved = 0;
    let mut pickup_executed = 0;
    let mut pickup_actions = 0;
    let mut target_comparisons = 0;
    let mut target_rollouts = 0;
    let mut target_saved = 0;
    let mut target_savings = 0;
    let mut target_executed = vec![0; board.stacks.len() + 1];
    let mut target_extracted_counts = TargetCounts::default();
    let mut target_completed_counts = TargetCounts::default();
    let mut target_saved_counts = TargetCounts::default();
    let mut target_executed_counts = TargetCounts::default();
    let mut target_deadline_hits = 0;
    let mut target_time = Duration::ZERO;
    let mut target_events = Vec::new();
    let mut deadline_skipped_normal = 0;
    let mut deadline_skipped_mixed = 0;
    let mut deadline_skipped_pickup = 0;
    let mut merge_deadline_breaks = 0;
    let mut mixed_deadline_breaks = 0;
    let mut pickup_deadline_breaks = 0;
    while !saved.is_empty() {
        let mut selected_mixed_actions = None;
        let mut selected_pickup_actions = None;
        let mut selected_target_candidate = None;
        let mut selected_target_saving = 0;
        if replans < MAX_REPLANS
            && Instant::now() >= deadline
            && let Some((cell, _)) = choose_target(&board, &distances)
            && board.stacks[cell].len() < MAX_HEIGHT
            && board.stacks[cell].top_run_len() == board.stacks[cell].len()
        {
            deadline_skipped_normal += 1;
            let hypothetical_replan = replans + deadline_skipped_normal;
            if mixed_attempts < MAX_MIXED_REPLANS
                && hypothetical_replan % MIXED_REPLAN_INTERVAL == 1
            {
                deadline_skipped_mixed += 1;
            }
            if pickup_attempts < MAX_PICKUP_REPLANS
                && hypothetical_replan % PICKUP_REPLAN_INTERVAL == 1
            {
                deadline_skipped_pickup += 1;
            }
        }
        if replans < MAX_REPLANS
            && Instant::now() < deadline
            && let Some((cell, color)) = choose_target(&board, &distances)
            && board.stacks[cell].len() < MAX_HEIGHT
            && board.stacks[cell].top_run_len() == board.stacks[cell].len()
        {
            replans += 1;
            let remaining = MAX_OPERATIONS - actions.len();
            if let Ok(fresh) = candidate_rollout(
                &board,
                &distances,
                &[],
                remaining,
                plan_length(&saved),
                Some(deadline),
                &mut rollout_stats[0],
            ) {
                rollouts += 1;
                if plan_length(&fresh) < plan_length(&saved) {
                    saved = fresh;
                }
            }
            let paths = merge_candidates(&board, cell, color);
            candidates += paths.len();
            for path in paths.into_iter().take(MAX_CANDIDATE_ROLLOUTS) {
                if Instant::now() >= deadline {
                    merge_deadline_breaks += 1;
                    break;
                }
                let Ok(mut continuation) = candidate_rollout(
                    &board,
                    &distances,
                    &path,
                    remaining,
                    plan_length(&saved),
                    Some(deadline),
                    &mut rollout_stats[1],
                ) else {
                    continue;
                };
                rollouts += 1;
                if path.len() + plan_length(&continuation) < plan_length(&saved) {
                    continuation.push_front(path);
                    saved = continuation;
                    accepted += 1;
                }
            }
            if mixed_attempts < MAX_MIXED_REPLANS
                && replans % MIXED_REPLAN_INTERVAL == 1
                && Instant::now() < deadline
            {
                mixed_attempts += 1;
                let partners = mixed_merge_paths(&board, cell, color, &distances);
                mixed_partners += partners.len();
                for (partner, merge_path) in partners {
                    if Instant::now() >= deadline {
                        mixed_deadline_breaks += 1;
                        break;
                    }
                    if merge_path.len() >= plan_length(&saved) {
                        continue;
                    }
                    let plans = mixed_transport_plans(&board, cell, partner, &merge_path);
                    mixed_candidates += plans.len();
                    for prefix in plans {
                        if Instant::now() >= deadline {
                            mixed_deadline_breaks += 1;
                            break;
                        }
                        let Ok(mut continuation) = candidate_rollout(
                            &board,
                            &distances,
                            &prefix,
                            remaining,
                            plan_length(&saved),
                            Some(deadline),
                            &mut rollout_stats[2],
                        ) else {
                            continue;
                        };
                        rollouts += 1;
                        mixed_rollouts += 1;
                        if prefix.len() + plan_length(&continuation) < plan_length(&saved) {
                            selected_mixed_actions = Some(prefix.len());
                            continuation.push_front(prefix);
                            saved = continuation;
                            mixed_accepted += 1;
                        }
                    }
                }
            }
            if pickup_attempts < MAX_PICKUP_REPLANS
                && replans % PICKUP_REPLAN_INTERVAL == 1
                && Instant::now() < deadline
            {
                pickup_attempts += 1;
                let found = pickup_candidates(
                    &board,
                    cell,
                    color,
                    &distances,
                    deadline,
                    &mut pickup_approach_stats,
                );
                pickup_triples += found.triples;
                pickup_approaches_count += found.approaches;
                pickup_candidates_count += found.plans.len();
                for prefix in found.plans.into_iter().take(MAX_PICKUP_ROLLOUTS) {
                    if Instant::now() >= deadline {
                        pickup_deadline_breaks += 1;
                        break;
                    }
                    let Ok(mut continuation) = candidate_rollout(
                        &board,
                        &distances,
                        &prefix,
                        remaining,
                        plan_length(&saved),
                        Some(deadline),
                        &mut rollout_stats[3],
                    ) else {
                        continue;
                    };
                    rollouts += 1;
                    pickup_rollouts += 1;
                    if prefix.len() + plan_length(&continuation) < plan_length(&saved) {
                        selected_mixed_actions = None;
                        selected_pickup_actions = Some(prefix.len());
                        continuation.push_front(prefix);
                        saved = continuation;
                        pickup_saved += 1;
                    }
                }
            }
            // The first ranked group is exactly the ordinary legacy target.
            // Its full rollout was already evaluated above; only alternatives
            // need another rollout, after all existing search candidates.
            let target_started = Instant::now();
            if let Some(targets) = ordinary_target_candidates(&board, &distances, &mut target_rng) {
                debug_assert_eq!(targets.first, (cell, color));
                if !targets.additional.is_empty() {
                    target_comparisons += 1;
                }
                let before_target_length = plan_length(&saved);
                for &candidate in &targets.additional {
                    target_extracted_counts.record(candidate, color);
                }
                for &candidate in &targets.additional {
                    if Instant::now() >= deadline {
                        target_deadline_hits += 1;
                        break;
                    }
                    let Some(unit) =
                        legacy_unit(&board, candidate.cell, candidate.color, &distances)
                    else {
                        continue;
                    };
                    let Ok(mut continuation) = candidate_rollout(
                        &board,
                        &distances,
                        &unit,
                        remaining,
                        plan_length(&saved),
                        Some(deadline),
                        &mut rollout_stats[4],
                    ) else {
                        if Instant::now() >= deadline {
                            target_deadline_hits += 1;
                            break;
                        }
                        continue;
                    };
                    rollouts += 1;
                    target_rollouts += 1;
                    target_completed_counts.record(candidate, color);
                    let candidate_length = unit.len() + plan_length(&continuation);
                    let saved_length = plan_length(&saved);
                    if candidate_length < saved_length {
                        selected_mixed_actions = None;
                        selected_pickup_actions = None;
                        selected_target_candidate = Some((candidate, color));
                        selected_target_saving = before_target_length - candidate_length;
                        target_saved += 1;
                        target_savings += saved_length - candidate_length;
                        target_saved_counts.record(candidate, color);
                        continuation.push_front(unit);
                        saved = continuation;
                    }
                }
            }
            target_time += target_started.elapsed();
        }
        if let Some((source, _)) = choose_target(&board, &distances) {
            let stack = &board.stacks[source];
            if stack.top_run_len() == stack.len() {
                existing_mixed_stats.not_mixed += 1;
            } else if let Some(colors) = existing_mixed_colors(stack) {
                if existing_mixed_stats.attempts >= MAX_EXISTING_MIXED_CALLS {
                    existing_mixed_stats.call_skips += 1;
                } else if existing_mixed_stats.elapsed >= EXISTING_MIXED_CASE_TIME {
                    existing_mixed_stats.case_time_skips += 1;
                } else if Instant::now() >= deadline {
                    existing_mixed_stats.global_time_skips += 1;
                } else {
                    let call_started = Instant::now();
                    let call_end = call_started + EXISTING_MIXED_CALL_TIME;
                    let case_end =
                        call_started + (EXISTING_MIXED_CASE_TIME - existing_mixed_stats.elapsed);
                    let local_deadline = deadline.min(call_end).min(case_end);
                    existing_mixed_stats.attempts += 1;
                    let found = existing_mixed_search(&board, source, colors, local_deadline);
                    existing_mixed_stats.record_search(&found);
                    let _ = writeln!(
                        existing_mixed_stats.events,
                        "existing_mixed_search step={} source={} colors={:?} stack={:?} pickups={:?} registered={} expanded={} terminals={:?} generated={} timed_out={} state_limit={} depth_limit={} S={}",
                        actions.len(),
                        source,
                        colors,
                        &stack.colors[..stack.len()],
                        found.pickups,
                        found.registered,
                        found.expanded,
                        found.terminals,
                        found.plans.len(),
                        found.timed_out,
                        found.state_limit,
                        found.depth_limit,
                        plan_length(&saved)
                    );
                    let mut executed_candidate = None;
                    for (index, candidate) in found.plans.into_iter().enumerate() {
                        if Instant::now() >= local_deadline {
                            break;
                        }
                        let before = plan_length(&saved);
                        let result = candidate_rollout(
                            &board,
                            &distances,
                            &candidate.prefix,
                            MAX_OPERATIONS - actions.len(),
                            before,
                            Some(local_deadline),
                            &mut existing_mixed_stats.rollout,
                        );
                        let completed = result.as_ref().ok().map(plan_length);
                        let status = match &result {
                            Ok(_) => "completed",
                            Err(RolloutStop::Incumbent) => "pruned",
                            Err(RolloutStop::Deadline) => "deadline",
                            Err(RolloutStop::OperationLimit) => "operation_limit",
                            Err(RolloutStop::NoMove) => "no_move",
                        };
                        let _ = writeln!(
                            existing_mixed_stats.events,
                            "existing_mixed_candidate step={} index={} S={} p={} status={} continuation={:?} collected={} end={} end_colors={:?} returned={:?} split={}",
                            actions.len(),
                            index,
                            before,
                            candidate.prefix.len(),
                            status,
                            completed,
                            candidate.collected,
                            candidate.end.cell,
                            candidate.end.colors,
                            candidate.returned,
                            candidate.split
                        );
                        if let Ok(mut continuation) = result {
                            let length = candidate.prefix.len() + plan_length(&continuation);
                            if length < before {
                                existing_mixed_stats.saved += 1;
                                existing_mixed_stats.savings += before - length;
                                let _ = writeln!(
                                    existing_mixed_stats.events,
                                    "existing_mixed_save step={} index={} saving={} prefix={:?} pickups={:?} collected={} returned={:?} split={}",
                                    actions.len(),
                                    index,
                                    before - length,
                                    candidate.prefix,
                                    found.pickups,
                                    candidate.collected,
                                    candidate.returned,
                                    candidate.split
                                );
                                executed_candidate = Some(index);
                                continuation.push_front(candidate.prefix);
                                saved = continuation;
                            }
                        }
                    }
                    if let Some(index) = executed_candidate {
                        existing_mixed_stats.executed += 1;
                        let _ = writeln!(
                            existing_mixed_stats.events,
                            "existing_mixed_execute step={} index={}",
                            actions.len(),
                            index
                        );
                    }
                    let ended = Instant::now();
                    existing_mixed_stats.elapsed += ended - call_started;
                    if ended >= local_deadline {
                        if local_deadline == deadline {
                            existing_mixed_stats.global_time_hits += 1;
                        } else if local_deadline == case_end {
                            existing_mixed_stats.case_time_hits += 1;
                        } else {
                            existing_mixed_stats.call_time_hits += 1;
                        }
                    }
                }
            } else {
                existing_mixed_stats.too_many_colors += 1;
            }
        }
        if let Some(length) = selected_mixed_actions {
            mixed_actions += length;
        }
        if let Some(length) = selected_pickup_actions {
            pickup_executed += 1;
            pickup_actions += length;
        }
        if let Some((candidate, first_color)) = selected_target_candidate {
            target_executed[candidate.rank] += 1;
            target_executed_counts.record(candidate, first_color);
            target_events.push((
                candidate.rank,
                actions.len(),
                selected_target_saving,
                candidate.color == first_color,
                candidate.slot.index(),
            ));
        }
        let unit = saved.pop_front().expect("Nonempty saved continuation");
        for action in unit {
            board.apply(action);
            actions.push(action);
        }
    }
    existing_mixed_stats.log();
    pickup_approach_stats.log();
    for (stats, kind) in rollout_stats
        .iter()
        .zip(["normal", "merge", "mixed", "pickup", "target"])
    {
        stats.log(kind);
    }
    eprintln!(
        "merge_search replans={replans} candidates={candidates} rollouts={rollouts} accepted={accepted} mixed_attempts={mixed_attempts} mixed_partners={mixed_partners} mixed_candidates={mixed_candidates} mixed_rollouts={mixed_rollouts} mixed_accepted={mixed_accepted} mixed_actions={mixed_actions} pickup_attempts={pickup_attempts} pickup_triples={pickup_triples} pickup_approaches={pickup_approaches_count} pickup_candidates={pickup_candidates_count} pickup_rollouts={pickup_rollouts} pickup_saved={pickup_saved} pickup_executed={pickup_executed} pickup_actions={pickup_actions} target_comparisons={target_comparisons} target_rollouts={target_rollouts} target_saved={target_saved} target_savings={target_savings} target_rank2={} target_rank3={} target_rank4={} target_deadline_hits={target_deadline_hits}",
        target_executed[2], target_executed[3], target_executed[4],
    );
    eprintln!(
        "target_sampling seed={TARGET_SAMPLE_SEED} extracted_rank={:?} completed_rank={:?} saved_rank={:?} executed_rank={:?} extracted_color={:?} completed_color={:?} saved_color={:?} executed_color={:?} extracted_slot={:?} completed_slot={:?} saved_slot={:?} executed_slot={:?}",
        target_extracted_counts.rank,
        target_completed_counts.rank,
        target_saved_counts.rank,
        target_executed_counts.rank,
        target_extracted_counts.color,
        target_completed_counts.color,
        target_saved_counts.color,
        target_executed_counts.color,
        target_extracted_counts.slot,
        target_completed_counts.slot,
        target_saved_counts.slot,
        target_executed_counts.slot,
    );
    eprintln!(
        "target_profile target_time_us={} deadline_skipped_normal={deadline_skipped_normal} deadline_skipped_mixed={deadline_skipped_mixed} deadline_skipped_pickup={deadline_skipped_pickup} merge_deadline_breaks={merge_deadline_breaks} mixed_deadline_breaks={mixed_deadline_breaks} pickup_deadline_breaks={pickup_deadline_breaks}",
        target_time.as_micros(),
    );
    let mut event_log = String::new();
    for (rank, operation, saving, same_color, slot) in target_events {
        let _ = write!(
            event_log,
            "{rank}:{operation}:{saving}:{same_color}:{slot},"
        );
    }
    eprintln!("target_events={event_log}");

    let mut output = String::new();
    for action in actions {
        let _ = writeln!(
            output,
            "{} {} {} {} {}",
            action.from / n,
            action.from % n,
            action.k,
            DIRECTIONS[action.direction].2,
            action.length,
        );
    }
    print!("{output}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board() -> Board {
        Board {
            n: 7,
            walls: vec![false; 49],
            nests: vec![None; 49],
            stacks: vec![Stack::default(); 49],
        }
    }

    fn compare_pickup_approaches(
        board: &Board,
        source: usize,
        partner: usize,
        thirds: &[usize],
    ) -> Vec<Vec<PickupApproach>> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let stacks = board.stacks.clone();
        let mut stats = PickupApproachStats::default();
        let mut results = Vec::new();
        if !thirds.is_empty() {
            let mut search = PickupApproachBfs::new(board, source, partner, thirds, &mut stats);
            for &third in thirds {
                let actual = search.request(third, deadline, &mut stats);
                let expected = reference_pickup_approaches(board, source, partner, third, deadline);
                // Action's equality covers all fields; packed colors cover order and count.
                assert_eq!(actual, expected, "third={third}");
                results.push(actual);
            }
            assert_eq!(stats.initialized, 1);
        }
        assert_eq!(stats.requests, thirds.len());
        assert_eq!(stats.deadline_hits, 0);
        assert_eq!(board.stacks, stacks);
        results
    }

    fn pickup_line_board(n: usize, cells: &[usize]) -> Board {
        let mut board = Board {
            n,
            walls: vec![true; n * n],
            nests: vec![None; n * n],
            stacks: vec![Stack::default(); n * n],
        };
        for &cell in cells {
            board.walls[cell] = false;
        }
        board
    }

    #[test]
    fn shared_pickup_zero_one_and_unreachable_targets() {
        let mut board = empty_board();
        board.stacks[22].push(0);
        board.stacks[23].push(1);
        board.nests[6] = Some(0);
        board.nests[48] = Some(1);
        let distances = vec![distances_from(&board, 6), distances_from(&board, 48)];
        let mut stats = PickupApproachStats::default();
        let candidates = pickup_candidates(
            &board,
            22,
            0,
            &distances,
            Instant::now() + Duration::from_secs(60),
            &mut stats,
        );
        assert_eq!(
            (
                candidates.triples,
                candidates.approaches,
                stats.initialized,
                stats.requests,
                stats.expanded
            ),
            (0, 0, 0, 0, 0)
        );

        let mut line = pickup_line_board(7, &[0, 24, 25, 48]);
        line.stacks[0].push(0);
        line.stacks[24].push(1);
        line.stacks[25].push(0);
        line.stacks[48].push(1);
        let single = compare_pickup_approaches(&line, 0, 24, &[25]);
        assert_eq!(single[0].len(), 1);
        assert_eq!(single[0][0].0.len(), 1);
        let multiple = compare_pickup_approaches(&line, 0, 24, &[48, 25]);
        assert!(multiple[0].is_empty());
        assert_eq!(multiple[1], single[0]);

        let deadline = Instant::now() + Duration::from_secs(60);
        let mut stats = PickupApproachStats::default();
        let mut search = PickupApproachBfs::new(&line, 0, 24, &[48, 25], &mut stats);
        assert!(search.request(48, deadline, &mut stats).is_empty());
        assert!(search.queue.is_empty());
        let expanded = stats.expanded;
        search.request(25, deadline, &mut stats);
        assert_eq!(stats.expanded, expanded);
    }

    #[test]
    fn shared_pickup_reuses_early_arrivals_and_resumes_at_state_boundaries() {
        let mut board = empty_board();
        board.stacks[0].push(0);
        board.stacks[24].push(1);
        board.stacks[25].push(0);
        board.stacks[48].push(1);
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut stats = PickupApproachStats::default();
        let mut search = PickupApproachBfs::new(&board, 0, 24, &[48, 25], &mut stats);
        let far = search.request(48, deadline, &mut stats);
        assert_eq!(far.len(), 2);
        assert!(search.arrivals[1].found.iter().all(|&found| found));
        let expanded = stats.expanded;
        let near = search.request(25, deadline, &mut stats);
        assert_eq!(near.len(), 2);
        assert_eq!(stats.expanded, expanded);
        assert_eq!(
            near,
            reference_pickup_approaches(&board, 0, 24, 25, deadline)
        );
        assert_eq!(
            far,
            reference_pickup_approaches(&board, 0, 24, 48, deadline)
        );
        // Far routes can use the other selected group as a springboard.
        assert!(
            far.iter()
                .any(|(path, _)| path.iter().any(|a| a.from == 25 && a.k == 1))
        );

        let mut stats = PickupApproachStats::default();
        let mut search = PickupApproachBfs::new(&board, 0, 24, &[25, 48], &mut stats);
        assert_eq!(search.request(25, deadline, &mut stats), near);
        assert!(!search.queue.is_empty());
        let expanded = stats.expanded;
        assert_eq!(search.request(48, deadline, &mut stats), far);
        assert!(stats.expanded > expanded);
        // Every state's predecessor is fixed at first discovery, including ties.
        assert!(stats.expanded <= board.stacks.len() * 2);
    }

    #[test]
    fn shared_pickup_preserves_first_path_among_equal_length_routes() {
        let mut board = empty_board();
        board.stacks[0].push(0);
        board.stacks[24].push(1);
        board.stacks[32].push(0);
        let results = compare_pickup_approaches(&board, 0, 24, &[32]);
        let first = &results[0][0];
        assert_eq!(
            first.0.iter().map(|a| a.direction).collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(first.1, TwoColorStack { bits: 2, len: 3 });
    }

    #[test]
    fn shared_pickup_depth_boundary_is_inclusive_for_arrival() {
        let cells: Vec<_> = std::iter::once(0).chain(201..220).collect();
        let mut board = pickup_line_board(20, &cells);
        board.stacks[0].push(0);
        board.stacks[201].push(1);
        board.stacks[213].push(0);
        board.stacks[214].push(1);
        let results = compare_pickup_approaches(&board, 0, 201, &[213, 214]);
        assert_eq!(results[0].len(), 1);
        assert_eq!(results[0][0].0.len(), MAX_PICKUP_APPROACH_STEPS);
        assert!(results[1].is_empty());
    }

    #[test]
    fn shared_pickup_retains_height_homogeneity_and_homecoming_guards() {
        let mut board = pickup_line_board(7, &[0, 24, 25]);
        board.stacks[0].push(0);
        board.stacks[24].push(1);
        for _ in 0..7 {
            board.stacks[25].push(0);
        }
        assert!(compare_pickup_approaches(&board, 0, 24, &[25])[0].is_empty());
        board.stacks[25] = Stack::default();
        board.stacks[25].push(0);
        board.stacks[25].push(1);
        assert!(compare_pickup_approaches(&board, 0, 24, &[25])[0].is_empty());
        board.stacks[25] = Stack::default();
        board.stacks[25].push(0);
        board.nests[25] = Some(1); // Direct incoming top is partner color.
        assert!(compare_pickup_approaches(&board, 0, 24, &[25])[0].is_empty());
        board.nests[25] = Some(0);
        assert_eq!(compare_pickup_approaches(&board, 0, 24, &[25])[0].len(), 1);
        board.walls[25] = true;
        assert!(compare_pickup_approaches(&board, 0, 24, &[25])[0].is_empty());
    }

    #[test]
    fn shared_pickup_deadline_keeps_queue_and_only_returns_complete_paths() {
        let mut board = empty_board();
        board.stacks[0].push(0);
        board.stacks[24].push(1);
        board.stacks[25].push(0);
        let mut stats = PickupApproachStats::default();
        let mut search = PickupApproachBfs::new(&board, 0, 24, &[25], &mut stats);
        assert!(search.request(25, Instant::now(), &mut stats).is_empty());
        assert_eq!((stats.expanded, stats.deadline_hits), (0, 1));
        assert_eq!(search.queue, VecDeque::from([24 * 2]));
        let deadline = Instant::now() + Duration::from_secs(60);
        assert_eq!(
            search.request(25, deadline, &mut stats),
            reference_pickup_approaches(&board, 0, 24, 25, deadline)
        );
    }

    #[test]
    fn shared_pickup_matches_reference_on_varied_backgrounds_and_target_orders() {
        for variant in 0..32 {
            let mut board = empty_board();
            for cell in 0..49 {
                board.walls[cell] = (cell * 7 + variant * 3) % 17 == 0;
                if cell % 9 == variant % 9 {
                    for _ in 0..1 + (cell + variant) % 6 {
                        board.stacks[cell].push(2);
                    }
                }
            }
            let source = 22;
            let partner = 23;
            let mut thirds = vec![6, 25, 48];
            if variant % 2 == 1 {
                thirds.reverse();
            }
            for (index, cell) in [source, partner, 6, 25, 48].into_iter().enumerate() {
                board.walls[cell] = false;
                board.stacks[cell] = Stack::default();
                board.stacks[cell].push((index % 2) as u8);
            }
            board.nests[10] = Some(0);
            board.nests[38] = Some(1);
            compare_pickup_approaches(&board, source, partner, &thirds);
        }
    }

    fn pair_fixture() -> (Board, Vec<Vec<usize>>) {
        let mut board = empty_board();
        board.nests[3] = Some(0);
        board.stacks[23].push(0);
        board.stacks[25].push(0);
        let distances = vec![distances_from(&board, 3)];
        (board, distances)
    }

    #[test]
    fn incumbent_bound_preserves_shorter_complete_rollouts() {
        let (board, distances) = pair_fixture();
        let unbounded = legacy_rollout(&board, &distances, MAX_OPERATIONS, None, None).unwrap();
        let length = plan_length(&unbounded);
        assert_eq!(
            unbounded.front().unwrap(),
            &choose_pair(&board, 23, 0, &distances).unwrap()
        );
        assert_eq!(
            legacy_rollout(&board, &distances, length, None, None).unwrap(),
            unbounded
        );
        assert_eq!(
            legacy_rollout(&board, &distances, MAX_OPERATIONS, None, Some(length + 1)).unwrap(),
            unbounded
        );
        let mut after = board.clone();
        for unit in &unbounded {
            for &action in unit {
                after.apply(action);
            }
        }
        assert!(after.stacks.iter().all(|stack| stack.len() == 0));
    }

    #[test]
    fn tied_and_longer_candidates_preserve_incumbent_and_input() {
        let (board, distances) = pair_fixture();
        let saved = legacy_rollout(&board, &distances, MAX_OPERATIONS, None, None).unwrap();
        let original = saved.clone();
        let stacks = board.stacks.clone();
        let length = plan_length(&saved);
        let mut stats = RolloutStats::default();
        for bound in [length, length - 1] {
            let result = candidate_rollout(
                &board,
                &distances,
                &[],
                MAX_OPERATIONS,
                bound,
                None,
                &mut stats,
            );
            assert_eq!(result.unwrap_err(), RolloutStop::Incumbent);
            assert_eq!(saved, original);
            assert_eq!(board.stacks, stacks);
        }
        assert_eq!((stats.attempts, stats.completed, stats.pruned), (2, 0, 2));
    }

    #[test]
    fn whole_pair_is_rejected_without_shorter_fallback() {
        let (board, distances) = pair_fixture();
        let (cell, color) = choose_target(&board, &distances).unwrap();
        assert_eq!(
            legacy_unit(&board, cell, color, &distances).unwrap().len(),
            2
        );
        assert_eq!(
            legacy_rollout(&board, &distances, MAX_OPERATIONS, None, Some(2)).unwrap_err(),
            RolloutStop::Incumbent
        );
        assert_eq!(
            legacy_rollout(&board, &distances, 1, None, None).unwrap_err(),
            RolloutStop::OperationLimit
        );
    }

    #[test]
    fn completed_prefix_uses_zero_continuation_and_guarded_subtraction() {
        let mut board = empty_board();
        board.nests[1] = Some(0);
        board.stacks[0].push(0);
        let distances = vec![distances_from(&board, 1)];
        let prefix = [Action {
            from: 0,
            k: 0,
            direction: 3,
            length: 1,
        }];
        let mut stats = RolloutStats::default();
        assert!(
            candidate_rollout(
                &board,
                &distances,
                &prefix,
                MAX_OPERATIONS,
                2,
                None,
                &mut stats
            )
            .unwrap()
            .is_empty()
        );
        for saved_length in [0, 1] {
            assert_eq!(
                candidate_rollout(
                    &board,
                    &distances,
                    &prefix,
                    MAX_OPERATIONS,
                    saved_length,
                    None,
                    &mut stats
                )
                .unwrap_err(),
                RolloutStop::Incumbent
            );
        }
        let finished = empty_board();
        assert!(
            legacy_rollout(&finished, &distances, 0, None, None)
                .unwrap()
                .is_empty()
        );
        assert!(
            legacy_rollout(&finished, &distances, 0, None, Some(1))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            legacy_rollout(&finished, &distances, 0, None, Some(0)).unwrap_err(),
            RolloutStop::Incumbent
        );
        assert_eq!((stats.attempts, stats.completed, stats.pruned), (3, 1, 2));
    }

    #[test]
    fn incomplete_reasons_are_counted_separately() {
        let (board, distances) = pair_fixture();
        let mut stats = RolloutStats::default();
        let expired = Instant::now();
        assert_eq!(
            candidate_rollout(
                &board,
                &distances,
                &[],
                MAX_OPERATIONS,
                100,
                Some(expired),
                &mut stats
            )
            .unwrap_err(),
            RolloutStop::Deadline
        );
        assert_eq!(
            candidate_rollout(&board, &distances, &[], 1, 100, None, &mut stats).unwrap_err(),
            RolloutStop::OperationLimit
        );
        let prefix = legacy_unit(&board, 23, 0, &distances).unwrap();
        assert_eq!(
            candidate_rollout(&board, &distances, &prefix, 1, 100, None, &mut stats).unwrap_err(),
            RolloutStop::OperationLimit
        );
        assert_eq!(
            (
                stats.attempts,
                stats.completed,
                stats.pruned,
                stats.timed_out,
                stats.operation_limit
            ),
            (3, 0, 0, 1, 2)
        );
    }

    #[test]
    fn ordinary_targets_preserve_legacy_first_choice_and_cell_ties() {
        let mut board = empty_board();
        board.nests[24] = Some(0);
        board.nests[6] = Some(1);
        for cell in [0, 48, 16, 18] {
            board.stacks[cell].push(0);
        }
        let distances = vec![distances_from(&board, 24), distances_from(&board, 6)];
        let mut rng = TargetRng::new(TARGET_SAMPLE_SEED);
        let targets = ordinary_target_candidates(&board, &distances, &mut rng).unwrap();
        assert_eq!(targets.first, choose_target(&board, &distances).unwrap());
        assert_eq!(
            targets
                .additional
                .iter()
                .map(|t| t.cell)
                .collect::<Vec<_>>(),
            vec![48, 16, 18]
        );
        assert_eq!(targets.additional[2].slot, TargetSlot::Fourth);

        board.stacks[25].push(0);
        let targets = ordinary_target_candidates(&board, &distances, &mut rng).unwrap();
        assert_eq!(targets.additional.len(), 3);
        assert!(
            targets
                .additional
                .iter()
                .any(|t| t.rank == 4 && t.slot == TargetSlot::Fourth)
        );
        assert!(
            targets
                .additional
                .windows(2)
                .all(|pair| pair[0].rank < pair[1].rank)
        );
        assert!(
            !targets
                .additional
                .iter()
                .any(|t| t.slot == TargetSlot::OtherColor)
        );

        for _ in 1..MAX_HEIGHT {
            board.stacks[25].push(0);
        }
        assert!(ordinary_target_candidates(&board, &distances, &mut rng).is_none());
        board.stacks[25] = Stack::default();
        board.stacks[25].push(0);
        board.stacks[25].push(1);
        assert!(ordinary_target_candidates(&board, &distances, &mut rng).is_none());
    }

    #[test]
    fn sampled_targets_are_distinct_ranked_and_reproducible() {
        let mut board = empty_board();
        board.nests[24] = Some(0);
        board.nests[6] = Some(1);
        for cell in 0..20 {
            if board.nests[cell].is_none() {
                board.stacks[cell].push((cell % 2) as u8);
            }
        }
        let distances = vec![distances_from(&board, 24), distances_from(&board, 6)];
        let mut left = TargetRng::new(TARGET_SAMPLE_SEED);
        let mut right = TargetRng::new(TARGET_SAMPLE_SEED);
        for _ in 0..20 {
            let a = ordinary_target_candidates(&board, &distances, &mut left).unwrap();
            let b = ordinary_target_candidates(&board, &distances, &mut right).unwrap();
            assert_eq!(a.first, b.first);
            assert_eq!(a.additional, b.additional);
            assert_eq!(a.first, choose_target(&board, &distances).unwrap());
            assert_eq!(a.additional.len(), 3);
            assert!(
                a.additional
                    .windows(2)
                    .all(|pair| pair[0].rank < pair[1].rank)
            );
            assert!(
                a.additional
                    .iter()
                    .any(|candidate| candidate.rank == 4 && candidate.slot == TargetSlot::Fourth)
            );
            assert!(a.additional.iter().all(|candidate| candidate.rank > 1));
            assert!(
                a.additional
                    .iter()
                    .filter(|candidate| candidate.slot == TargetSlot::OtherColor)
                    .all(|candidate| candidate.color != a.first.1)
            );
        }
        let mut counts = TargetCounts::default();
        counts.record(
            TargetCandidate {
                cell: 0,
                color: 1,
                rank: 12,
                slot: TargetSlot::Overall,
            },
            0,
        );
        assert_eq!(counts.rank, [0, 0, 1]);
        assert_eq!(counts.color, [0, 1]);
        assert_eq!(counts.slot, [0, 0, 1, 0]);
    }

    #[test]
    fn sampled_targets_include_every_alternative_when_groups_are_few() {
        let mut board = empty_board();
        board.nests[24] = Some(0);
        board.nests[6] = Some(1);
        board.stacks[0].push(0);
        board.stacks[48].push(1);
        let distances = vec![distances_from(&board, 24), distances_from(&board, 6)];
        let mut rng = TargetRng::new(TARGET_SAMPLE_SEED);
        let two = ordinary_target_candidates(&board, &distances, &mut rng).unwrap();
        assert_eq!(two.additional.len(), 1);
        assert_eq!(two.additional[0].rank, 2);

        board.stacks[16].push(0);
        let three = ordinary_target_candidates(&board, &distances, &mut rng).unwrap();
        assert_eq!(
            three.additional.iter().map(|t| t.rank).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn shared_legacy_unit_matches_first_rollout_unit() {
        let mut board = empty_board();
        let source = 3 * 7 + 2;
        board.nests[3] = Some(0);
        board.stacks[source].push(0);
        board.stacks[source + 2].push(0);
        let distances = vec![distances_from(&board, 3)];
        let (cell, color) = choose_target(&board, &distances).unwrap();
        let unit = legacy_unit(&board, cell, color, &distances).unwrap();
        assert_eq!(unit.len(), 2);
        let rollout = legacy_rollout(&board, &distances, MAX_OPERATIONS, None, None).unwrap();
        let first = rollout.front().unwrap();
        let action_fields = |actions: &[Action]| {
            actions
                .iter()
                .map(|a| (a.from, a.k, a.direction, a.length))
                .collect::<Vec<_>>()
        };
        assert_eq!(action_fields(&unit), action_fields(first));
    }

    #[test]
    fn pickup_preserves_three_blocks_and_continues_after_partial_homecoming() {
        let mut board = empty_board();
        let source = 3 * 7 + 1;
        let partner = source + 1;
        let third = source + 3;
        let source_nest = third + 1;
        let partner_nest = 2 * 7 + 6;
        board.stacks[source].push(0);
        board.stacks[partner].push(1);
        board.stacks[third].push(0);
        board.nests[source_nest] = Some(0);
        board.nests[partner_nest] = Some(1);
        let merge = [Action {
            from: source,
            k: 0,
            direction: 3,
            length: 1,
        }];
        let deadline = Instant::now() + Duration::from_secs(1);
        let approaches = pickup_approaches(&board, source, partner, third, deadline);
        let (approach, colors) = approaches
            .iter()
            .find(|(actions, colors)| actions.len() == 2 && colors.bits == 0b010)
            .unwrap();
        assert_eq!(colors.len, 3);
        let partial = colors.after_home(Some(0), 0, 1);
        assert_eq!((partial.bits, partial.len), (0b10, 2));
        assert!(!partial.is_single_color());
        let plans = pickup_finish_plans(
            &board,
            [source, partner, third],
            &merge,
            approach,
            *colors,
            deadline,
        );
        assert!(!plans.is_empty());
        assert!(plans.iter().any(|plan| {
            let mut after = board.clone();
            let mut saw_partial = false;
            for &action in plan {
                after.apply(action);
                if after.stacks[source_nest].len() == 2
                    && after.stacks[source_nest].top_run_len() == 1
                {
                    saw_partial = true;
                }
            }
            saw_partial
                && after.stacks[third].len() == 0
                && after.stacks[partner_nest].len() == 1
                && after.stacks[partner_nest].last() == Some(0)
        }));
    }

    #[test]
    fn pickup_rejects_landing_above_height_limit() {
        let mut board = empty_board();
        let source = 3 * 7 + 1;
        let partner = source + 1;
        let third = source + 2;
        for _ in 0..4 {
            board.stacks[source].push(0);
        }
        for _ in 0..3 {
            board.stacks[partner].push(1);
        }
        for _ in 0..2 {
            board.stacks[third].push(0);
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        assert!(pickup_approaches(&board, source, partner, third, deadline).is_empty());
    }

    #[test]
    fn mixed_transport_checks_top_color_before_homecoming() {
        let mut board = empty_board();
        let source = 3 * 7 + 1;
        let partner = source + 1;
        let source_nest = partner + 1;
        let partner_nest = 2 * 7 + 2;
        board.stacks[source].push(0);
        board.stacks[partner].push(1);
        board.nests[source_nest] = Some(0);
        board.nests[partner_nest] = Some(1);
        let merge = [Action {
            from: source,
            k: 0,
            direction: 3,
            length: 1,
        }];
        let plans = mixed_transport_plans(&board, source, partner, &merge);
        assert_eq!(plans.len(), 1);
        let mut after = board.clone();
        for &action in &plans[0] {
            after.apply(action);
        }
        assert_eq!(after.stacks[partner_nest].last(), Some(0));
        assert_eq!(after.stacks[source_nest].len(), 0);
    }

    #[test]
    fn mixed_transport_jumps_from_fixed_springboard_and_restores_it() {
        let mut board = empty_board();
        let source = 3 * 7 + 1;
        let partner = source + 1;
        let springboard = partner + 1;
        let source_nest = 3 * 7 + 6;
        board.stacks[source].push(0);
        board.stacks[partner].push(1);
        board.stacks[springboard].push(2);
        board.stacks[springboard].push(2);
        board.nests[source_nest] = Some(0);
        board.nests[2 * 7 + 2] = Some(1);
        let merge = [Action {
            from: source,
            k: 0,
            direction: 3,
            length: 1,
        }];
        let plans = mixed_transport_plans(&board, source, partner, &merge);
        assert_eq!(plans.len(), 2);
        let springboard_plan = plans.iter().find(|plan| plan.len() == 3).unwrap();
        assert_eq!(springboard_plan[2].length, 3);
        let mut after = board.clone();
        for &action in springboard_plan {
            after.apply(action);
        }
        assert_eq!(after.stacks[source_nest].last(), Some(1));
        assert_eq!(after.stacks[source_nest].len(), 1);
        assert_eq!(after.stacks[springboard], board.stacks[springboard]);
    }

    #[test]
    fn group_move_leaves_enough_support_to_jump_six_slimes_three_cells() {
        let mut board = empty_board();
        board.walls.fill(true);
        let source = 3 * 7;
        board.walls[source..source + 7].fill(false);
        board.nests[source + 6] = Some(1);
        board.stacks[source].push(0);
        for _ in 0..7 {
            board.stacks[source].push(1);
        }
        for cell in [source + 1, source + 2] {
            for _ in 0..3 {
                board.stacks[cell].push(0);
            }
        }
        board.stacks[source + 3].push(1);
        let distances = distances_from(&board, source + 6);
        assert!(choose_move(&board, source, 1, 7, &distances).is_none());
        let (action, to) = choose_group_move(&board, source, 1, &distances).unwrap();
        assert_eq!((action.k, action.length, to), (2, 3, source + 3));
        let mut after = board.clone();
        after.apply(action);
        assert_eq!(&after.stacks[source].colors[..2], &[0, 1]);
        assert_eq!(after.stacks[to].top_run_len(), 7);
        // Towers can be passed over, but a wall cannot be jumped over.
        board.walls[source + 2] = true;
        let (action, to) = choose_group_move(&board, source, 1, &distances).unwrap();
        assert_eq!((action.k, action.length, to), (3, 1, source + 1));
    }

    #[test]
    fn group_move_limits_landing_height_before_returning_home() {
        let mut board = empty_board();
        board.walls.fill(true);
        let source = 3 * 7;
        board.walls[source..source + 2].fill(false);
        board.nests[source + 1] = Some(1);
        for _ in 0..8 {
            board.stacks[source].push(1);
        }
        for _ in 0..6 {
            board.stacks[source + 1].push(0);
        }
        let distances = distances_from(&board, source + 1);
        let (action, to) = choose_group_move(&board, source, 1, &distances).unwrap();
        assert_eq!((action.k, action.length, to), (6, 1, source + 1));
        board.apply(action);
        assert_eq!(board.stacks[source].len(), 6);
        assert_eq!(board.stacks[to].len(), 6);
    }

    #[test]
    fn group_move_still_moves_the_whole_run_when_possible() {
        let mut board = empty_board();
        let source = 3 * 7;
        board.nests[source + 6] = Some(0);
        for _ in 0..4 {
            board.stacks[source].push(0);
        }
        let distances = distances_from(&board, source + 6);
        let (action, to) = choose_group_move(&board, source, 0, &distances).unwrap();
        assert_eq!((action.k, action.length, to), (0, 1, source + 1));
    }

    #[test]
    fn pair_completes_a_sideways_merge_and_stops_at_two() {
        let mut board = empty_board();
        let source = 3 * 7 + 2;
        let partner = 3 * 7 + 4;
        board.nests[3] = Some(0);
        board.stacks[source].push(0);
        board.stacks[partner].push(0);
        let distances = vec![distances_from(&board, 3)];
        let plan = choose_pair(&board, source, 0, &distances).unwrap();
        assert_eq!(plan.len(), 2);
        for action in plan {
            board.apply(action);
        }
        assert_eq!(board.stacks[source].len(), 0);
        assert_eq!(board.stacks[partner].len(), 2);
        assert_eq!(board.stacks[partner].top_run_len(), 2);
        assert!(choose_pair(&board, partner, 0, &distances).is_none());
    }

    #[test]
    fn pair_rejects_distant_partners_and_existing_towers() {
        let mut board = empty_board();
        let source = 3 * 7;
        let partner = source + 4;
        board.nests[3] = Some(0);
        board.stacks[source].push(0);
        board.stacks[partner].push(0);
        let distances = vec![distances_from(&board, 3)];
        assert!(choose_pair(&board, source, 0, &distances).is_none());
        board.stacks[partner].len = 0;
        board.stacks[source + 2].push(0);
        board.stacks[source + 2].push(0);
        assert!(choose_pair(&board, source, 0, &distances).is_none());
    }

    #[test]
    fn support_loss_counts_a_usable_jump_but_respects_walls() {
        let mut board = empty_board();
        let source = 3 * 7 + 3;
        let nest = 3 * 7 + 6;
        board.nests[3] = Some(0);
        board.nests[nest] = Some(1);
        board.stacks[source].push(0);
        board.stacks[source - 1].push(1);
        let distances = vec![distances_from(&board, 3), distances_from(&board, nest)];
        assert_eq!(lost_singleton_support(&board, source, &distances), 1);
        board.walls[source + 1] = true;
        let distances = vec![distances_from(&board, 3), distances_from(&board, nest)];
        assert_eq!(lost_singleton_support(&board, source, &distances), 0);
    }

    fn put_colors(board: &mut Board, cell: usize, colors: &[u8]) {
        board.stacks[cell] = Stack::default();
        for &color in colors {
            board.stacks[cell].push(color);
        }
    }

    #[test]
    fn existing_mixed_alternating_transitions_match_actual_board() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1, 0, 1]);
        put_colors(&mut board, 23, &[0, 0]);
        put_colors(&mut board, 25, &[1]);
        board.nests[17] = Some(0);
        board.nests[31] = Some(1);
        let (model, initial) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let mut nodes = vec![(initial, 0)];
        let mut seen = HashSet::from([initial]);
        let mut head = 0;
        let mut partial_return = false;
        while head < nodes.len() {
            let (state, depth) = nodes[head];
            head += 1;
            if depth == 4 || model.terminal(state) {
                continue;
            }
            for t in model.transitions(state) {
                let mut actual = model.board_at(state);
                let old_count: usize = actual.stacks.iter().map(Stack::len).sum();
                assert!(checked_existing_action(&mut actual, t.action));
                assert_eq!(actual.stacks, model.board_at(t.state).stacks);
                partial_return |= actual.stacks.iter().map(Stack::len).sum::<usize>() < old_count
                    && !t.state.colors.is_single_color();
                if seen.insert(t.state) {
                    nodes.push((t.state, depth + 1));
                }
            }
        }
        assert!(partial_return);
        assert!(nodes.len() > 30);
    }

    #[test]
    fn existing_mixed_mask_changes_background_and_jump_range() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 23, &[0, 0]);
        let (model, mut state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        state.cell = 23;
        let pass = state;
        state.collected = 1;
        assert_eq!(model.stack(23, pass.collected).len(), 2);
        assert_eq!(model.stack(23, state.collected).len(), 0);
        assert!(model.transitions(pass).iter().any(|t| t.action.length == 3));
        assert!(
            model
                .transitions(state)
                .iter()
                .all(|t| t.action.length == 1)
        );
        assert_ne!(model.board_at(pass).stacks, model.board_at(state).stacks);
        assert_ne!(pass, state);
    }

    #[test]
    fn existing_mixed_pass_and_collect_are_distinct_and_cannot_repeat() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 23, &[0]);
        let (model, initial) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let arrivals: Vec<_> = model
            .transitions(initial)
            .into_iter()
            .filter(|t| t.state.cell == 23)
            .collect();
        assert_eq!(arrivals.len(), 2);
        assert_eq!(arrivals[0].state.collected, 0);
        assert_eq!(arrivals[1].state.collected, 1);
        assert_eq!(
            model.board_at(arrivals[0].state).stacks,
            model.board_at(arrivals[1].state).stacks
        );
        assert_eq!(model.stack(23, arrivals[0].state.collected).len(), 1);
        assert_eq!(model.stack(23, arrivals[1].state.collected).len(), 0);
        let away = model
            .transitions(arrivals[1].state)
            .into_iter()
            .find(|t| t.state.cell == 24)
            .unwrap();
        let returns: Vec<_> = model
            .transitions(away.state)
            .into_iter()
            .filter(|t| t.state.cell == 23)
            .collect();
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].state.collected, 1);
        assert_eq!(returns[0].state.colors.len, 3);
    }

    #[test]
    fn existing_mixed_height_check_precedes_home_and_uses_remaining_count() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 0, 1, 0, 1, 0, 1, 1]);
        put_colors(&mut board, 23, &[1, 1]);
        put_colors(&mut board, 26, &[1, 1]);
        board.nests[25] = Some(0);
        board.nests[23] = Some(0);
        let (model, initial) = ExistingMixedModel::new(&board, 24, [0, 1]);
        assert!(model.pickups.contains(&23));
        assert!(model.pickups.contains(&26));
        assert!(
            !model
                .transitions(initial)
                .iter()
                .any(|t| t.state.cell == 23)
        );
        let partial = model
            .transitions(initial)
            .into_iter()
            .find(|t| t.state.cell == 25)
            .unwrap();
        assert_eq!(partial.state.colors.len, 6);
        assert!(!partial.state.colors.is_single_color());
        let collected = model
            .transitions(partial.state)
            .into_iter()
            .find(|t| t.state.cell == 26 && t.state.collected != 0)
            .unwrap();
        assert_eq!(collected.state.colors.len, 8);
        let mut actual = board.clone();
        assert!(checked_existing_action(&mut actual, partial.action));
        assert!(checked_existing_action(&mut actual, collected.action));
        assert_eq!(actual.stacks, model.board_at(collected.state).stacks);
    }

    #[test]
    fn existing_mixed_nest_split_returns_lower_and_moves_upper() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 0, 0, 0, 1, 1, 1, 1]);
        board.nests[24] = Some(0);
        let (model, initial) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let split = model
            .transitions(initial)
            .into_iter()
            .find(|t| t.split && t.action.direction == 3 && t.action.length == 2)
            .unwrap();
        assert_eq!(split.action.k, 4);
        assert_eq!(split.state.cell, 26);
        assert!(model.terminal(split.state));
        let mut actual = board.clone();
        assert!(checked_existing_action(&mut actual, split.action));
        assert_eq!(actual.stacks[24].len(), 0);
        assert_eq!(&actual.stacks[26].colors[..4], &[1, 1, 1, 1]);
        assert_eq!(actual.stacks, model.board_at(split.state).stacks);
        let mut occupied = board.clone();
        put_colors(&mut occupied, 26, &[1]);
        let (m, i) = ExistingMixedModel::new(&occupied, 24, [0, 1]);
        assert!(
            !m.transitions(i)
                .iter()
                .any(|t| t.split && t.state.cell == 26)
        );
        let mut walled = board.clone();
        walled.walls[25] = true;
        let (m, i) = ExistingMixedModel::new(&walled, 24, [0, 1]);
        assert!(
            !m.transitions(i)
                .iter()
                .any(|t| t.split && t.action.direction == 3)
        );
        let mut alternating = board;
        put_colors(&mut alternating, 24, &[0, 1, 0, 1]);
        let (m, i) = ExistingMixedModel::new(&alternating, 24, [0, 1]);
        assert!(!m.transitions(i).iter().any(|t| t.split));
    }

    #[test]
    fn existing_mixed_full_height_search_and_incumbent_fallback() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 0, 0, 0, 1, 1, 1, 1]);
        board.nests[24] = Some(0);
        board.nests[0] = Some(1);
        assert_eq!(existing_mixed_colors(&board.stacks[24]), Some([0, 1]));
        let found =
            existing_mixed_search(&board, 24, [0, 1], Instant::now() + Duration::from_secs(60));
        assert!(found.expanded > 0 && found.registered <= 4000);
        assert!(found.invalid == 0 && !found.plans.is_empty());
        assert!(found.plans.iter().any(|p| p.split && p.returned[0] == 4));
        let distances = vec![distances_from(&board, 24), distances_from(&board, 0)];
        let prefix = &found.plans[0].prefix;
        let mut stats = RolloutStats::default();
        let complete = candidate_rollout(
            &board,
            &distances,
            prefix,
            MAX_OPERATIONS,
            1000,
            None,
            &mut stats,
        )
        .unwrap();
        assert!(prefix.len() + plan_length(&complete) < 1000);
        let before = board.stacks.clone();
        let saved = Plan::from([vec![Action {
            from: 24,
            k: 4,
            direction: 3,
            length: 1,
        }]]);
        let saved_before = saved.clone();
        assert!(matches!(
            candidate_rollout(
                &board,
                &distances,
                prefix,
                MAX_OPERATIONS,
                plan_length(&saved),
                None,
                &mut stats
            ),
            Err(RolloutStop::Incumbent)
        ));
        assert_eq!(board.stacks, before);
        assert_eq!(saved, saved_before);
    }

    #[test]
    fn existing_mixed_expired_and_no_terminal_preserve_input() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        let before = board.stacks.clone();
        let found =
            existing_mixed_search(&board, 24, [0, 1], Instant::now() - Duration::from_secs(1));
        assert!(found.timed_out && found.plans.is_empty());
        assert_eq!(board.stacks, before);
        board.walls.fill(true);
        board.walls[24] = false;
        let found =
            existing_mixed_search(&board, 24, [0, 1], Instant::now() + Duration::from_secs(60));
        assert!(found.plans.is_empty() && !found.timed_out && found.invalid == 0);
        assert_eq!(found.registered, 1);
    }

    #[test]
    fn existing_mixed_depth_24_terminal_is_inclusive() {
        let mut board = Board {
            n: 25,
            walls: vec![true; 625],
            nests: vec![None; 625],
            stacks: vec![Stack::default(); 625],
        };
        for cell in 0..25 {
            board.walls[cell] = false;
        }
        put_colors(&mut board, 0, &[1, 0]);
        board.nests[24] = Some(0);
        let found =
            existing_mixed_search(&board, 0, [0, 1], Instant::now() + Duration::from_secs(60));
        assert_eq!(found.invalid, 0);
        assert!(
            found
                .plans
                .iter()
                .any(|p| p.prefix.len() == 24 && p.end.cell == 24)
        );
        assert!(found.plans.iter().all(|p| p.prefix.len() <= 24));
    }

    #[test]
    fn existing_mixed_case0000_fixed_state_is_legal() {
        let rows = [
            "............",
            "...c.##.....",
            ".A..c##.....",
            "...c#.##....",
            "c..##.###B..",
            "aa.#...##.b.",
            "..##C..###..",
            ".a#.d#..##.b",
            "a##d.##b.##.",
            "d...####b...",
            "...######...",
            ".d.......D..",
        ];
        let mut board = Board {
            n: 12,
            walls: vec![false; 144],
            nests: vec![None; 144],
            stacks: vec![Stack::default(); 144],
        };
        for (r, row) in rows.iter().enumerate() {
            for (c, symbol) in row.bytes().enumerate() {
                match symbol {
                    b'#' => board.walls[r * 12 + c] = true,
                    b'A'..=b'L' => board.nests[r * 12 + c] = Some(symbol - b'A'),
                    _ => {}
                }
            }
        }
        put_colors(&mut board, 60, &[0]);
        put_colors(&mut board, 61, &[0]);
        put_colors(&mut board, 70, &[1]);
        put_colors(&mut board, 85, &[0]);
        put_colors(&mut board, 88, &[3]);
        put_colors(&mut board, 95, &[1]);
        put_colors(&mut board, 96, &[0]);
        put_colors(&mut board, 99, &[3]);
        put_colors(&mut board, 108, &[3, 2, 2, 2, 2]);
        put_colors(&mut board, 116, &[1, 1]);
        put_colors(&mut board, 133, &[3]);
        assert_eq!(existing_mixed_colors(&board.stacks[108]), Some([2, 3]));
        let before = board.stacks.clone();
        let found = existing_mixed_search(
            &board,
            108,
            [2, 3],
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(found.invalid, 0);
        assert!(!found.plans.is_empty());
        assert!(found.plans.len() <= 8);
        assert!(found.registered <= 4000);
        for plan in found.plans {
            let mut actual = board.clone();
            for action in plan.prefix {
                assert!(checked_existing_action(&mut actual, action));
            }
            assert!(actual.stacks.iter().all(|s| s.len() <= 8));
        }
        assert_eq!(board.stacks, before);
    }
}
