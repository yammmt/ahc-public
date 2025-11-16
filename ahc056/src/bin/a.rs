use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use std::collections::VecDeque;
use std::fmt::{self, Formatter};
use std::time::{Duration, Instant};

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)+);
        }
    };
}

const TIME_LIMIT_MS: u64 = 1000;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
    Stay,
}

impl std::fmt::Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Dir::Up => write!(f, "U"),
            Dir::Down => write!(f, "D"),
            Dir::Left => write!(f, "L"),
            Dir::Right => write!(f, "R"),
            Dir::Stay => write!(f, "S"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TransitionRules {
    new_color: usize,
    new_state: usize,
    dir: Dir,
}

impl std::fmt::Display for TransitionRules {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.new_color, self.new_state, self.dir)
    }
}

#[inline]
fn twod_to_oned((x, y): (usize, usize), n: usize) -> usize {
    x * n + y
}

#[inline]
fn oned_to_twod(xy: usize, n: usize) -> (usize, usize) {
    (xy / n, xy % n)
}

#[inline]
fn move_dir_from_1d(pfrom: usize, pto: usize, n: usize) -> Dir {
    let pfrom_2d = oned_to_twod(pfrom, n);
    let pto_2d = oned_to_twod(pto, n);
    let mv_i = (
        pto_2d.0 as isize - pfrom_2d.0 as isize,
        pto_2d.1 as isize - pfrom_2d.1 as isize,
    );
    match mv_i {
        (-1, 0) => Dir::Up,
        (1, 0) => Dir::Down,
        (0, -1) => Dir::Left,
        (0, 1) => Dir::Right,
        _ => Dir::Stay, // 安全に Stay を返す（通常は到達しない）
    }
}

fn shortest_path(ps: usize, gs: usize, edges: &Vec<Vec<usize>>) -> Vec<usize> {
    let n = edges.len();
    let mut que = VecDeque::new();
    let mut comes_from = vec![None; n];
    que.push_back((ps, ps));
    while let Some((pos_cur, pos_from)) = que.pop_front() {
        if comes_from[pos_cur].is_some() {
            continue;
        }
        comes_from[pos_cur] = Some(pos_from);
        if pos_cur == gs {
            break;
        }
        for &pos_next in &edges[pos_cur] {
            if comes_from[pos_next].is_none() {
                que.push_back((pos_next, pos_cur));
            }
        }
    }
    let mut ret = vec![gs];
    let mut cur_pos = gs;
    while let Some(comes_from) = comes_from[cur_pos] {
        ret.push(comes_from);
        if comes_from == ps {
            break;
        }
        cur_pos = comes_from;
    }
    ret.reverse();
    ret
}

fn could_merge_move(dir_i: &Vec<Option<Dir>>, dir_j: &Vec<Option<Dir>>, state_num: usize) -> bool {
    let some_appears_i = dir_i.iter().any(|x| x.is_some());
    let some_appears_j = dir_j.iter().any(|x| x.is_some());
    if !some_appears_i && !some_appears_j {
        return false;
    }
    for i in 0..state_num {
        if i < dir_i.len() && i < dir_j.len() {
            if dir_i[i].is_some() && dir_j[i].is_some() && dir_i[i] != dir_j[i] {
                return false;
            }
        }
    }
    true
}

fn merge_colors(
    colors: &mut Vec<usize>,
    use_colors: &mut Vec<bool>,
    move_dirs: &mut Vec<Vec<Option<Dir>>>,
    a: usize,
    b: usize,
) {
    let col_len = move_dirs.get(0).map(|v| v.len()).unwrap_or(0);
    for i in 0..col_len {
        if move_dirs[b][i].is_some() {
            move_dirs[a][i] = move_dirs[b][i];
            move_dirs[b][i] = None;
        }
    }
    for c in colors.iter_mut() {
        if *c == b {
            *c = a;
        }
    }
    if b < use_colors.len() {
        use_colors[b] = false;
    }
}

