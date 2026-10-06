use proconio::input;
use proconio::marker::Bytes;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::time::{Duration, Instant};

const DIRECTIONS: [(isize, isize, char); 4] =
    [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const MAX_OPERATIONS: usize = 100_000;
const MAX_HEIGHT: usize = 8;
const MAX_PAIR_STEPS: usize = 3;
const MAX_REPLANS: usize = 512;
const MAX_SIMPLE_PAIR_CANDIDATES: usize = 8;
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
const MAX_PICKUP_PARTNERS: usize = 4;
const MAX_PICKUP_THIRDS: usize = 3;
const MAX_PICKUP_APPROACH_STEPS: usize = 12;
const MAX_PICKUP_FINISH_STEPS: usize = 24;
const MAX_PICKUP_STATES: usize = 4000;
const MAX_PICKUP_ROLLOUTS: usize = 8;
const LOG_EVENTS: bool = false;

macro_rules! log_event {
    ($dst:expr, $($arg:tt)*) => {
        if LOG_EVENTS {
            let _ = writeln!($dst, $($arg)*);
        }
    };
}

const SEARCH_DEADLINE: Duration = Duration::from_millis(1950);

#[derive(Clone, Copy, Debug, Default)]
struct Stack {
    colors: [u8; MAX_HEIGHT], // Bottom to top in colors[..len].
    len: u8,
}

impl PartialEq for Stack {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.colors[..self.len()] == other.colors[..other.len()]
    }
}

impl Eq for Stack {}

impl Stack {
    #[inline]
    fn len(&self) -> usize {
        usize::from(self.len)
    }

    #[inline]
    fn last(&self) -> Option<u8> {
        (self.len > 0).then(|| self.colors[self.len() - 1])
    }

    fn push(&mut self, color: u8) {
        assert!(self.len() < MAX_HEIGHT);
        self.colors[self.len()] = color;
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.colors[self.len()])
    }

    fn top_run_len(&self) -> usize {
        let Some(color) = self.last() else {
            return 0;
        };
        let len = self.len();
        let mut count = 0;
        while count < len && self.colors[len - 1 - count] == color {
            count += 1;
        }
        count
    }
}

#[derive(Default)]
struct BfsBuffer {
    costs: Vec<u16>,
    seen: Vec<u32>,
    stamp: u32,
    queue: Vec<u16>,
}

thread_local! {
    static SCRATCH_BOARD: std::cell::RefCell<Board> = std::cell::RefCell::new(Board {
        n: 0,
        adj: Default::default(),
        walls: Vec::new(),
        nests: Vec::new(),
        stacks: Vec::new(),
    });
    static BFS_BUFFER: std::cell::RefCell<BfsBuffer> = std::cell::RefCell::new(BfsBuffer::default());
}

