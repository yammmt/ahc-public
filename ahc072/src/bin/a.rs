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

#[derive(Clone, Copy)]
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
// Units are kept intact when a saved continuation is resumed.
fn legacy_rollout(
    initial: &Board,
    distances: &[Vec<usize>],
    limit: usize,
    deadline: Option<Instant>,
) -> Option<Plan> {
    let mut board = initial.clone();
    let mut plan = Plan::new();
    let mut count = 0;
    while let Some((cell, color)) = choose_target(&board, distances) {
        if count >= limit || deadline.is_some_and(|time| Instant::now() >= time) {
            return None;
        }
        let unit = if let Some(pair) = choose_pair(&board, cell, color, distances)
            && count + pair.len() <= limit
        {
            pair
        } else if count + 2 <= limit
            && let Some(relay) = choose_relay(&board, cell, color, &distances[color])
        {
            relay.actions.to_vec()
        } else {
            vec![choose_group_move(&board, cell, color, &distances[color])?.0]
        };
        if count + unit.len() > limit {
            return None;
        }
        for &action in &unit {
            board.apply(action);
        }
        count += unit.len();
        plan.push_back(unit);
    }
    Some(plan)
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

// Search to a chosen third group while both original colors are still present.
// Homecoming before pickup is excluded, so the two arrival orientations suffice.
fn pickup_approaches(
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
        for (_, third) in thirds.into_iter().take(MAX_PICKUP_THIRDS) {
            if Instant::now() >= deadline {
                break;
            }
            result.triples += 1;
            let approaches = pickup_approaches(board, source, partner, third, deadline);
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
    let mut saved = legacy_rollout(&board, &distances, MAX_OPERATIONS, None)
        .expect("Legacy solver must produce a complete baseline");
    let mut actions = Vec::new();
    let mut replans = 0;
    let mut candidates = 0;
    let mut rollouts = 1;
    let mut accepted = 0;
    let mut mixed_attempts = 0;
    let mut mixed_partners = 0;
    let mut mixed_candidates = 0;
    let mut mixed_rollouts = 0;
    let mut mixed_accepted = 0;
    let mut mixed_actions = 0;
    let mut pickup_attempts = 0;
    let mut pickup_triples = 0;
    let mut pickup_approaches_count = 0;
    let mut pickup_candidates_count = 0;
    let mut pickup_rollouts = 0;
    let mut pickup_saved = 0;
    let mut pickup_executed = 0;
    let mut pickup_actions = 0;
    while !saved.is_empty() {
        let mut selected_mixed_actions = None;
        let mut selected_pickup_actions = None;
        if replans < MAX_REPLANS
            && Instant::now() < deadline
            && let Some((cell, color)) = choose_target(&board, &distances)
            && board.stacks[cell].len() < MAX_HEIGHT
            && board.stacks[cell].top_run_len() == board.stacks[cell].len()
        {
            replans += 1;
            let remaining = MAX_OPERATIONS - actions.len();
            if let Some(fresh) = legacy_rollout(&board, &distances, remaining, Some(deadline)) {
                rollouts += 1;
                if plan_length(&fresh) < plan_length(&saved) {
                    saved = fresh;
                }
            }
            let paths = merge_candidates(&board, cell, color);
            candidates += paths.len();
            for path in paths.into_iter().take(MAX_CANDIDATE_ROLLOUTS) {
                if Instant::now() >= deadline {
                    break;
                }
                if path.len() >= plan_length(&saved) {
                    continue;
                }
                let mut after = board.clone();
                for &action in &path {
                    after.apply(action);
                }
                let Some(mut continuation) =
                    legacy_rollout(&after, &distances, remaining - path.len(), Some(deadline))
                else {
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
                        break;
                    }
                    if merge_path.len() >= plan_length(&saved) {
                        continue;
                    }
                    let plans = mixed_transport_plans(&board, cell, partner, &merge_path);
                    mixed_candidates += plans.len();
                    for prefix in plans {
                        if Instant::now() >= deadline {
                            break;
                        }
                        if prefix.len() >= plan_length(&saved) {
                            continue;
                        }
                        let mut after = board.clone();
                        for &action in &prefix {
                            after.apply(action);
                        }
                        let Some(mut continuation) = legacy_rollout(
                            &after,
                            &distances,
                            remaining - prefix.len(),
                            Some(deadline),
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
                let found = pickup_candidates(&board, cell, color, &distances, deadline);
                pickup_triples += found.triples;
                pickup_approaches_count += found.approaches;
                pickup_candidates_count += found.plans.len();
                for prefix in found.plans.into_iter().take(MAX_PICKUP_ROLLOUTS) {
                    if Instant::now() >= deadline {
                        break;
                    }
                    if prefix.len() >= plan_length(&saved) {
                        continue;
                    }
                    let mut after = board.clone();
                    for &action in &prefix {
                        after.apply(action);
                    }
                    let Some(mut continuation) = legacy_rollout(
                        &after,
                        &distances,
                        remaining - prefix.len(),
                        Some(deadline),
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
        }
        if let Some(length) = selected_mixed_actions {
            mixed_actions += length;
        }
        if let Some(length) = selected_pickup_actions {
            pickup_executed += 1;
            pickup_actions += length;
        }
        let unit = saved.pop_front().expect("Nonempty saved continuation");
        for action in unit {
            board.apply(action);
            actions.push(action);
        }
    }
    eprintln!(
        "merge_search replans={replans} candidates={candidates} rollouts={rollouts} accepted={accepted} mixed_attempts={mixed_attempts} mixed_partners={mixed_partners} mixed_candidates={mixed_candidates} mixed_rollouts={mixed_rollouts} mixed_accepted={mixed_accepted} mixed_actions={mixed_actions} pickup_attempts={pickup_attempts} pickup_triples={pickup_triples} pickup_approaches={pickup_approaches_count} pickup_candidates={pickup_candidates_count} pickup_rollouts={pickup_rollouts} pickup_saved={pickup_saved} pickup_executed={pickup_executed} pickup_actions={pickup_actions}"
    );

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
}