#[derive(Clone)]
struct Solution {
    color_num: usize,
    state_num: usize,
    init_colors: Vec<usize>,                // length grid_size
    rules: Vec<Vec<Option<TransitionRules>>> // [color][state]
}

#[fastout]
fn main() {
    let start_time_all = Instant::now();
    let mut rng = SmallRng::seed_from_u64(1);

    input! {
        n: usize,
        k: usize,
        _t: usize,
        vnn: [Chars; n],
        hnn: [Chars; n - 1],
        xyk: [(usize, usize); k],
    }
    let grid_size_1d = n * n;

    // build edges (安全チェック付き)
    let mut edges = vec![vec![]; grid_size_1d];
    for (i, v) in vnn.iter().enumerate() {
        // v: Chars for row i; assumed length at least n-1 (safe-guard)
        for (j, &vv) in v.iter().enumerate() {
            if vv == '0' && j + 1 < n {
                let a = twod_to_oned((i, j), n);
                let b = twod_to_oned((i, j + 1), n);
                edges[a].push(b);
                edges[b].push(a);
            }
        }
    }
    for (i, h) in hnn.iter().enumerate() {
        for (j, &hh) in h.iter().enumerate() {
            if hh == '0' && i + 1 < n {
                let a = twod_to_oned((i, j), n);
                let b = twod_to_oned((i + 1, j), n);
                edges[a].push(b);
                edges[b].push(a);
            }
        }
    }

    // build full path visiting xyk in order
    let mut paths = vec![];
    let mut cur_pos = twod_to_oned(*xyk.first().unwrap(), n);
    for &xy in xyk.iter().skip(1) {
        let goal = twod_to_oned(xy, n);
        let path = shortest_path(cur_pos, goal, &edges);
        if paths.is_empty() {
            if !path.is_empty() {
                paths.push(path[0]);
            }
        }
        for &p in path.iter().skip(1) {
            paths.push(p);
        }
        cur_pos = *path.last().unwrap();
    }
    debug!("paths: {paths:?}");

    // ---- separate solvers ----

    // front (pattern-based) solver
    let front_sol = solve_front(&paths, n, k);

    // back (merge-try) solver, with time limit
    let break_time = Duration::from_millis(TIME_LIMIT_MS);
    let start_time = Instant::now();
    let back_sol = solve_back(&paths, n, k, start_time, break_time, &mut rng);

    // choose better (smaller color_num + state_num). tie -> front preference
    let score_front = front_sol.color_num + front_sol.state_num;
    let score_back = back_sol.color_num + back_sol.state_num;
    debug!("score_front={} score_back={}", score_front, score_back);

    let chosen = if score_back < score_front { back_sol } else { front_sol };

    // output
    let rules_num = count_rules(&chosen.rules);
    println!("{} {} {}", chosen.color_num, chosen.state_num, rules_num);
    for i in 0..n {
        for j in 0..n {
            let p = twod_to_oned((i, j), n);
            print!("{}", chosen.init_colors[p]);
            if j + 1 != n {
                print!(" ");
            } else {
                println!();
            }
        }
    }
    for i in 0..chosen.color_num {
        for j in 0..chosen.state_num {
            if i < chosen.rules.len() && j < chosen.rules[i].len() {
                if let Some(t) = chosen.rules[i][j] {
                    println!("{} {} {}", i, j, t);
                }
            }
        }
    }

    debug!("total elapsed: {:?}", start_time_all.elapsed());
}

fn count_rules(rules: &Vec<Vec<Option<TransitionRules>>>) -> usize {
    let mut cnt = 0;
    for row in rules.iter() {
        for cell in row.iter() {
            if cell.is_some() {
                cnt += 1;
            }
        }
    }
    cnt
}