fn cell_hash(cell: usize, stack: &Stack) -> u64 {
    if stack.len == 0 {
        return 0;
    }
    let mut code = u64::from(stack.len);
    for &color in &stack.colors[..stack.len()] {
        code = code * 13 + u64::from(color) + 1;
    }
    let mut value = code ^ ((cell as u64) << 40) ^ 0x9E37_79B9_7F4A_7C15;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn board_hash(board: &Board) -> u64 {
    board
        .stacks
        .iter()
        .enumerate()
        .fold(0, |hash, (cell, stack)| hash ^ cell_hash(cell, stack))
}

// Apply an action and keep the board hash in sync.
fn apply_hashed(board: &mut Board, hash: &mut u64, action: Action) -> usize {
    let mut to = action.from;
    for _ in 0..action.length {
        to = board.adjacent(to, action.direction).unwrap();
    }
    *hash ^= cell_hash(action.from, &board.stacks[action.from]) ^ cell_hash(to, &board.stacks[to]);
    board.apply(action);
    *hash ^= cell_hash(action.from, &board.stacks[action.from]) ^ cell_hash(to, &board.stacks[to]);
    to
}

// Keys are already well-mixed board hashes.
#[derive(Default)]
struct IdentityHasher(u64);

impl std::hash::Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("IdentityHasher only hashes u64 keys");
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

// States visited by the incumbent plan. A rollout reaching one of them can
// finish with the incumbent's remaining units.
struct Reference {
    // Board hash at a unit boundary -> index of the next unit.
    index: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityHasher>>,
    units: Vec<Vec<Action>>,
    // Operations remaining from each unit boundary.
    remaining: Vec<usize>,
}

impl Reference {
    fn build(board: &Board, plan: &Plan) -> Self {
        let mut board = board.clone();
        let mut hash = board_hash(&board);
        let units: Vec<Vec<Action>> = plan.iter().cloned().collect();
        let mut remaining = vec![0; units.len() + 1];
        for j in (0..units.len()).rev() {
            remaining[j] = remaining[j + 1] + units[j].len();
        }
        let mut index = HashMap::with_capacity_and_hasher(units.len() + 1, Default::default());
        for (j, unit) in units.iter().enumerate() {
            index.insert(hash, j);
            for &action in unit {
                apply_hashed(&mut board, &mut hash, action);
            }
        }
        index.insert(hash, units.len());
        Self {
            index,
            units,
            remaining,
        }
    }
}

thread_local! {
    static REFERENCE: std::cell::RefCell<Option<Reference>> = const { std::cell::RefCell::new(None) };
}

fn set_reference(board: &Board, plan: &Plan) {
    let reference = Reference::build(board, plan);
    REFERENCE.with(|cell| *cell.borrow_mut() = Some(reference));
}

#[derive(Clone)]
struct Board {
    n: usize,
    // Neighbor table built by build_adjacency; empty means compute on demand.
    adj: std::rc::Rc<Vec<[u16; 4]>>,
    walls: Vec<bool>,
    nests: Vec<Option<u8>>,
    stacks: Vec<Stack>,
}

impl Board {
    fn build_adjacency(&mut self) {
        self.adj = std::rc::Rc::new(Vec::new());
        let table = (0..self.n * self.n)
            .map(|cell| {
                let mut row = [u16::MAX; 4];
                for (direction, slot) in row.iter_mut().enumerate() {
                    if let Some(next) = self.adjacent(cell, direction) {
                        *slot = next as u16;
                    }
                }
                row
            })
            .collect();
        self.adj = std::rc::Rc::new(table);
    }

    #[inline]
    fn adjacent(&self, cell: usize, direction: usize) -> Option<usize> {
        if let Some(row) = self.adj.get(cell) {
            let next = row[direction];
            return (next != u16::MAX).then_some(usize::from(next));
        }
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

    fn apply(&mut self, action: Action) -> usize {
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
        self.stacks[action.from].len = action.k as u8;
        for &color in jumping.iter().take(jumping_count) {
            self.stacks[to].push(color);
        }
        self.return_home(action.from);
        self.return_home(to);
        to
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AdjacentSpringboardPlan {
    actions: [Action; 2],
    source: usize,
    color: usize,
    landing: usize,
    gain: usize,
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

fn adjacent_springboard_plan(
    board: &Board,
    a_action: Action,
    distances: &[Vec<usize>],
) -> Option<AdjacentSpringboardPlan> {
    let a = a_action.from;
    let a_height = board.stacks[a].len();
    if a_height == 0 {
        return None;
    }
    let mut best = None;
    for direction_to_a in 0..DIRECTIONS.len() {
        let Some(b) = board.adjacent(a, direction_to_a) else {
            continue;
        };
        let b_stack = board.stacks[b];
        let Some(b_color) = b_stack.last() else {
            continue;
        };
        let b_color = usize::from(b_color);
        let b_height = b_stack.len();
        if b_stack.top_run_len() != b_height
            || b_color == board.stacks[a].last().map_or(b_color, usize::from)
            || a_height + b_height > MAX_HEIGHT
            || board.nests[a] == Some(b_color as u8)
        {
            continue;
        }
        let first = Action {
            from: b,
            k: 0,
            direction: direction_to_a ^ 1,
            length: 1,
        };
        let mut after_first = board.clone();
        if !checked_existing_action(&mut after_first, first)
            || after_first.stacks[a].len() != a_height + b_height
        {
            continue;
        }
        for direction in 0..DIRECTIONS.len() {
            let mut landing = a;
            for length in 1..=a_height + 1 {
                let Some(next) = board.adjacent(landing, direction) else {
                    break;
                };
                landing = next;
                if length < 2
                    || landing == b
                    || (board.stacks[landing].len() > 0
                        && board.nests[landing] != Some(b_color as u8))
                    || board.stacks[landing].len() + b_height > MAX_HEIGHT
                {
                    continue;
                }
                let second = Action {
                    from: a,
                    k: a_height,
                    direction,
                    length,
                };
                let mut replay = after_first.clone();
                if !checked_existing_action(&mut replay, second) {
                    continue;
                }
                if replay.stacks[a] != board.stacks[a]
                    || replay.stacks[b].len() != 0
                    || (board.nests[landing] == Some(b_color as u8)
                        && replay.stacks[landing] != board.stacks[landing])
                    || replay.stacks.iter().enumerate().any(|(cell, stack)| {
                        cell != a && cell != b && cell != landing && *stack != board.stacks[cell]
                    })
                {
                    continue;
                }
                let distance_before = distances[b_color][b];
                let distance_after = distances[b_color][landing];
                let gain = distance_before.saturating_sub(distance_after.saturating_add(2));
                if gain == 0 {
                    continue;
                }
                let plan = AdjacentSpringboardPlan {
                    actions: [first, second],
                    source: b,
                    color: b_color,
                    landing,
                    gain,
                };
                if best
                    .as_ref()
                    .is_none_or(|current: &AdjacentSpringboardPlan| {
                        (
                            plan.gain,
                            Reverse(plan.source),
                            Reverse(plan.actions[1].direction),
                            Reverse(plan.actions[1].length),
                        ) > (
                            current.gain,
                            Reverse(current.source),
                            Reverse(current.actions[1].direction),
                            Reverse(current.actions[1].length),
                        )
                    })
                {
                    best = Some(plan);
                }
            }
        }
    }
    best
}

// Packed choose_target priority: larger is chosen first, 0 means empty.
// Ties prefer the smaller cell, as in choose_target.
fn target_key(board: &Board, distances: &[Vec<usize>], cell: usize) -> u32 {
    let stack = &board.stacks[cell];
    let Some(color) = stack.last() else {
        return 0;
    };
    let cleanup_priority = if stack.len() == MAX_HEIGHT {
        2
    } else if stack.top_run_len() != stack.len() {
        1
    } else {
        0
    };
    let distance = distances[usize::from(color)][cell].min(0x3FFF) as u32;
    (cleanup_priority << 24) | (distance << 10) | (1023 - cell as u32)
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

const MAX_ALTERNATIVE_MOVES: usize = 16;
const MAX_MIXED_ALTERNATIVE_MOVES: usize = 10;

// Single moves of the top run (or part of it) that approach the nest,
// best distance gain first.
fn alternative_moves(board: &Board, cell: usize, distances: &[usize]) -> Vec<Action> {
    let stack = &board.stacks[cell];
    let height = stack.len();
    let current = distances[cell];
    let mut moves = Vec::new();
    for moving in 1..=stack.top_run_len() {
        let k = height - moving;
        for direction in 0..DIRECTIONS.len() {
            let mut to = cell;
            for length in 1..=k + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                if distances[to] >= current || board.stacks[to].len() + moving > MAX_HEIGHT {
                    continue;
                }
                let action = Action {
                    from: cell,
                    k,
                    direction,
                    length,
                };
                moves.push(((current - distances[to], moving), action));
            }
        }
    }
    moves.sort_by(|a, b| b.0.cmp(&a.0));
    let mut result: Vec<Action> = moves
        .into_iter()
        .take(MAX_ALTERNATIVE_MOVES)
        .map(|(_, action)| action)
        .collect();
    // Mixed stacks: also carry lower slimes along with the top run.
    let mut mixed = Vec::new();
    for moving in stack.top_run_len() + 1..=height {
        let k = height - moving;
        for direction in 0..DIRECTIONS.len() {
            let mut to = cell;
            for length in 1..=k + 1 {
                let Some(next) = board.adjacent(to, direction) else {
                    break;
                };
                to = next;
                if distances[to] >= current || board.stacks[to].len() + moving > MAX_HEIGHT {
                    continue;
                }
                let action = Action {
                    from: cell,
                    k,
                    direction,
                    length,
                };
                mixed.push(((current - distances[to], Reverse(moving)), action));
            }
        }
    }
    mixed.sort_by(|a, b| b.0.cmp(&a.0));
    result.extend(
        mixed
            .into_iter()
            .take(MAX_MIXED_ALTERNATIVE_MOVES)
            .map(|(_, action)| action),
    );
    result
}

const MAX_INCOMING_MOVES: usize = 6;
// Whole mixed stacks travel together only on sparse boards.
static USE_BUS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const BUS_MAX_SLIMES: usize = 90;
const RANDOM_STACK_CANDIDATES: usize = 4;

// Moves of other stacks' top runs onto the target cell, preferring groups
// whose own nest distance does not grow by riding along.
fn incoming_moves(board: &Board, target: usize, distances: &[Vec<usize>]) -> Vec<Action> {
    let target_height = board.stacks[target].len();
    let mut moves = Vec::new();
    for direction in 0..DIRECTIONS.len() {
        let mut from = target;
        for length in 1..MAX_HEIGHT {
            let Some(next) = board.adjacent(from, direction) else {
                break;
            };
            from = next;
            let stack = &board.stacks[from];
            let Some(color) = stack.last() else {
                continue;
            };
            let distance = &distances[usize::from(color)];
            let run = stack.top_run_len();
            for moving in 1..=run {
                let k = stack.len() - moving;
                if length > k + 1 || target_height + moving > MAX_HEIGHT {
                    continue;
                }
                let gain = distance[from] as isize - distance[target] as isize;
                moves.push((
                    (gain, moving),
                    Action {
                        from,
                        k,
                        direction: direction ^ 1,
                        length,
                    },
                ));
            }
        }
    }
    moves.sort_by(|a, b| b.0.cmp(&a.0));
    moves
        .into_iter()
        .filter(|((gain, _), _)| *gain >= 0)
        .take(MAX_INCOMING_MOVES)
        .map(|(_, action)| action)
        .collect()
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
    let (ordinary_end, ordinary_end_top) = SCRATCH_BOARD.with(|scratch| {
        let mut after_first = scratch.borrow_mut();
        // Boards sharing a built neighbor table also share walls and nests.
        if board.adj.is_empty() || !std::rc::Rc::ptr_eq(&after_first.adj, &board.adj) {
            after_first.n = board.n;
            after_first.adj.clone_from(&board.adj);
            after_first.walls.clone_from(&board.walls);
            after_first.nests.clone_from(&board.nests);
        }
        after_first.stacks.clone_from(&board.stacks);
        after_first.apply(ordinary);
        let (_, ordinary_end) = choose_group_move(&after_first, ordinary_to, color, distances)?;
        Some((ordinary_end, after_first.stacks[ordinary_end].last()))
    })?;
    if ordinary_end_top == Some(color as u8) {
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
    transport_cost_within(board, start, color, moving, removed, usize::MAX)
}

// Same as transport_cost, but gives up beyond `max_cost`.
fn transport_cost_within(
    board: &Board,
    start: usize,
    color: usize,
    moving: usize,
    removed: &[usize],
    max_cost: usize,
) -> Option<usize> {
    let height = |cell| {
        if removed.contains(&cell) {
            0
        } else {
            board.stacks[cell].len()
        }
    };
    BFS_BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        let BfsBuffer {
            costs,
            seen,
            stamp,
            queue,
        } = &mut *buffer;
        if seen.len() != board.stacks.len() {
            seen.clear();
            seen.resize(board.stacks.len(), 0);
            costs.resize(board.stacks.len(), 0);
            *stamp = 0;
        }
        *stamp = stamp.wrapping_add(1);
        if *stamp == 0 {
            seen.fill(0);
            *stamp = 1;
        }
        let stamp = *stamp;
        queue.clear();
        queue.push(start as u16);
        costs[start] = 0;
        seen[start] = stamp;
        let mut head = 0;
        while head < queue.len() {
            let cell = usize::from(queue[head]);
            head += 1;
            if board.nests[cell] == Some(color as u8) {
                return Some(costs[cell] as usize);
            }
            let next_cost = costs[cell] + 1;
            if usize::from(next_cost) > max_cost {
                continue;
            }
            for direction in 0..DIRECTIONS.len() {
                let mut to = cell;
                for _ in 0..=height(cell) {
                    let Some(next) = board.adjacent(to, direction) else {
                        break;
                    };
                    to = next;
                    if seen[to] != stamp && height(to) + moving <= MAX_HEIGHT {
                        seen[to] = stamp;
                        costs[to] = next_cost;
                        queue.push(to as u16);
                    }
                }
            }
        }
        None
    })
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
    // Cells within MAX_PAIR_STEPS: (cell, steps, previous action, partner).
    // Partners are recorded but never expanded.
    let mut visited: Vec<(usize, usize, Option<Action>, bool)> = Vec::with_capacity(32);
    let mut candidates = Vec::new();
    visited.push((cell, 0, None, false));
    let mut head = 0;
    while head < visited.len() {
        let (from, from_steps, _, partner) = visited[head];
        head += 1;
        if partner || from_steps == MAX_PAIR_STEPS {
            continue;
        }
        for direction in 0..DIRECTIONS.len() {
            let Some(to) = board.adjacent(from, direction) else {
                continue;
            };
            if visited.iter().any(|&(seen, ..)| seen == to) || board.nests[to] == Some(color as u8)
            {
                continue;
            }
            let target = &board.stacks[to];
            let is_partner = target.len() == 1 && target.last() == Some(color as u8);
            if target.len() > 0 && !is_partner {
                continue;
            }
            let action = Action {
                from,
                k: 0,
                direction,
                length: 1,
            };
            visited.push((to, from_steps + 1, Some(action), is_partner));
            if is_partner {
                candidates.push(to);
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let entry = |target: usize| *visited.iter().find(|&&(seen, ..)| seen == target).unwrap();
    let source_cost = transport_cost(board, cell, color, 1, &[cell])?;
    let support_loss = lost_singleton_support(board, cell, distances);
    let mut best = None;
    for partner in candidates {
        let Some(partner_cost) = transport_cost(board, partner, color, 1, &[partner]) else {
            continue;
        };
        let separate_cost = source_cost + partner_cost;
        let fixed_cost = entry(partner).1 + support_loss;
        if fixed_cost >= separate_cost {
            continue;
        }
        let Some(pair_cost) = transport_cost_within(
            board,
            partner,
            color,
            2,
            &[cell, partner],
            separate_cost - fixed_cost - 1,
        ) else {
            continue;
        };
        let merged_cost = fixed_cost + pair_cost;
        if merged_cost >= separate_cost {
            continue;
        }
        let priority = (
            separate_cost - merged_cost,
            MAX_PAIR_STEPS - entry(partner).1,
        );
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
        let action = entry(to).2?;
        actions.push(action);
        to = action.from;
    }
    actions.reverse();
    Some(actions)
}

// A legacy decision is one unit: a short pair, a relay, or one ordinary move.
// Budget checks belong to the caller and never change this choice.
// Move a whole mixed stack one cell when that brings every color closer.
fn bus_move(board: &Board, cell: usize, distances: &[Vec<usize>]) -> Option<Action> {
    let stack = &board.stacks[cell];
    if stack.top_run_len() == stack.len() {
        return None;
    }
    let colors = &stack.colors[..stack.len()];
    for direction in 0..DIRECTIONS.len() {
        let Some(to) = board.adjacent(cell, direction) else {
            continue;
        };
        if board.stacks[to].len() > 0 {
            continue;
        }
        if colors
            .iter()
            .all(|&color| distances[usize::from(color)][to] < distances[usize::from(color)][cell])
        {
            return Some(Action {
                from: cell,
                k: 0,
                direction,
                length: 1,
            });
        }
    }
    None
}

fn legacy_unit(
    board: &Board,
    cell: usize,
    color: usize,
    distances: &[Vec<usize>],
) -> Option<Vec<Action>> {
    if USE_BUS.load(std::sync::atomic::Ordering::Relaxed)
        && let Some(action) = bus_move(board, cell, distances)
    {
        return Some(vec![action]);
    }
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
    legacy_rollout_owned(initial.clone(), distances, limit, deadline, incumbent_bound)
}

fn legacy_rollout_owned(
    mut board: Board,
    distances: &[Vec<usize>],
    limit: usize,
    deadline: Option<Instant>,
    incumbent_bound: Option<usize>,
) -> Result<Plan, RolloutStop> {
    if incumbent_bound == Some(0) {
        return Err(RolloutStop::Incumbent);
    }
    let mut plan = Plan::new();
    let mut count = 0;
    let mut keys: Vec<u32> = (0..board.stacks.len())
        .map(|cell| target_key(&board, distances, cell))
        .collect();
    let mut steps = 0usize;
    let use_reference = REFERENCE.with(|cell| cell.borrow().is_some());
    let mut hash = if use_reference { board_hash(&board) } else { 0 };
    loop {
        if use_reference {
            let hit = REFERENCE.with(|cell| {
                let reference = cell.borrow();
                let reference = reference.as_ref().unwrap();
                let &j = reference.index.get(&hash)?;
                let total = count + reference.remaining[j];
                if incumbent_bound.is_some_and(|bound| total >= bound) {
                    return Some(Err(RolloutStop::Incumbent));
                }
                if total > limit {
                    return Some(Err(RolloutStop::OperationLimit));
                }
                Some(Ok(reference.units[j..].to_vec()))
            });
            match hit {
                Some(Ok(tail)) => {
                    plan.extend(tail);
                    return Ok(plan);
                }
                Some(Err(stop)) => return Err(stop),
                None => {}
            }
        }
        let key = keys.iter().copied().max().unwrap_or(0);
        if key == 0 {
            break;
        }
        let cell = 1023 - (key & 1023) as usize;
        let color = usize::from(board.stacks[cell].last().unwrap());
        if count >= limit {
            return Err(RolloutStop::OperationLimit);
        }
        steps += 1;
        if steps % 8 == 1 && deadline.is_some_and(|time| Instant::now() >= time) {
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
            let to = if use_reference {
                apply_hashed(&mut board, &mut hash, action)
            } else {
                board.apply(action)
            };
            keys[action.from] = target_key(&board, distances, action.from);
            keys[to] = target_key(&board, distances, to);
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
        legacy_rollout_owned(
            after,
            distances,
            remaining - prefix.len(),
            deadline,
            Some(saved_length - prefix.len()),
        )
    };
    stats.record(&result);
    result
}

// Try only adjacent homogeneous towers. Carry both colors along the first
// common shortest-path direction, and keep merge + transport as one unit.
fn simple_pair_candidates(board: &Board, distances: &[Vec<usize>]) -> Vec<Vec<Action>> {
    let mut candidates = Vec::new();
    for source in 0..board.stacks.len() {
        let source_stack = &board.stacks[source];
        if source_stack.len() == 0 || source_stack.top_run_len() != source_stack.len() {
            continue;
        }
        let source_color = usize::from(source_stack.colors[0]);
        for direction in 0..DIRECTIONS.len() {
            let Some(partner) = board.adjacent(source, direction) else {
                continue;
            };
            let partner_stack = &board.stacks[partner];
            if partner_stack.len() == 0
                || partner_stack.top_run_len() != partner_stack.len()
                || source_stack.len() + partner_stack.len() > MAX_HEIGHT
            {
                continue;
            }
            let partner_color = usize::from(partner_stack.colors[0]);
            if source_color == partner_color {
                continue;
            }
            let merge = Action {
                from: source,
                k: 0,
                direction,
                length: 1,
            };
            let mut after = board.clone();
            after.apply(merge);
            let mut cell = partner;
            let mut prefix = vec![merge];
            let height = source_stack.len() + partner_stack.len();
            // Homecoming may remove one color during the merge or transport.
            // Stop as soon as the two-color group no longer exists intact.
            while after.stacks[cell].len() == height {
                let next_move = (0..DIRECTIONS.len()).find_map(|direction| {
                    let next = after.adjacent(cell, direction)?;
                    (after.stacks[next].len() == 0
                        && distances[source_color][next] < distances[source_color][cell]
                        && distances[partner_color][next] < distances[partner_color][cell])
                        .then_some((direction, next))
                });
                let Some((direction, next)) = next_move else {
                    break;
                };
                let action = Action {
                    from: cell,
                    k: 0,
                    direction,
                    length: 1,
                };
                after.apply(action);
                prefix.push(action);
                cell = next;
            }
            if prefix.len() > 1 {
                candidates.push(prefix);
                if candidates.len() == MAX_SIMPLE_PAIR_CANDIDATES {
                    return candidates;
                }
            }
        }
    }
    candidates
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
const MAX_EXISTING_MIXED_HOME_TABLES: usize = 8;
const MAX_EXISTING_MIXED_CALLS: usize = 64;
const EXISTING_MIXED_CALL_TIME: Duration = Duration::from_millis(10);
const EXISTING_MIXED_CASE_TIME: Duration = Duration::from_millis(60);
const EXISTING_MIXED_RESTART_TIME: Duration = Duration::from_millis(15);

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

#[derive(Clone)]
struct ExistingMixedEdge {
    state: ExistingMixedState,
    actions: Vec<Action>,
    split: bool,
    sender: bool,
}

#[derive(Default)]
struct SenderStats {
    requests: usize,
    bfs: usize,
    expanded: usize,
    found: usize,
    steps: usize,
    lengths: [usize; 25],
    invalid: usize,
    timed_out: usize,
    depth_hits: usize,
    registered: usize,
    selected: usize,
    elapsed: Duration,
}

impl SenderStats {
    fn add(&mut self, other: &Self) {
        self.requests += other.requests;
        self.bfs += other.bfs;
        self.expanded += other.expanded;
        self.found += other.found;
        self.steps += other.steps;
        for (a, b) in self.lengths.iter_mut().zip(other.lengths) {
            *a += b;
        }
        self.invalid += other.invalid;
        self.timed_out += other.timed_out;
        self.depth_hits += other.depth_hits;
        self.registered += other.registered;
        self.selected += other.selected;
        self.elapsed += other.elapsed;
    }
}

impl ExistingMixedModel {
    fn sender_edge(
        &self,
        state: ExistingMixedState,
        index: usize,
        max_steps: usize,
        deadline: Instant,
        stats: &mut SenderStats,
    ) -> Option<ExistingMixedEdge> {
        stats.requests += 1;
        let sender = self.pickups[index];
        let group = self.stack(sender, state.collected);
        if state.colors.is_single_color()
            || state.collected & (1 << index) != 0
            || sender == state.cell
            || group.len() == 0
            || self.stack(state.cell, state.collected).len()
                + usize::from(state.colors.len)
                + group.len()
                > MAX_HEIGHT
            || max_steps == 0
        {
            return None;
        }
        let started = Instant::now();
        let result = self.sender_path(state, index, max_steps, deadline, stats);
        stats.elapsed += started.elapsed();
        result
    }

    fn sender_path(
        &self,
        state: ExistingMixedState,
        index: usize,
        max_steps: usize,
        deadline: Instant,
        stats: &mut SenderStats,
    ) -> Option<ExistingMixedEdge> {
        stats.bfs += 1;
        let sender = self.pickups[index];
        let group = self.stack(sender, state.collected);
        let q = group.len();
        let color = group.last().unwrap();
        let before = self.board_at(state);
        let mut background = before.clone();
        background.stacks[sender] = Stack::default();
        let mut distances = vec![usize::MAX; background.stacks.len()];
        let mut previous = vec![None; background.stacks.len()];
        let mut queue = VecDeque::from([sender]);
        distances[sender] = 0;
        let mut arrival = None;
        'bfs: while let Some(from) = queue.pop_front() {
            if Instant::now() >= deadline {
                stats.timed_out += 1;
                return None;
            }
            if distances[from] >= max_steps {
                stats.depth_hits += 1;
                continue;
            }
            stats.expanded += 1;
            let support = background.stacks[from].len();
            for direction in 0..DIRECTIONS.len() {
                let mut to = from;
                for length in 1..=support + 1 {
                    let Some(next) = background.adjacent(to, direction) else {
                        break;
                    };
                    to = next;
                    let target = background.stacks[to];
                    if target.len() + q > MAX_HEIGHT {
                        continue;
                    }
                    if to != state.cell
                        && (target.top_run_len() != target.len()
                            || background.nests[to] == Some(color))
                    {
                        continue;
                    }
                    if distances[to] != usize::MAX {
                        continue;
                    }
                    distances[to] = distances[from] + 1;
                    previous[to] = Some((
                        from,
                        Action {
                            from,
                            k: support,
                            direction,
                            length,
                        },
                    ));
                    if to == state.cell {
                        arrival = Some(to);
                        break 'bfs;
                    }
                    queue.push_back(to);
                }
            }
        }
        let mut at = arrival?;
        let mut actions = Vec::with_capacity(distances[at]);
        while let Some((prior, action)) = previous[at] {
            actions.push(action);
            at = prior;
        }
        actions.reverse();
        // Arrival appends a homogeneous sender; it never reverses the receiver.
        let bit = u16::from(color == self.colors[1]);
        let joined = TwoColorStack {
            bits: state.colors.bits
                | ((if bit == 1 {
                    TwoColorStack::mask(q as u8)
                } else {
                    0
                }) << state.colors.len),
            len: state.colors.len + q as u8,
        }
        .after_home(
            self.background.nests[state.cell],
            self.colors[0],
            self.colors[1],
        );
        let end = ExistingMixedState {
            cell: state.cell,
            colors: joined,
            collected: state.collected | (1 << index),
        };
        let mut actual = before;
        for &action in &actions {
            if Instant::now() >= deadline {
                stats.timed_out += 1;
                return None;
            }
            if !checked_existing_action(&mut actual, action) {
                stats.invalid += 1;
                return None;
            }
        }
        if actual.stacks != self.board_at(end).stacks {
            stats.invalid += 1;
            return None;
        }
        stats.found += 1;
        stats.steps += actions.len();
        stats.lengths[actions.len()] += 1;
        Some(ExistingMixedEdge {
            state: end,
            actions,
            split: false,
            sender: true,
        })
    }
}

struct ExistingMixedNode {
    state: ExistingMixedState,
    depth: usize,
    previous: Option<(usize, Vec<Action>)>,
    split: bool,
    sender_merges: usize,
    settled: bool,
    ticket: usize,
}

// Positive integer costs bound each state's strict improvements to at most 24.
// Hence the heap (including stale items) is bounded by 4000 * 25 entries.
struct ExistingMixedFrontier {
    nodes: Vec<ExistingMixedNode>,
    indices: HashMap<ExistingMixedState, usize>,
    queue: BinaryHeap<Reverse<(usize, usize, usize)>>,
    next_ticket: usize,
    capacity: usize,
}

impl ExistingMixedFrontier {
    fn new(initial: ExistingMixedState, capacity: usize) -> Self {
        Self {
            nodes: vec![ExistingMixedNode {
                state: initial,
                depth: 0,
                previous: None,
                split: false,
                sender_merges: 0,
                settled: false,
                ticket: 0,
            }],
            indices: HashMap::from([(initial, 0)]),
            queue: BinaryHeap::from([Reverse((0, 0, 0))]),
            next_ticket: 1,
            capacity,
        }
    }
    fn pop(&mut self, result: &mut ExistingMixedSearch) -> Option<usize> {
        while let Some(Reverse((depth, ticket, index))) = self.queue.pop() {
            let node = &mut self.nodes[index];
            if node.settled || node.depth != depth || node.ticket != ticket {
                result.stale += 1;
                continue;
            }
            node.settled = true;
            result.settled += 1;
            return Some(index);
        }
        None
    }
    fn relax(&mut self, parent: usize, edge: ExistingMixedEdge, result: &mut ExistingMixedSearch) {
        assert!(self.nodes[parent].settled);
        let depth = self.nodes[parent].depth + edge.actions.len();
        if depth > MAX_EXISTING_MIXED_DEPTH {
            result.depth_limit = true;
            return;
        }
        let index = if let Some(&index) = self.indices.get(&edge.state) {
            if self.nodes[index].settled || self.nodes[index].depth <= depth {
                return;
            }
            result.updates += 1;
            index
        } else {
            if self.nodes.len() >= self.capacity {
                result.state_limit = true;
                return;
            }
            let index = self.nodes.len();
            self.indices.insert(edge.state, index);
            self.nodes.push(ExistingMixedNode {
                state: edge.state,
                depth,
                previous: None,
                split: false,
                sender_merges: 0,
                settled: false,
                ticket: 0,
            });
            result.registered += 1;
            index
        };
        if edge.sender {
            result.sender.registered += 1;
        }
        let sender_merges = self.nodes[parent].sender_merges + usize::from(edge.sender);
        let node = &mut self.nodes[index];
        node.depth = depth;
        node.previous = Some((parent, edge.actions));
        node.split = edge.split;
        node.sender_merges = sender_merges;
        node.ticket = self.next_ticket;
        self.queue.push(Reverse((depth, self.next_ticket, index)));
        self.next_ticket += 1;
        result.queue_peak = result.queue_peak.max(self.queue.len());
    }
    fn path(&self, mut index: usize) -> Vec<Action> {
        let mut chunks = Vec::new();
        while let Some((parent, actions)) = &self.nodes[index].previous {
            chunks.push(actions.as_slice());
            index = *parent;
        }
        chunks.into_iter().rev().flatten().copied().collect()
    }
}

struct ExistingMixedPlan {
    prefix: Vec<Action>,
    collected: u8,
    end: ExistingMixedState,
    split: bool,
    returned: [usize; 2],
    sender_merges: usize,
}

struct ExistingMixedTerminalInfo {
    state: ExistingMixedState,
    depth: usize,
    returned: [usize; 2],
    split: bool,
    sender_merges: usize,
}

#[derive(Default)]
struct ExistingMixedSearch {
    plans: Vec<ExistingMixedPlan>,
    terminal_info: Vec<ExistingMixedTerminalInfo>,
    sender: SenderStats,
    updates: usize,
    stale: usize,
    settled: usize,
    queue_peak: usize,
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
    home_tables: usize,
    home_unreachable: usize,
    home_fallback_groups: usize,
    home_elapsed: Duration,
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

// A terminal leaves either no moving slime or one homogeneous moving group.
// This is the old ranking, retained as the fallback when a bounded home-cost
// calculation cannot complete before the local deadline.
fn existing_mixed_terminal_nest_distance(
    plan: &ExistingMixedPlan,
    colors: [u8; 2],
    distances: &[Vec<usize>],
) -> usize {
    let moving = plan.end.colors;
    if moving.len == 0 {
        return 0;
    }
    assert!(moving.is_single_color());
    let color = colors[usize::from(moving.bits != 0)];
    distances[usize::from(color)][plan.end.cell]
}

fn rank_existing_mixed_terminal_indices(
    plans: &[ExistingMixedPlan],
    colors: [u8; 2],
    distances: &[Vec<usize>],
) -> Vec<usize> {
    let mut order: Vec<_> = (0..plans.len()).collect();
    order.sort_unstable_by_key(|&index| {
        (
            plans[index].prefix.len(),
            existing_mixed_terminal_nest_distance(&plans[index], colors, distances),
            index,
        )
    });
    order
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ExistingMixedHomeKey {
    collected: u8,
    returned: [usize; 2],
    color: u8,
    moving: u8,
}

#[derive(Default)]
struct ExistingMixedHomeStats {
    tables: usize,
    unreachable: usize,
    fallback_groups: usize,
    elapsed: Duration,
}

fn existing_mixed_home_key(plan: &ExistingMixedPlan, colors: [u8; 2]) -> ExistingMixedHomeKey {
    let moving = plan.end.colors;
    let color = if moving.len == 0 {
        u8::MAX
    } else {
        colors[usize::from(moving.bits != 0)]
    };
    ExistingMixedHomeKey {
        collected: plan.collected,
        returned: plan.returned,
        color,
        moving: moving.len,
    }
}

fn terminal_transport_background(model: &ExistingMixedModel, plan: &ExistingMixedPlan) -> Board {
    let mut background = model.board_at(plan.end);
    background.stacks[plan.end.cell] = Stack::default();
    background
}

// Build the directed movement graph first, then traverse its reverse edges.
// An edge into an empty matching nest reaches the all-returned goal.  The
// source support height determines the forward action, so reversing actions
// directly would be incorrect.
fn transport_home_costs(
    background: &Board,
    color: u8,
    moving: usize,
    deadline: Instant,
) -> Option<Vec<usize>> {
    let started = Instant::now();
    let mut reverse = vec![Vec::new(); background.stacks.len()];
    let mut goal = vec![false; background.stacks.len()];
    for (from, can_return) in goal.iter_mut().enumerate() {
        if background.walls[from] {
            continue;
        }
        if Instant::now() >= deadline {
            return None;
        }
        let support = background.stacks[from].len();
        for direction in 0..DIRECTIONS.len() {
            let mut to = from;
            for _ in 1..=support + 1 {
                let Some(next) = background.adjacent(to, direction) else {
                    break;
                };
                to = next;
                let target = background.stacks[to];
                if target.len() + moving > MAX_HEIGHT {
                    continue;
                }
                if background.nests[to] == Some(color) && target.len() == 0 {
                    *can_return = true;
                } else if !(background.nests[to] == Some(color) && target.last() == Some(color)) {
                    // Landing on matching background slime would also remove
                    // that background, which this fixed-background model forbids.
                    reverse[to].push(from);
                }
            }
        }
    }
    let mut costs = vec![usize::MAX; background.stacks.len()];
    let mut queue = VecDeque::new();
    for (cell, &can_return) in goal.iter().enumerate() {
        if can_return {
            costs[cell] = 1;
            queue.push_back(cell);
        }
    }
    while let Some(to) = queue.pop_front() {
        if Instant::now() >= deadline {
            return None;
        }
        for &from in &reverse[to] {
            if costs[from] == usize::MAX {
                costs[from] = costs[to] + 1;
                queue.push_back(from);
            }
        }
    }
    let _ = started;
    Some(costs)
}

fn rank_existing_mixed_terminal_home_cost_indices(
    plans: &[ExistingMixedPlan],
    model: &ExistingMixedModel,
    colors: [u8; 2],
    distances: &[Vec<usize>],
    deadline: Instant,
) -> (Vec<usize>, ExistingMixedHomeStats) {
    let started = Instant::now();
    let old_order = rank_existing_mixed_terminal_indices(plans, colors, distances);
    let mut order = old_order.clone();
    let mut groups: HashMap<ExistingMixedHomeKey, Vec<usize>> = HashMap::new();
    for &index in &old_order {
        groups
            .entry(existing_mixed_home_key(&plans[index], colors))
            .or_default()
            .push(index);
    }
    let mut tables = HashMap::new();
    let mut stats = ExistingMixedHomeStats::default();
    for group in groups.values() {
        if group.len() < 2 {
            continue;
        }
        let key = existing_mixed_home_key(&plans[group[0]], colors);
        let costs = if key.moving == 0 {
            None
        } else if let Some(costs) = tables.get(&key) {
            Some(costs)
        } else if tables.len() >= MAX_EXISTING_MIXED_HOME_TABLES || Instant::now() >= deadline {
            stats.fallback_groups += 1;
            continue;
        } else {
            let background = terminal_transport_background(model, &plans[group[0]]);
            debug_assert!(group.iter().all(|&index| {
                terminal_transport_background(model, &plans[index]).stacks == background.stacks
            }));
            let Some(costs) =
                transport_home_costs(&background, key.color, usize::from(key.moving), deadline)
            else {
                stats.fallback_groups += 1;
                continue;
            };
            stats.tables += 1;
            tables.insert(key, costs);
            tables.get(&key)
        };
        let mut ranked = group.clone();
        ranked.sort_unstable_by_key(|&index| {
            let h = costs.map_or(0, |costs| costs[plans[index].end.cell]);
            (
                h == usize::MAX,
                plans[index].prefix.len().saturating_add(h),
                plans[index].prefix.len(),
                old_order.iter().position(|&old| old == index).unwrap(),
            )
        });
        stats.unreachable += group
            .iter()
            .filter(|&&index| costs.is_some_and(|costs| costs[plans[index].end.cell] == usize::MAX))
            .count();
        for (&old, &new) in group.iter().zip(ranked.iter()) {
            let position = old_order.iter().position(|&index| index == old).unwrap();
            order[position] = new;
        }
    }
    stats.elapsed = started.elapsed();
    (order, stats)
}

#[cfg(test)]
fn select_existing_mixed_terminal_indices(
    plans: &[ExistingMixedPlan],
    colors: [u8; 2],
    distances: &[Vec<usize>],
) -> Vec<usize> {
    let ranked = rank_existing_mixed_terminal_indices(plans, colors, distances);
    let mut selected = vec![false; plans.len()];
    let mut represented = [false; 8];
    let mut order = Vec::new();
    for &index in &ranked {
        let mask = usize::from(plans[index].collected);
        if !represented[mask] && order.len() < MAX_EXISTING_MIXED_ROLLOUTS {
            represented[mask] = true;
            selected[index] = true;
            order.push(index);
        }
    }
    for &index in &ranked {
        if order.len() >= MAX_EXISTING_MIXED_ROLLOUTS {
            break;
        }
        if !selected[index] {
            order.push(index);
        }
    }
    order
}

fn select_existing_mixed_terminal_home_cost_indices(
    plans: &[ExistingMixedPlan],
    model: &ExistingMixedModel,
    colors: [u8; 2],
    distances: &[Vec<usize>],
    deadline: Instant,
) -> (Vec<usize>, ExistingMixedHomeStats) {
    let (ranked, stats) =
        rank_existing_mixed_terminal_home_cost_indices(plans, model, colors, distances, deadline);
    let mut selected = vec![false; plans.len()];
    let mut represented = [false; 8];
    let mut order = Vec::new();
    for &index in &ranked {
        let mask = usize::from(plans[index].collected);
        if !represented[mask] && order.len() < MAX_EXISTING_MIXED_ROLLOUTS {
            represented[mask] = true;
            selected[index] = true;
            order.push(index);
        }
    }
    for &index in &ranked {
        if order.len() >= MAX_EXISTING_MIXED_ROLLOUTS {
            break;
        }
        if !selected[index] {
            order.push(index);
        }
    }
    (order, stats)
}

fn existing_mixed_search(
    board: &Board,
    source: usize,
    colors: [u8; 2],
    distances: &[Vec<usize>],
    deadline: Instant,
) -> ExistingMixedSearch {
    let (model, initial) = ExistingMixedModel::new(board, source, colors);
    let mut result = ExistingMixedSearch {
        pickups: model.pickups.clone(),
        pickup_limit: model.pickup_limit,
        registered: 1,
        ..Default::default()
    };
    let mut frontier = ExistingMixedFrontier::new(initial, MAX_EXISTING_MIXED_STATES);
    let mut terminal_boards = HashSet::new();
    let mut terminals: Vec<ExistingMixedPlan> = Vec::new();
    'search: while !frontier.queue.is_empty() {
        if Instant::now() >= deadline {
            result.timed_out = true;
            break;
        }
        let Some(index) = frontier.pop(&mut result) else {
            break;
        };
        let state = frontier.nodes[index].state;
        let depth = frontier.nodes[index].depth;
        if model.terminal(state) {
            result.terminals[usize::from(state.collected)] += 1;
            if terminals.len() >= MAX_EXISTING_MIXED_STATES {
                result.terminal_limit = true;
                continue;
            }
            let path = frontier.path(index);
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
            result.terminal_info.push(ExistingMixedTerminalInfo {
                state,
                depth,
                returned,
                split: frontier.nodes[index].split,
                sender_merges: frontier.nodes[index].sender_merges,
            });
            terminals.push(ExistingMixedPlan {
                prefix: path,
                collected: state.collected,
                end: state,
                split: frontier.nodes[index].split,
                returned,
                sender_merges: frontier.nodes[index].sender_merges,
            });
        } else {
            if depth >= MAX_EXISTING_MIXED_DEPTH {
                result.depth_limit = true;
                continue;
            }
            result.expanded += 1;
            for transition in model.transitions(state) {
                frontier.relax(
                    index,
                    ExistingMixedEdge {
                        state: transition.state,
                        actions: vec![transition.action],
                        split: transition.split,
                        sender: false,
                    },
                    &mut result,
                );
            }
            for sender in 0..model.pickups.len() {
                if Instant::now() >= deadline {
                    result.timed_out = true;
                    break 'search;
                }
                if let Some(edge) = model.sender_edge(
                    state,
                    sender,
                    MAX_EXISTING_MIXED_DEPTH - depth,
                    deadline,
                    &mut result.sender,
                ) {
                    frontier.relax(index, edge, &mut result);
                }
                if Instant::now() >= deadline {
                    result.timed_out = true;
                    break 'search;
                }
            }
        }
    }
    // First prefer fewer interval operations, then a closer remaining group,
    // then the original settled order.  Representatives and remaining slots
    // deliberately use the same ranking.
    result.rollout_limit = terminals.len() > MAX_EXISTING_MIXED_ROLLOUTS;
    let (order, home_stats) = select_existing_mixed_terminal_home_cost_indices(
        &terminals, &model, colors, distances, deadline,
    );
    result.home_tables = home_stats.tables;
    result.home_unreachable = home_stats.unreachable;
    result.home_fallback_groups = home_stats.fallback_groups;
    result.home_elapsed = home_stats.elapsed;
    let mut terminals: Vec<_> = terminals.into_iter().map(Some).collect();
    result.sender.selected = order
        .iter()
        .map(|&i| terminals[i].as_ref().unwrap().sender_merges)
        .sum();
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
    sender: SenderStats,
    weighted_updates: usize,
    weighted_stale: usize,
    weighted_settled: usize,
    weighted_queue_peak: usize,
    sender_candidates: usize,
    sender_saved: usize,
    sender_executed: usize,
    home_tables: usize,
    home_unreachable: usize,
    home_fallback_groups: usize,
    home_elapsed: Duration,
}

impl ExistingMixedStats {
    fn record_search(&mut self, result: &ExistingMixedSearch) {
        self.sender.add(&result.sender);
        self.weighted_updates += result.updates;
        self.weighted_stale += result.stale;
        self.weighted_settled += result.settled;
        self.weighted_queue_peak = self.weighted_queue_peak.max(result.queue_peak);
        self.sender_candidates += result.plans.iter().filter(|p| p.sender_merges > 0).count();
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
        self.home_tables += result.home_tables;
        self.home_unreachable += result.home_unreachable;
        self.home_fallback_groups += result.home_fallback_groups;
        self.home_elapsed += result.home_elapsed;
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
        eprintln!(
            "existing_mixed_home tables={} unreachable={} fallback_groups={} time_us={}",
            self.home_tables,
            self.home_unreachable,
            self.home_fallback_groups,
            self.home_elapsed.as_micros()
        );
        eprintln!(
            "existing_sender requests={} bfs={} expanded={} cache_hits=0 found={} path_steps={} invalid={} timed_out={} depth_hits={} registered={} selected_merges={} selected_candidates={} saved={} executed={} time_us={} updates={} stale={} settled={} queue_peak={}",
            self.sender.requests,
            self.sender.bfs,
            self.sender.expanded,
            self.sender.found,
            self.sender.steps,
            self.sender.invalid,
            self.sender.timed_out,
            self.sender.depth_hits,
            self.sender.registered,
            self.sender.selected,
            self.sender_candidates,
            self.sender_saved,
            self.sender_executed,
            self.sender.elapsed.as_micros(),
            self.weighted_updates,
            self.weighted_stale,
            self.weighted_settled,
            self.weighted_queue_peak
        );
        eprintln!("existing_sender_lengths counts={:?}", self.sender.lengths);
        self.rollout.log("existing_mixed");
        eprint!("{}", self.events);
    }
}

// Restart the whole search with other target sampling seeds while time
// remains, and output the shortest answer. The first run is the original one.
const RESTART_MARGIN: Duration = Duration::from_millis(50);

fn main() {
    input! {
        n: usize,
        k: usize,
        rows: [Bytes; n],
    }
    let started = Instant::now();
    let deadline = started + SEARCH_DEADLINE;
    let slimes = rows
        .iter()
        .flatten()
        .filter(|symbol| symbol.is_ascii_lowercase())
        .count();
    USE_BUS.store(slimes < BUS_MAX_SLIMES, std::sync::atomic::Ordering::Relaxed);
    if std::env::var("LEGACY_ONLY").is_ok() {
        // Experiment mode: report only the plain policy length.
        let expired = started;
        let plan = solve(n, k, &rows, TARGET_SAMPLE_SEED, expired, &[]);
        println!("{}", plan.len());
        return;
    }
    let mut best = solve(n, k, &rows, TARGET_SAMPLE_SEED, deadline, &[]);
    let mut runs = 1;
    let mut seed = TARGET_SAMPLE_SEED;
    let mut rng = TargetRng::new(TARGET_SAMPLE_SEED ^ 0x5DEE_CE66);
    while Instant::now() + RESTART_MARGIN < deadline {
        seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
        // Alternate fresh restarts with re-searching a suffix of the best plan.
        let keep = if runs % 2 == 0 {
            rng.index(best.len().max(1))
        } else {
            0
        };
        let output = solve(n, k, &rows, seed, deadline, &best[..keep]);
        runs += 1;
        if output.len() < best.len() {
            best = output;
        }
    }
    eprintln!("restarts runs={runs} best={}", best.len());
    let mut output = String::new();
    for action in best {
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

fn solve(
    n: usize,
    k: usize,
    rows: &[Vec<u8>],
    sample_seed: u64,
    deadline: Instant,
    prefix: &[Action],
) -> Vec<Action> {
    let mut board = Board {
        n,
        adj: Default::default(),
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
    board.build_adjacency();

    let distances: Vec<_> = nest_cells
        .iter()
        .map(|&nest| distances_from(&board, nest))
        .collect();

    let mut actions = Vec::new();
    for &action in prefix {
        board.apply(action);
        actions.push(action);
    }
    let mut target_rng = TargetRng::new(sample_seed);
    let mut move_rng = TargetRng::new(sample_seed ^ 0x3C6E_F372_FE94_F82B);
    // Restarts spend less time on the existing mixed search.
    let existing_mixed_case_time = if sample_seed == TARGET_SAMPLE_SEED {
        EXISTING_MIXED_CASE_TIME
    } else {
        EXISTING_MIXED_RESTART_TIME
    };
    let mut saved = legacy_rollout(&board, &distances, MAX_OPERATIONS, None, None)
        .expect("Legacy solver must produce a complete baseline");
    let mut existing_mixed_stats = ExistingMixedStats::default();
    let mut replans = 0;
    let mut candidates = 0;
    let mut rollouts = 1;
    let mut rollout_stats = [RolloutStats::default(); 5];
    let mut accepted = 0;
    let mut simple_pair_generated = 0;
    let mut simple_pair_evaluated = 0;
    let mut simple_pair_saved = 0;
    let mut simple_pair_executed = 0;
    let mut simple_pair_stats = RolloutStats::default();
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
    let mut springboard_generated = 0;
    let mut springboard_evaluated = 0;
    let mut springboard_saved = 0;
    let mut springboard_executed = 0;
    let mut target_time = Duration::ZERO;
    let mut target_events = Vec::new();
    let mut deadline_skipped_normal = 0;
    let mut deadline_skipped_mixed = 0;
    let mut deadline_skipped_pickup = 0;
    let mut merge_deadline_breaks = 0;
    let mut mixed_deadline_breaks = 0;
    let mut pickup_deadline_breaks = 0;
    while !saved.is_empty() {
        set_reference(&board, &saved);
        let mut selected_simple_pair = None;
        let mut selected_mixed_actions = None;
        let mut selected_pickup_actions = None;
        let mut selected_target_candidate = None;
        let mut selected_springboard = None;
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
                    set_reference(&board, &saved);
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
                    set_reference(&board, &saved);
                    accepted += 1;
                }
            }
            if false && mixed_attempts < MAX_MIXED_REPLANS
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
                            set_reference(&board, &saved);
                            mixed_accepted += 1;
                        }
                    }
                }
            }
            if false && pickup_attempts < MAX_PICKUP_REPLANS
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
                        set_reference(&board, &saved);
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
                let springboard = targets
                    .additional
                    .last()
                    .and_then(|_| saved.front())
                    .and_then(|unit| unit.first())
                    .and_then(|&first| adjacent_springboard_plan(&board, first, &distances));
                springboard_generated += usize::from(springboard.is_some());
                for (index, &candidate) in targets.additional.iter().enumerate() {
                    if Instant::now() >= deadline {
                        target_deadline_hits += 1;
                        break;
                    }
                    let replacement = (index + 1 == targets.additional.len())
                        .then_some(springboard)
                        .flatten();
                    let unit = if let Some(plan) = replacement {
                        springboard_evaluated += 1;
                        plan.actions.to_vec()
                    } else {
                        let Some(unit) =
                            legacy_unit(&board, candidate.cell, candidate.color, &distances)
                        else {
                            continue;
                        };
                        unit
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
                    if replacement.is_none() {
                        target_completed_counts.record(candidate, color);
                    }
                    let candidate_length = unit.len() + plan_length(&continuation);
                    let saved_length = plan_length(&saved);
                    if candidate_length < saved_length {
                        selected_mixed_actions = None;
                        selected_pickup_actions = None;
                        selected_target_saving =
                            before_target_length.saturating_sub(candidate_length);
                        if let Some(plan) = replacement {
                            selected_target_candidate = None;
                            selected_springboard = Some(plan);
                            springboard_saved += 1;
                        } else {
                            selected_target_candidate = Some((candidate, color));
                            selected_springboard = None;
                            target_saved += 1;
                            target_savings += saved_length.saturating_sub(candidate_length);
                            target_saved_counts.record(candidate, color);
                        }
                        continuation.push_front(unit);
                        saved = continuation;
                        set_reference(&board, &saved);
                    }
                }
            }
            target_time += target_started.elapsed();
        }
        if Instant::now() < deadline
            && let Some((cell, color)) = choose_target(&board, &distances)
        {
            let remaining = MAX_OPERATIONS - actions.len();
            // Plain pilot step: other single moves of the target's top run.
            let first_unit = saved.front().cloned();
            let mut alternatives = alternative_moves(&board, cell, &distances[color]);
            alternatives.extend(incoming_moves(&board, cell, &distances));
            // A few moves of random other stacks.
            let occupied: Vec<usize> = (0..board.stacks.len())
                .filter(|&other| other != cell && board.stacks[other].len() > 0)
                .collect();
            for _ in 0..RANDOM_STACK_CANDIDATES.min(occupied.len()) {
                let other = occupied[move_rng.index(occupied.len())];
                let other_color = usize::from(board.stacks[other].last().unwrap());
                alternatives.extend(
                    alternative_moves(&board, other, &distances[other_color])
                        .into_iter()
                        .take(1),
                );
            }
            for action in alternatives {
                if Instant::now() >= deadline {
                    break;
                }
                if first_unit.as_deref() == Some(&[action][..]) {
                    continue;
                }
                let prefix = [action];
                let Ok(mut continuation) = candidate_rollout(
                    &board,
                    &distances,
                    &prefix,
                    remaining,
                    plan_length(&saved),
                    Some(deadline),
                    &mut rollout_stats[4],
                ) else {
                    continue;
                };
                if 1 + plan_length(&continuation) < plan_length(&saved) {
                    selected_mixed_actions = None;
                    selected_pickup_actions = None;
                    selected_target_candidate = None;
                    selected_springboard = None;
                    continuation.push_front(prefix.to_vec());
                    saved = continuation;
                    set_reference(&board, &saved);
                }
            }
        }
        if let Some((source, _)) = choose_target(&board, &distances) {
            let stack = &board.stacks[source];
            if stack.top_run_len() == stack.len() {
                existing_mixed_stats.not_mixed += 1;
            } else if let Some(colors) = existing_mixed_colors(stack) {
                if existing_mixed_stats.attempts >= MAX_EXISTING_MIXED_CALLS {
                    existing_mixed_stats.call_skips += 1;
                } else if existing_mixed_stats.elapsed >= existing_mixed_case_time {
                    existing_mixed_stats.case_time_skips += 1;
                } else if Instant::now() >= deadline {
                    existing_mixed_stats.global_time_skips += 1;
                } else {
                    let call_started = Instant::now();
                    let call_end = call_started + EXISTING_MIXED_CALL_TIME;
                    let case_end =
                        call_started + (existing_mixed_case_time - existing_mixed_stats.elapsed);
                    let local_deadline = deadline.min(call_end).min(case_end);
                    existing_mixed_stats.attempts += 1;
                    let found =
                        existing_mixed_search(&board, source, colors, &distances, local_deadline);
                    existing_mixed_stats.record_search(&found);
                    log_event!(
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
                    log_event!(
                        existing_mixed_stats.events,
                        "existing_sender_search step={} requests={} bfs={} expanded={} found={} path_steps={} invalid={} timed_out={} depth_hits={} registered={} selected_merges={} time_us={} updates={} stale={} settled={} queue_peak={}",
                        actions.len(),
                        found.sender.requests,
                        found.sender.bfs,
                        found.sender.expanded,
                        found.sender.found,
                        found.sender.steps,
                        found.sender.invalid,
                        found.sender.timed_out,
                        found.sender.depth_hits,
                        found.sender.registered,
                        found.sender.selected,
                        found.sender.elapsed.as_micros(),
                        found.updates,
                        found.stale,
                        found.settled,
                        found.queue_peak
                    );
                    for terminal in &found.terminal_info {
                        log_event!(
                            existing_mixed_stats.events,
                            "existing_mixed_terminal step={} p={} collected={} end={} bits={} len={} returned={:?} split={} sender_merges={}",
                            actions.len(),
                            terminal.depth,
                            terminal.state.collected,
                            terminal.state.cell,
                            terminal.state.colors.bits,
                            terminal.state.colors.len,
                            terminal.returned,
                            terminal.split,
                            terminal.sender_merges
                        );
                    }
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
                        log_event!(
                            existing_mixed_stats.events,
                            "existing_mixed_candidate step={} index={} S={} p={} status={} continuation={:?} collected={} end={} end_colors={:?} returned={:?} split={} sender_merges={}",
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
                            candidate.split,
                            candidate.sender_merges
                        );
                        if let Ok(mut continuation) = result {
                            let length = candidate.prefix.len() + plan_length(&continuation);
                            if length < before {
                                existing_mixed_stats.saved += 1;
                                existing_mixed_stats.sender_saved += candidate.sender_merges;

                                existing_mixed_stats.savings += before.saturating_sub(length);
                                log_event!(
                                    existing_mixed_stats.events,
                                    "existing_mixed_save step={} index={} saving={} prefix={:?} pickups={:?} collected={} returned={:?} split={}",
                                    actions.len(),
                                    index,
                                    before.saturating_sub(length),
                                    candidate.prefix,
                                    found.pickups,
                                    candidate.collected,
                                    candidate.returned,
                                    candidate.split
                                );
                                executed_candidate = Some((index, candidate.sender_merges));
                                continuation.push_front(candidate.prefix);
                                saved = continuation;
                                set_reference(&board, &saved);
                            }
                        }
                    }
                    if let Some((index, sender_merges)) = executed_candidate {
                        existing_mixed_stats.executed += 1;
                        existing_mixed_stats.sender_executed += sender_merges;
                        log_event!(
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
        if false && actions.len() < MAX_REPLANS && Instant::now() < deadline {
            let prefixes = simple_pair_candidates(&board, &distances);
            simple_pair_generated += prefixes.len();
            for prefix in prefixes {
                if Instant::now() >= deadline {
                    break;
                }
                simple_pair_evaluated += 1;
                let Ok(mut continuation) = candidate_rollout(
                    &board,
                    &distances,
                    &prefix,
                    MAX_OPERATIONS - actions.len(),
                    plan_length(&saved),
                    Some(deadline),
                    &mut simple_pair_stats,
                ) else {
                    continue;
                };
                rollouts += 1;
                if prefix.len() + plan_length(&continuation) < plan_length(&saved) {
                    selected_mixed_actions = None;
                    selected_pickup_actions = None;
                    selected_target_candidate = None;
                    selected_springboard = None;
                    selected_simple_pair = Some(prefix.clone());
                    continuation.push_front(prefix);
                    saved = continuation;
                    set_reference(&board, &saved);
                    simple_pair_saved += 1;
                }
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
        if selected_springboard.is_some_and(|plan| unit.as_slice() == plan.actions) {
            springboard_executed += 1;
        }
        if selected_simple_pair.as_ref() == Some(&unit) {
            simple_pair_executed += 1;
        }
        for action in unit {
            board.apply(action);
            actions.push(action);
        }
    }
    REFERENCE.with(|cell| *cell.borrow_mut() = None);
    simple_pair_stats.log("simple_pair");
    eprintln!(
        "simple_pair generated={simple_pair_generated} evaluated={simple_pair_evaluated} saved={simple_pair_saved} executed={simple_pair_executed}"
    );
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
        "target_sampling seed={sample_seed} extracted_rank={:?} completed_rank={:?} saved_rank={:?} executed_rank={:?} extracted_color={:?} completed_color={:?} saved_color={:?} executed_color={:?} extracted_slot={:?} completed_slot={:?} saved_slot={:?} executed_slot={:?}",
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
    eprintln!(
        "adjacent_springboard generated={springboard_generated} evaluated={springboard_evaluated} saved={springboard_saved} executed={springboard_executed}"
    );

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board() -> Board {
        Board {
            n: 7,
            adj: Default::default(),
            walls: vec![false; 49],
            nests: vec![None; 49],
            stacks: vec![Stack::default(); 49],
        }
    }

    fn test_nest_distances(board: &Board) -> Vec<Vec<usize>> {
        let mut distances = vec![vec![usize::MAX; board.stacks.len()]; 12];
        for (cell, &nest) in board.nests.iter().enumerate() {
            if let Some(color) = nest {
                distances[usize::from(color)] = distances_from(board, cell);
            }
        }
        distances
    }

    fn mixed_terminal_plan(
        operations: usize,
        collected: u8,
        cell: usize,
        bits: u16,
        len: u8,
    ) -> ExistingMixedPlan {
        ExistingMixedPlan {
            prefix: vec![
                Action {
                    from: 0,
                    k: 0,
                    direction: 3,
                    length: 1,
                };
                operations
            ],
            collected,
            end: ExistingMixedState {
                cell,
                colors: TwoColorStack { bits, len },
                collected,
            },
            split: false,
            returned: [0; 2],
            sender_merges: 0,
        }
    }

    #[test]
    fn mixed_terminal_ranking_prefers_shorter_then_closer_then_original_order() {
        let mut board = empty_board();
        board.nests[0] = Some(0);
        let distances = test_nest_distances(&board);
        let plans = vec![
            mixed_terminal_plan(1, 0, 6, 0, 1),
            mixed_terminal_plan(2, 0, 1, 0, 1),
            mixed_terminal_plan(1, 0, 3, 0, 1),
            mixed_terminal_plan(1, 0, 3, 0, 1),
        ];
        assert_eq!(
            rank_existing_mixed_terminal_indices(&plans, [0, 1], &distances),
            vec![2, 3, 0, 1]
        );
    }

    #[test]
    fn mixed_terminal_ranking_treats_empty_group_as_distance_zero() {
        let mut board = empty_board();
        board.nests[0] = Some(0);
        let distances = test_nest_distances(&board);
        let plans = vec![
            mixed_terminal_plan(1, 0, 6, 0, 1),
            mixed_terminal_plan(1, 0, 48, 0, 0),
        ];
        assert_eq!(
            existing_mixed_terminal_nest_distance(&plans[1], [0, 1], &distances),
            0
        );
        assert_eq!(
            rank_existing_mixed_terminal_indices(&plans, [0, 1], &distances),
            vec![1, 0]
        );
    }

    #[test]
    fn mixed_terminal_selection_keeps_best_representative_for_each_mask_up_to_eight() {
        let mut board = empty_board();
        board.nests[0] = Some(0);
        let distances = test_nest_distances(&board);
        let mut plans = vec![mixed_terminal_plan(2, 0, 1, 0, 1)];
        plans.push(mixed_terminal_plan(1, 0, 1, 0, 1));
        for mask in 1..8 {
            plans.push(mixed_terminal_plan(1, mask, 1, 0, 1));
        }
        let selected = select_existing_mixed_terminal_indices(&plans, [0, 1], &distances);
        assert_eq!(selected.len(), MAX_EXISTING_MIXED_ROLLOUTS);
        assert_eq!(selected, (1..=8).collect::<Vec<_>>());
    }

    fn forward_transport_home_costs(background: &Board, color: u8, moving: usize) -> Vec<usize> {
        let mut costs = vec![usize::MAX; background.stacks.len()];
        for start in 0..background.stacks.len() {
            if background.walls[start] {
                continue;
            }
            let mut queue = VecDeque::from([(start, 0usize)]);
            let mut seen = vec![false; background.stacks.len()];
            seen[start] = true;
            while let Some((from, depth)) = queue.pop_front() {
                let support = background.stacks[from].len();
                for direction in 0..DIRECTIONS.len() {
                    let mut to = from;
                    for _ in 1..=support + 1 {
                        let Some(next) = background.adjacent(to, direction) else {
                            break;
                        };
                        to = next;
                        let target = background.stacks[to];
                        if target.len() + moving > MAX_HEIGHT {
                            continue;
                        }
                        if background.nests[to] == Some(color) && target.len() == 0 {
                            costs[start] = depth + 1;
                            queue.clear();
                            break;
                        } else if !(background.nests[to] == Some(color)
                            && target.last() == Some(color))
                            && !seen[to]
                        {
                            seen[to] = true;
                            queue.push_back((to, depth + 1));
                        }
                    }
                }
            }
        }
        costs
    }

    #[test]
    fn transport_home_reverse_bfs_matches_forward_with_walls_and_supports() {
        let mut board = empty_board();
        board.walls.fill(true);
        for cell in 0..7 {
            board.walls[cell] = false;
        }
        board.walls[3] = true;
        board.nests[6] = Some(0);
        put_colors(&mut board, 1, &[1]);
        put_colors(&mut board, 4, &[1]);
        let reverse =
            transport_home_costs(&board, 0, 1, Instant::now() + Duration::from_secs(60)).unwrap();
        assert_eq!(reverse, forward_transport_home_costs(&board, 0, 1));
        assert_eq!(reverse[0], usize::MAX);
        assert_eq!(reverse[5], 1);

        let mut blocked = board.clone();
        blocked.walls[3] = false;
        put_colors(&mut blocked, 6, &[2; MAX_HEIGHT]);
        assert_eq!(
            transport_home_costs(&blocked, 0, 1, Instant::now() + Duration::from_secs(60)).unwrap()
                [5],
            usize::MAX
        );
    }

    #[test]
    fn mixed_terminal_home_cost_ranking_uses_p_plus_h_and_deadline_fallback() {
        let mut board = empty_board();
        board.nests[6] = Some(0);
        put_colors(&mut board, 0, &[0, 1]);
        let (model, _) = ExistingMixedModel::new(&board, 0, [0, 1]);
        let plans = vec![
            mixed_terminal_plan(1, 0, 1, 0, 1),
            mixed_terminal_plan(3, 0, 4, 0, 1),
        ];
        let distances = test_nest_distances(&board);
        assert_eq!(
            rank_existing_mixed_terminal_indices(&plans, [0, 1], &distances),
            vec![0, 1]
        );
        let (ranked, stats) = rank_existing_mixed_terminal_home_cost_indices(
            &plans,
            &model,
            [0, 1],
            &distances,
            Instant::now() + Duration::from_secs(60),
        );
        assert_eq!(ranked, vec![1, 0]);
        assert_eq!(stats.tables, 1);
        let (fallback, stats) = rank_existing_mixed_terminal_home_cost_indices(
            &plans,
            &model,
            [0, 1],
            &distances,
            Instant::now() - Duration::from_secs(1),
        );
        assert_eq!(fallback, vec![0, 1]);
        assert_eq!(stats.fallback_groups, 1);
    }

    #[test]
    fn adjacent_springboard_moves_neighbor_before_preserving_the_departing_tower() {
        let mut board = empty_board();
        board.walls.fill(true);
        for cell in 0..7 {
            board.walls[cell] = false;
        }
        put_colors(&mut board, 2, &[0]);
        put_colors(&mut board, 1, &[1]);
        board.nests[6] = Some(1);
        let distances = test_nest_distances(&board);
        let plan = adjacent_springboard_plan(
            &board,
            Action {
                from: 2,
                k: 0,
                direction: 3,
                length: 1,
            },
            &distances,
        )
        .unwrap();
        assert_eq!(plan.source, 1);
        assert_eq!(plan.landing, 4);
        assert_eq!(plan.gain, 1);
        let mut replay = board.clone();
        for action in plan.actions {
            assert!(checked_existing_action(&mut replay, action));
        }
        assert_eq!(replay.stacks[2], board.stacks[2]);
        assert_eq!(replay.stacks[1].len(), 0);

        let mut walled = board.clone();
        walled.walls[3] = true;
        assert!(
            adjacent_springboard_plan(
                &walled,
                Action {
                    from: 2,
                    k: 0,
                    direction: 3,
                    length: 1,
                },
                &test_nest_distances(&walled),
            )
            .is_none()
        );

        let mut home = board.clone();
        home.nests[2] = Some(1);
        assert!(
            adjacent_springboard_plan(
                &home,
                Action {
                    from: 2,
                    k: 0,
                    direction: 3,
                    length: 1,
                },
                &test_nest_distances(&home),
            )
            .is_none()
        );

        let mut tall = board;
        put_colors(&mut tall, 2, &[0, 0, 0, 0, 0, 0, 0]);
        put_colors(&mut tall, 1, &[1, 1]);
        assert!(
            adjacent_springboard_plan(
                &tall,
                Action {
                    from: 2,
                    k: 0,
                    direction: 3,
                    length: 1,
                },
                &test_nest_distances(&tall),
            )
            .is_none()
        );
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
            adj: Default::default(),
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
        let distances = test_nest_distances(&board);
        let found = existing_mixed_search(
            &board,
            24,
            [0, 1],
            &distances,
            Instant::now() + Duration::from_secs(60),
        );
        assert!(found.expanded > 0 && found.registered <= 4000);
        assert!(found.invalid == 0 && !found.plans.is_empty());
        assert!(found.plans.iter().any(|p| p.split && p.returned[0] == 4));
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
        let distances = test_nest_distances(&board);
        let found = existing_mixed_search(
            &board,
            24,
            [0, 1],
            &distances,
            Instant::now() - Duration::from_secs(1),
        );
        assert!(found.timed_out && found.plans.is_empty());
        assert_eq!(board.stacks, before);
        board.walls.fill(true);
        board.walls[24] = false;
        let found = existing_mixed_search(
            &board,
            24,
            [0, 1],
            &distances,
            Instant::now() + Duration::from_secs(60),
        );
        assert!(found.plans.is_empty() && !found.timed_out && found.invalid == 0);
        assert_eq!(found.registered, 1);
    }

    #[test]
    fn existing_mixed_depth_24_terminal_is_inclusive() {
        let mut board = Board {
            n: 25,
            adj: Default::default(),
            walls: vec![true; 625],
            nests: vec![None; 625],
            stacks: vec![Stack::default(); 625],
        };
        for cell in 0..25 {
            board.walls[cell] = false;
        }
        put_colors(&mut board, 0, &[1, 0]);
        board.nests[24] = Some(0);
        let distances = test_nest_distances(&board);
        let found = existing_mixed_search(
            &board,
            0,
            [0, 1],
            &distances,
            Instant::now() + Duration::from_secs(60),
        );
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
            adj: Default::default(),
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
        let distances = test_nest_distances(&board);
        let found = existing_mixed_search(
            &board,
            108,
            [2, 3],
            &distances,
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
    fn sender_test_deadline() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    #[test]
    fn sender_arrival_preserves_receiver_order_and_differs_from_pickup() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 23, &[1]);
        let (model, state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let incoming = model
            .sender_edge(
                state,
                0,
                24,
                sender_test_deadline(),
                &mut SenderStats::default(),
            )
            .unwrap();
        let outgoing = model
            .transitions(state)
            .into_iter()
            .find(|t| t.state.cell == 23 && t.state.collected == 1)
            .unwrap();
        assert_eq!(
            &model.board_at(incoming.state).stacks[24].colors[..3],
            &[0, 1, 1]
        );
        assert_eq!(
            &model.board_at(outgoing.state).stacks[23].colors[..3],
            &[1, 1, 0]
        );
        assert_eq!(incoming.actions.len(), 1);
        let mut actual = board.clone();
        for a in incoming.actions {
            assert!(checked_existing_action(&mut actual, a));
        }
        assert_eq!(actual.stacks, model.board_at(incoming.state).stacks);
    }

    #[test]
    fn sender_route_leaves_support_and_empties_origin() {
        let mut board = empty_board();
        board.walls.fill(true);
        board.walls[21..=24].fill(false);
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 21, &[1]);
        put_colors(&mut board, 22, &[2, 2]);
        let (model, state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let edge = model
            .sender_edge(
                state,
                0,
                24,
                sender_test_deadline(),
                &mut SenderStats::default(),
            )
            .unwrap();
        assert_eq!(edge.actions.len(), 2);
        assert_eq!(edge.actions[0].k, 0);
        assert_eq!(
            (
                edge.actions[1].from,
                edge.actions[1].k,
                edge.actions[1].length
            ),
            (22, 2, 2)
        );
        let mut actual = board.clone();
        for &a in &edge.actions {
            assert!(checked_existing_action(&mut actual, a));
        }
        assert_eq!(actual.stacks[21].len(), 0);
        assert_eq!(actual.stacks[22], board.stacks[22]);
        assert_eq!(actual.stacks, model.board_at(edge.state).stacks);
        assert!(
            model
                .sender_edge(
                    state,
                    0,
                    1,
                    sender_test_deadline(),
                    &mut SenderStats::default()
                )
                .is_none()
        );
    }

    #[test]
    fn sender_arrival_above_background_checks_height_and_retains_support() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 23, &[2, 2]);
        put_colors(&mut board, 22, &[1]);
        let (model, initial) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let state = model
            .transitions(initial)
            .into_iter()
            .find(|t| t.state.cell == 23)
            .unwrap()
            .state;
        let edge = model
            .sender_edge(
                state,
                0,
                24,
                sender_test_deadline(),
                &mut SenderStats::default(),
            )
            .unwrap();
        assert_eq!(model.stack(23, edge.state.collected).len(), 2);
        assert_eq!(
            &model.board_at(edge.state).stacks[23].colors[..5],
            &[2, 2, 1, 0, 1]
        );
        let mut actual = model.board_at(state);
        for a in edge.actions {
            assert!(checked_existing_action(&mut actual, a));
        }
        assert_eq!(actual.stacks, model.board_at(edge.state).stacks);
        let mut full = board;
        put_colors(&mut full, 23, &[2, 2, 2, 2, 2, 2]);
        let (model, mut state) = ExistingMixedModel::new(&full, 24, [0, 1]);
        state.cell = 23;
        let mut stats = SenderStats::default();
        assert!(
            model
                .sender_edge(state, 0, 24, sender_test_deadline(), &mut stats)
                .is_none()
        );
        assert_eq!(stats.bfs, 0);
    }

    #[test]
    fn sender_home_on_arrival_but_not_during_route() {
        let mut board = empty_board();
        board.walls.fill(true);
        board.walls[21..=24].fill(false);
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 21, &[0]);
        board.nests[24] = Some(0);
        let (model, state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let edge = model
            .sender_edge(
                state,
                0,
                24,
                sender_test_deadline(),
                &mut SenderStats::default(),
            )
            .unwrap();
        assert_eq!(edge.state.colors, state.colors);
        assert_eq!(edge.state.collected, 1);
        let mut actual = board.clone();
        for a in edge.actions {
            assert!(checked_existing_action(&mut actual, a));
        }
        assert_eq!(actual.stacks, model.board_at(edge.state).stacks);
        board.nests[22] = Some(0);
        let (model, state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        assert!(
            model
                .sender_edge(
                    state,
                    0,
                    24,
                    sender_test_deadline(),
                    &mut SenderStats::default()
                )
                .is_none()
        );
    }

    #[test]
    fn sender_rejects_collected_same_cell_single_and_expired() {
        let mut board = empty_board();
        put_colors(&mut board, 24, &[0, 1]);
        put_colors(&mut board, 23, &[1]);
        let (model, state) = ExistingMixedModel::new(&board, 24, [0, 1]);
        let mut invalid = state;
        invalid.collected = 1;
        assert!(
            model
                .sender_edge(
                    invalid,
                    0,
                    24,
                    sender_test_deadline(),
                    &mut SenderStats::default()
                )
                .is_none()
        );
        invalid = state;
        invalid.cell = 23;
        assert!(
            model
                .sender_edge(
                    invalid,
                    0,
                    24,
                    sender_test_deadline(),
                    &mut SenderStats::default()
                )
                .is_none()
        );
        invalid = state;
        invalid.colors = TwoColorStack { bits: 0, len: 2 };
        assert!(
            model
                .sender_edge(
                    invalid,
                    0,
                    24,
                    sender_test_deadline(),
                    &mut SenderStats::default()
                )
                .is_none()
        );
        let mut stats = SenderStats::default();
        assert!(
            model
                .sender_edge(
                    state,
                    0,
                    24,
                    Instant::now() - Duration::from_secs(1),
                    &mut stats
                )
                .is_none()
        );
        assert_eq!(stats.timed_out, 1);
    }

    #[test]
    fn weighted_frontier_improves_at_capacity_and_discards_stale_paths() {
        let state = ExistingMixedState {
            cell: 0,
            colors: TwoColorStack { bits: 1, len: 2 },
            collected: 0,
        };
        let mid = ExistingMixedState { cell: 1, ..state };
        let end = ExistingMixedState { cell: 2, ..state };
        let extra = ExistingMixedState { cell: 3, ..state };
        let a = Action {
            from: 0,
            k: 0,
            direction: 3,
            length: 1,
        };
        let b = Action { from: 1, ..a };
        let mut q = ExistingMixedFrontier::new(state, 3);
        let mut result = ExistingMixedSearch {
            registered: 1,
            ..Default::default()
        };
        assert_eq!(q.pop(&mut result), Some(0));
        q.relax(
            0,
            ExistingMixedEdge {
                state: end,
                actions: vec![a, b, a],
                split: false,
                sender: true,
            },
            &mut result,
        );
        q.relax(
            0,
            ExistingMixedEdge {
                state: mid,
                actions: vec![a],
                split: false,
                sender: false,
            },
            &mut result,
        );
        q.relax(
            0,
            ExistingMixedEdge {
                state: extra,
                actions: vec![a],
                split: false,
                sender: false,
            },
            &mut result,
        );
        assert!(result.state_limit);
        let m = q.pop(&mut result).unwrap();
        assert_eq!(q.nodes[m].state, mid);
        q.relax(
            m,
            ExistingMixedEdge {
                state: end,
                actions: vec![b],
                split: false,
                sender: false,
            },
            &mut result,
        );
        let e = q.pop(&mut result).unwrap();
        assert_eq!(q.nodes[e].depth, 2);
        assert_eq!(q.path(e), vec![a, b]);
        assert_eq!(q.nodes[e].sender_merges, 0);
        assert_eq!(result.updates, 1);
        assert_eq!(result.registered, 3);
        assert!(q.pop(&mut result).is_none());
        assert_eq!(result.stale, 1);
    }

    #[test]
    fn weighted_equal_cost_keeps_first_path_and_segment_order() {
        let state = ExistingMixedState {
            cell: 0,
            colors: TwoColorStack { bits: 1, len: 2 },
            collected: 0,
        };
        let end = ExistingMixedState { cell: 2, ..state };
        let other = ExistingMixedState { cell: 3, ..state };
        let a = Action {
            from: 0,
            k: 0,
            direction: 3,
            length: 1,
        };
        let b = Action { from: 1, ..a };
        let mut q = ExistingMixedFrontier::new(state, 4);
        let mut r = ExistingMixedSearch::default();
        q.pop(&mut r);
        q.relax(
            0,
            ExistingMixedEdge {
                state: end,
                actions: vec![a, b],
                split: false,
                sender: true,
            },
            &mut r,
        );
        q.relax(
            0,
            ExistingMixedEdge {
                state: other,
                actions: vec![b, a],
                split: false,
                sender: true,
            },
            &mut r,
        );
        q.relax(
            0,
            ExistingMixedEdge {
                state: end,
                actions: vec![b, a],
                split: false,
                sender: true,
            },
            &mut r,
        );
        let e = q.pop(&mut r).unwrap();
        assert_eq!(q.nodes[e].state, end);
        assert_eq!(q.path(e), vec![a, b]);
        assert_eq!(q.nodes[e].sender_merges, 1);
        let other_index = q.pop(&mut r).unwrap();
        assert_eq!(q.nodes[other_index].state, other);
    }

    fn sender_case0000_fixture() -> Board {
        let mut b = Board {
            n: 12,
            adj: Default::default(),
            walls: vec![false; 144],
            nests: vec![None; 144],
            stacks: vec![Stack::default(); 144],
        };
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
        for (r, row) in rows.iter().enumerate() {
            for (c, v) in row.bytes().enumerate() {
                if v == b'#' {
                    b.walls[r * 12 + c] = true;
                } else if v.is_ascii_uppercase() {
                    b.nests[r * 12 + c] = Some(v - b'A');
                }
            }
        }
        for (cell, colors) in [
            (60, vec![0]),
            (61, vec![0]),
            (70, vec![1]),
            (85, vec![0]),
            (88, vec![3]),
            (95, vec![1]),
            (96, vec![0]),
            (99, vec![3]),
            (108, vec![3, 2, 2, 2, 2]),
            (116, vec![1, 1]),
            (133, vec![3]),
        ] {
            put_colors(&mut b, cell, &colors);
        }
        b
    }

    #[test]
    fn sender_case0000_two_step_route_and_complete_fixed_interval() {
        let mut board = sender_case0000_fixture();
        let (model, mut state) = ExistingMixedModel::new(&board, 108, [2, 3]);
        let specs = [
            (108, 0, 3, 1),
            (133, 0, 0, 1),
            (121, 0, 0, 1),
            (109, 0, 3, 1),
            (110, 0, 3, 1),
            (111, 0, 0, 1),
            (99, 0, 0, 1),
            (87, 0, 3, 1),
            (88, 0, 0, 1),
            (76, 4, 3, 2),
        ];
        let actions: Vec<_> = specs
            .iter()
            .map(|&(from, k, direction, length)| Action {
                from,
                k,
                direction,
                length,
            })
            .collect();
        state = model
            .transitions(state)
            .into_iter()
            .find(|t| t.action == actions[0] && t.state.collected == 0)
            .unwrap()
            .state;
        assert!(checked_existing_action(&mut board, actions[0]));
        let sender = model
            .sender_edge(
                state,
                0,
                24,
                sender_test_deadline(),
                &mut SenderStats::default(),
            )
            .unwrap();
        assert_eq!(sender.actions, &actions[1..3]);
        for &a in &sender.actions {
            assert!(checked_existing_action(&mut board, a));
        }
        state = sender.state;
        assert_eq!(&board.stacks[109].colors[..6], &[2, 2, 2, 2, 3, 3]);
        assert_eq!(board.stacks, model.board_at(state).stacks);
        for (index, &action) in actions.iter().enumerate().skip(3) {
            let transition = model
                .transitions(state)
                .into_iter()
                .filter(|t| t.action == action)
                .max_by_key(|t| t.state.collected.count_ones())
                .unwrap();
            assert!(checked_existing_action(&mut board, action));
            state = transition.state;
            assert_eq!(
                board.stacks,
                model.board_at(state).stacks,
                "at operation {index}"
            );
        }
        assert_eq!(state.collected, 7);
        assert_eq!(state.cell, 78);
        assert_eq!(&board.stacks[78].colors[..4], &[3, 3, 3, 3]);
        assert!(board.stacks[76].len() == 0 && model.terminal(state));
    }

    #[test]
    fn weighted_search_is_deterministic_and_replays_sender_segments() {
        let b = sender_case0000_fixture();
        let distances = test_nest_distances(&b);
        let x = existing_mixed_search(&b, 108, [2, 3], &distances, sender_test_deadline());
        let y = existing_mixed_search(&b, 108, [2, 3], &distances, sender_test_deadline());
        assert_eq!(
            x.plans.iter().map(|p| &p.prefix).collect::<Vec<_>>(),
            y.plans.iter().map(|p| &p.prefix).collect::<Vec<_>>()
        );
        assert_eq!(
            (x.registered, x.updates, x.stale),
            (y.registered, y.updates, y.stale)
        );
        assert_eq!(x.invalid, 0);
        assert_eq!(x.sender.invalid, 0);
        let (model, _) = ExistingMixedModel::new(&b, 108, [2, 3]);
        for p in x.plans {
            let mut actual = b.clone();
            for a in &p.prefix {
                assert!(checked_existing_action(&mut actual, *a));
            }
            assert_eq!(actual.stacks, model.board_at(p.end).stacks);
            assert!(p.prefix.len() <= 24);
        }
    }
}
