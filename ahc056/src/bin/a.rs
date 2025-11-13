use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
use rand::SeedableRng;
use rand::rngs::SmallRng;
// use rand::seq::SliceRandom;
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

// 2 s
const TIME_LIMIT_MS: u64 = 1500;
// const TIME_LIMIT_MS: u64 = 10000;

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
        _ => unreachable!(),
    }
}

/// 二点間最短経路を求める.
/// - 最短経路が複数ある際には, どれか一つだけを返す.
/// - 入出力はすべて一次元座標とする.
/// - 返す経路には, 始点と終点をそれぞれ含む.
fn shortest_path(ps: usize, gs: usize, edges: &Vec<Vec<usize>>) -> Vec<usize> {
    let n = edges.len();

    let mut que = VecDeque::new();
    let mut comes_from = vec![None; n];
    // (次, 遷移元)
    // 訪問済み判定の便宜上, 始点も入れておく
    que.push_back((ps, ps));
    while let Some((pos_cur, pos_from)) = que.pop_front() {
        if comes_from[pos_cur].is_some() {
            continue;
        }

        comes_from[pos_cur] = Some(pos_from);
        if pos_cur == gs {
            // 枝刈り
            break;
        }
        // 完全乱択より経路重複しない側に手厚くした方がよさそうなのだが
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

/// 二つの色に割り当てられた行動を確認し, 相反する行動がなければ `true` を返す.
#[allow(dead_code)]
fn could_merge_move(dir_i: &Vec<Option<Dir>>, dir_j: &Vec<Option<Dir>>, state_num: usize) -> bool {
    // 不要色同士をマージしない
    let some_appears_i = dir_i.iter().any(|x| x.is_some());
    let some_appears_j = dir_j.iter().any(|x| x.is_some());
    if !some_appears_i && !some_appears_j {
        return false;
    }

    for i in 0..state_num {
        if dir_i[i].is_some() && dir_j[i].is_some() && dir_i[i] != dir_j[i] {
            return false;
        }
    }

    true
}

/// 色 b の行動を色 a にマージする. エラー判定は行わないので, 実行可能判定は呼び元が責任を追う.
/// マージされた色 b の行動は, すべて `None` になる.
#[allow(dead_code)]
fn merge_colors(
    colors: &mut Vec<usize>,
    use_colors: &mut Vec<bool>,
    move_dirs: &mut Vec<Vec<Option<Dir>>>,
    a: usize,
    b: usize,
) {
    // move rule を a 側に移す
    for i in 0..move_dirs[0].len() {
        if move_dirs[b][i].is_some() {
            move_dirs[a][i] = move_dirs[b][i];
            move_dirs[b][i] = None;
        }
    }
    // 色代表を a に
    // TODO: UF で高速化
    for c in colors.iter_mut() {
        if *c == b {
            *c = a;
        }
    }
    use_colors[b] = false;
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let break_time = Duration::from_millis(TIME_LIMIT_MS);
    let mut _rng = SmallRng::seed_from_u64(1);
    // let mut rng = SmallRng::from_rng(&mut rand::rng());

    input! {
        n: usize,
        k: usize,
        // TODO: 活用したい
        _t: usize,
        vnn: [Chars; n],
        hnn: [Chars; n - 1],
        xyk: [(usize, usize); k],
    }
    let grid_size_1d = n * n;

    // 移動可能な方向を定義
    let mut edges = vec![vec![]; n * n];
    for (i, v) in vnn.iter().enumerate() {
        for (j, &vv) in v.iter().enumerate() {
            if vv == '0' {
                let a = twod_to_oned((i, j), n);
                let b = twod_to_oned((i, j + 1), n);
                edges[a].push(b);
                edges[b].push(a);
            }
        }
    }
    for (i, h) in hnn.iter().enumerate() {
        for (j, &hh) in h.iter().enumerate() {
            if hh == '0' {
                let a = twod_to_oned((i, j), n);
                let b = twod_to_oned((i + 1, j), n);
                edges[a].push(b);
                edges[b].push(a);
            }
        }
    }

    // 雑
    let mut best_score = usize::MAX;
    let mut color_num_ans = usize::MAX;
    let mut state_num_ans = usize::MAX;
    let mut rules_num_ans = 0;
    let mut init_colors_ans = vec![0; grid_size_1d];
    let mut rules_ans = vec![];
    while start_time.elapsed() < break_time {
        // 目的地順に最短経路を求める
        // 経路は後の迂回試行を考えると線形リストの方が都合がよいかもしれない
        let mut paths = vec![];
        let mut cur_pos = twod_to_oned(*xyk.first().unwrap(), n);
        for &xy in xyk.iter().skip(1) {
            let goal = twod_to_oned(xy, n);
            // TODO: 経路選択に乱択を入れる
            let path = shortest_path(cur_pos, goal, &edges);
            // 直前の終点 (目的地) と今の始点の重複除去は reverse -> pop -> reverse -> append でも
            // 動作はするが, reverse 二度かける分の定数倍が嫌
            if paths.is_empty() {
                paths.push(path[0]);
            }
            for &p in path.iter().skip(1) {
                paths.push(p);
            }

            cur_pos = *path.last().unwrap();
        }
        debug!("paths: {paths:?}");

        // 全経路に固有の (色, 内部状態) の組を割り当てる
        // 解を出力するには, 次の移動先, 内部状態と自身のマスの次の色がわかればよい
        let mut ptrn_sqrt = 10;
        while ptrn_sqrt * ptrn_sqrt < paths.len() {
            ptrn_sqrt += 1;
        }
        // moves_each_vertex[i][j]: マス i を j 回目に通過した際の (色, 内部状態)
        let mut moves_each_vertex = vec![vec![None; k]; grid_size_1d];
        let mut vertex_pass_count = vec![0; grid_size_1d];
        let mut init_colors = vec![0; grid_size_1d];
        for (i, &p) in paths.iter().enumerate() {
            let cur_color = i / ptrn_sqrt;
            let cur_state = i % ptrn_sqrt;
            moves_each_vertex[p][vertex_pass_count[p]] = Some((cur_color, cur_state));
            if vertex_pass_count[p] == 0 {
                init_colors[p] = cur_color;
            }
            vertex_pass_count[p] += 1;
        }

        // moves[色][内部状態] = (塗り替える色, 新しい内部状態, 移動方向)
        let mut ans_moves = vec![vec![None; ptrn_sqrt]; ptrn_sqrt];
        vertex_pass_count.fill(0);
        // 最後のマスに指示は不要であるので最初から省く
        for (i, &p) in paths.iter().take(paths.len() - 1).enumerate() {
            let cur_color = i / ptrn_sqrt;
            let cur_state = i % ptrn_sqrt;
            let next_p = paths[i + 1];
            let new_color = if let Some(m) = moves_each_vertex[p][vertex_pass_count[p] + 1] {
                m.0
            } else {
                cur_color
            };
            let new_state = if let Some(m) = moves_each_vertex[next_p][vertex_pass_count[next_p]] {
                m.1
            } else {
                cur_state
            };
            let dir = move_dir_from_1d(p, paths[i + 1], n);
            ans_moves[cur_color][cur_state] = Some(TransitionRules {
                new_color,
                new_state,
                dir,
            });
            vertex_pass_count[p] += 1;
        }

        let color_num = ptrn_sqrt;
        let state_num = ptrn_sqrt;

        let cur_score = color_num + state_num;
        if cur_score < best_score {
            best_score = cur_score;
            color_num_ans = color_num;
            state_num_ans = state_num;
            init_colors_ans = init_colors;

            let mut rules_num = 0;
            for i in 0..ptrn_sqrt {
                for j in 0..ptrn_sqrt {
                    if ans_moves[i][j].is_none() {
                        break;
                    }

                    rules_num += 1;
                }
            }
            rules_num_ans = rules_num;
            rules_ans = ans_moves;
        }

        // 乱択ができていないので
        break;
    }

    println!("{color_num_ans} {state_num_ans} {rules_num_ans}");
    for i in 0..n {
        for j in 0..n {
            let p = twod_to_oned((i, j), n);
            print!("{}", init_colors_ans[p]);
            if j != n - 1 {
                print!(" ");
            } else {
                println!();
            }
        }
    }
    for i in 0..color_num_ans {
        for j in 0..state_num_ans {
            if let Some(t) = rules_ans[i][j] {
                println!("{i} {j} {t}");
            }
        }
    }
}