/// 前半の解法：パターン的に (color,state) を割り当てる実装（元ロジック準拠）
fn solve_front(paths: &Vec<usize>, n: usize, k: usize) -> Solution {
    let grid_size_1d = n * n;
    // decide color/state counts from pattern sqrt
    let mut ptrn_sqrt = 10usize;
    while ptrn_sqrt * ptrn_sqrt < paths.len().max(1) {
        ptrn_sqrt += 1;
    }
    let (color_num, state_num) = if (ptrn_sqrt - 1) * ptrn_sqrt >= paths.len() {
        (ptrn_sqrt - 1, ptrn_sqrt)
    } else {
        (ptrn_sqrt, ptrn_sqrt)
    };

    // moves_each_vertex[p][pass_index] = Some((color,state))
    let mut moves_each_vertex = vec![vec![None; paths.len().max(1)]; grid_size_1d];
    let mut vertex_pass_count = vec![0usize; grid_size_1d];
    let mut init_colors = vec![0usize; grid_size_1d];

    for (i, &p) in paths.iter().enumerate() {
        let cur_color = i / state_num;
        let cur_state = i % state_num;
        let idx = vertex_pass_count[p];
        if idx < moves_each_vertex[p].len() {
            moves_each_vertex[p][idx] = Some((cur_color, cur_state));
        }
        if vertex_pass_count[p] == 0 {
            init_colors[p] = cur_color;
        }
        vertex_pass_count[p] += 1;
    }

    let mut ans_moves = vec![vec![None; state_num]; color_num];
    vertex_pass_count.fill(0);
    for (i, &p) in paths.iter().take(paths.len().saturating_sub(1)).enumerate() {
        let cur_color = i / state_num;
        let cur_state = i % state_num;
        let next_p = paths[i + 1];
        let pass_idx = vertex_pass_count[p];
        let new_color = if pass_idx + 1 < moves_each_vertex[p].len() {
            if let Some(m) = moves_each_vertex[p][pass_idx + 1] {
                m.0
            } else {
                cur_color
            }
        } else {
            cur_color
        };
        let pass_next = vertex_pass_count[next_p];
        let new_state = if pass_next < moves_each_vertex[next_p].len() {
            if let Some(m) = moves_each_vertex[next_p][pass_next] {
                m.1
            } else {
                cur_state
            }
        } else {
            cur_state
        };
        let dir = move_dir_from_1d(p, paths[i + 1], n);
        if cur_color < ans_moves.len() && cur_state < ans_moves[cur_color].len() {
            ans_moves[cur_color][cur_state] = Some(TransitionRules {
                new_color,
                new_state,
                dir,
            });
        }
        vertex_pass_count[p] += 1;
    }

    Solution {
        color_num,
        state_num,
        init_colors,
        rules: ans_moves,
    }
}

