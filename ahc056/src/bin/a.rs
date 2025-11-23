use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
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
struct TransitionRule {
    new_color: usize,
    new_state: usize,
    dir: Dir,
}

impl std::fmt::Display for TransitionRule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.new_color, self.new_state, self.dir)
    }
}

#[derive(Clone, Copy, Debug)]
struct OutRule {
    in_rule: (usize, usize),
    out_rule: TransitionRule,
}

impl std::fmt::Display for OutRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.in_rule.0, self.in_rule.1, self.out_rule)
    }
}

#[derive(Clone, Debug)]
struct Solution {
    color_num: usize,
    state_num: usize,
    init_colors: Vec<usize>,
    rules: Vec<OutRule>,
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

/// i 番目に max(x, y) が小さい順の (x, y) を返す。
fn ith_square_coord(i: usize) -> (usize, usize) {
    // 層 M = floor(sqrt(i))
    let m = (i as f64).sqrt().floor() as usize;
    // 層 M の開始 index
    let start = m * m;
    // 層 internal index (0..2m)
    let t = i - start;

    if m == 0 {
        return (0, 0);
    }

    if t <= m {
        // 上側の辺
        (t, m)
    } else {
        // 右側の辺
        (m, t - (m + 1))
    }
}

/// 団子解法で解く
/// 団子解法: 経路を固定し, すべての経路に一意の (色, 内部状態) を割り振る
fn solve_dumpling(n: usize, paths: &Vec<usize>) -> Solution {
    let grid_size_1d = n * n;
    let mut color_num = 1;
    let mut state_num = 1;
    let mut paths_reversed = paths.clone();
    paths_reversed.reverse();

    let mut state_last = 0;
    let mut rules = vec![];
    let mut color_and_state_per_cell: Vec<Vec<(usize, usize)>> = vec![vec![]; grid_size_1d];
    let mut used_color_and_state_num = 1;
    for (i, &c) in paths_reversed.iter().enumerate() {
        if i == 0 {
            continue;
        }

        // 移動先の情報
        let new_color = if let Some(cc) = color_and_state_per_cell[c].last() {
            cc.0
        } else {
            0
        };
        let new_state = state_last;
        let dir = move_dir_from_1d(paths_reversed[i], paths_reversed[i - 1], n);

        // 今のマスの情報
        let coord = if i == paths.len() - 1 {
            (0, 0)
        } else {
            ith_square_coord(used_color_and_state_num)
        };
        color_and_state_per_cell[c].push(coord);
        used_color_and_state_num += 1;

        rules.push(OutRule {
            in_rule: coord,
            out_rule: TransitionRule {
                new_color,
                new_state,
                dir,
            },
        });

        state_last = coord.1;
        color_num = color_num.max(coord.0);
        state_num = state_num.max(coord.1);
    }
    color_num += 1;
    state_num += 1;
    debug!("{color_and_state_per_cell:?}");

    let mut init_colors = vec![0; grid_size_1d];
    // 通過なしマスは色 0 で塗っておく
    for (i, cs) in color_and_state_per_cell.iter().enumerate() {
        if let Some(&a) = cs.last() {
            init_colors[i] = a.0;
        }
    }

    Solution {
        color_num,
        state_num,
        init_colors,
        rules,
    }
}

#[fastout]
fn main() {
    let _start_time_all = Instant::now();
    let _break_time = Duration::from_millis(TIME_LIMIT_MS);

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

    // 複数解法を比較する方法も取れるよう, 関数に切り出す
    let dumpling_ans = solve_dumpling(n, &paths);

    // output
    // サイズ
    println!(
        "{} {} {}",
        dumpling_ans.color_num,
        dumpling_ans.state_num,
        dumpling_ans.rules.len()
    );
    // 色
    for (i, c) in dumpling_ans.init_colors.iter().enumerate() {
        print!("{c}");
        if i % n == n - 1 {
            println!();
        } else {
            print!(" ");
        }
    }
    // 遷移規則
    for r in &dumpling_ans.rules {
        println!("{r}");
    }
}