/// 後半の解法：パスを基に move_dirs を初期化し、時間制限内で色マージを試行（元ロジック準拠）
fn solve_back(
    paths: &Vec<usize>,
    n: usize,
    k: usize,
    start_time: Instant,
    break_time: Duration,
    rng: &mut SmallRng,
) -> Solution {
    let grid_size_1d = n * n;
    // construct initial move_dirs by walking paths (as original did)
    let max_state_capacity = paths.len().max(k).max(1);
    let mut move_dirs = vec![vec![None; max_state_capacity]; grid_size_1d];
    let mut is_state_update_points = vec![false; grid_size_1d];
    let mut state_update_points: Vec<usize> = vec![];

    let mut cur_pos = *paths.first().unwrap_or(&0);
    let mut cur_state = 0usize;
    for (i, &p) in paths.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if cur_state >= max_state_capacity {
            break;
        }
        // set move from cur_pos at cur_state to p
        move_dirs[cur_pos][cur_state] = Some(move_dir_from_1d(cur_pos, p, n));
        // check next-next to detect state update point (original heuristic)
        if i + 1 < paths.len().saturating_sub(0) {
            let next_idx = paths[i + 1];
            if next_idx < grid_size_1d && move_dirs[next_idx][cur_state].is_some() {
                state_update_points.push(cur_pos);
                is_state_update_points[cur_pos] = true;
                cur_state = cur_state.saturating_add(1);
            }
        }
        cur_pos = p;
    }
    let state_num = cur_state + 1;

    // prepare initial colors (1:1 mapping)
    let mut init_colors = (0..grid_size_1d).map(|x| x).collect::<Vec<usize>>();

    // best-so-far
    let mut best_score = usize::MAX;
    let mut best_move_dirs = move_dirs.clone();
    let mut best_colors = init_colors.clone();
    let mut best_color2out = vec![None; grid_size_1d];
    let mut best_state_update_points = state_update_points.clone();
    let mut best_color_num = grid_size_1d;
    let mut best_state_num = state_num;

    // time-limited randomized merging
    while start_time.elapsed() < break_time {
        let mut colors = (0..grid_size_1d).collect::<Vec<usize>>();
        let mut use_colors = vec![true; grid_size_1d];
        let mut indices = (0..grid_size_1d).collect::<Vec<usize>>();
        indices.shuffle(rng);
        for i in 0..grid_size_1d {
            let ii = indices[i];
            if !use_colors[ii] || is_state_update_points[ii] {
                continue;
            }
            for j in i + 1..grid_size_1d {
                let jj = indices[j];
                if !use_colors[jj] || is_state_update_points[jj] {
                    continue;
                }
                let a = colors[ii];
                let b = colors[jj];
                if a >= move_dirs.len() || b >= move_dirs.len() {
                    continue;
                }
                if could_merge_move(&move_dirs[a], &move_dirs[b], state_num) {
                    let aa = a.min(b);
                    let bb = a.max(b);
                    merge_colors(&mut colors, &mut use_colors, &mut move_dirs, aa, bb);
                }
            }
        }

        // compress colors
        let mut color2outcolor = vec![None; grid_size_1d];
        let mut color_i = 0usize;
        for (i, &c) in use_colors.iter().enumerate() {
            if c {
                color2outcolor[i] = Some(color_i);
                color_i += 1;
            }
        }
        let color_num = color_i;
        let cur_score = color_num + state_num;
        if cur_score < best_score {
            best_score = cur_score;
            best_move_dirs = move_dirs.clone();
            best_colors = colors.clone();
            best_color2out = color2outcolor.clone();
            best_state_update_points = state_update_points.clone();
            best_color_num = color_num;
            best_state_num = state_num;
        }
    }

    // build rules from best_move_dirs
    let mut rules: Vec<Vec<Option<TransitionRules>>> =
        vec![vec![None; best_state_num]; best_color_num.max(1)];
    let mut init_colors_out = vec![0usize; grid_size_1d];
    for i in 0..grid_size_1d {
        let outc = if let Some(mapped) = best_color2out.get(i).and_then(|&x| x) {
            mapped
        } else {
            // if not mapped, map by representative color
            let rep = best_colors[i];
            best_color2out.get(rep).and_then(|&x| x).unwrap_or(0)
        };
        init_colors_out[i] = outc;
    }

    // fill rules from best_move_dirs: iterate over original vertices and their per-state moves
    for v in 0..grid_size_1d {
        // determine output color index
        let rep = best_colors[v];
        let cur_color = best_color2out[rep].unwrap_or(0);
        for s in 0..best_state_num {
            if s < best_move_dirs[v].len() {
                if let Some(dir) = best_move_dirs[v][s] {
                    // determine new_state: try to preserve original heuristic (if this v is a state update point)
                    let new_state = if s < best_state_update_points.len()
                        && v == best_state_update_points[s]
                    {
                        s + 1
                    } else {
                        s
                    };
                    if cur_color < rules.len() && s < rules[cur_color].len() {
                        rules[cur_color][s] = Some(TransitionRules {
                            new_color: cur_color,
                            new_state,
                            dir,
                        });
                    }
                }
            }
        }
    }

    Solution {
        color_num: best_color_num,
        state_num: best_state_num,
        init_colors: init_colors_out,
        rules,
    }
}
