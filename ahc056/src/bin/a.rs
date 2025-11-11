use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
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
const TIME_LIMIT_MS: u64 = 1800;
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

#[derive(Debug)]
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
fn shortest_path<T>(ps: usize, gs: usize, edges: &Vec<Vec<usize>>, rng: &mut T) -> Vec<usize>
where
    T: RngCore,
{
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
        // TODO: remove!
        let mut indices: Vec<usize> = (0..edges[pos_cur].len()).collect();
        indices.shuffle(rng);
        for i in indices {
            let pos_next = edges[pos_cur][i];
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
    let mut rng = SmallRng::seed_from_u64(1);
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
    let mut move_dirs_ans = vec![vec![None; k]; grid_size_1d];
    let mut colors_ans = vec![];
    let mut color2outcolor_ans = vec![];
    let mut state_update_points_ans = vec![];
    let mut color_num_ans = usize::MAX;
    let mut state_num_ans = usize::MAX;
    while start_time.elapsed() < break_time {
        // 目的地順に最短経路を求める
        // 経路は後の迂回試行を考えると線形リストの方が都合がよいかもしれない
        let mut paths = vec![];
        let mut cur_pos = twod_to_oned(*xyk.first().unwrap(), n);
        for &xy in xyk.iter().skip(1) {
            let goal = twod_to_oned(xy, n);
            // TODO: 経路選択に乱択を入れる
            let path = shortest_path(cur_pos, goal, &edges, &mut rng);
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

        // 経路を可能な限り一筆書きに辿る
        // 内部状態の更新は, 次に訪問するマスがその内部状態で訪問済みであった時点
        // TODO: この方法では, state をこれ以上削減し辛い
        //       色を塗り替えて再利用する方法は思い浮かぶのだが, 実装が辛そう...
        let mut cur_pos = twod_to_oned(*xyk.first().unwrap(), n);
        let mut cur_state = 0;
        // move_dirs[色][内部状態]
        let mut move_dirs = vec![vec![None; k]; grid_size_1d];
        // この二つの変数は意図が重複しとらんか
        let mut is_state_update_points = vec![false; grid_size_1d];
        let mut state_update_points = vec![];
        for (i, &p) in paths.iter().enumerate() {
            if i == 0 {
                continue;
            }

            move_dirs[cur_pos][cur_state] = Some(move_dir_from_1d(cur_pos, p, n));
            if i + 1 < paths.len() - 1 && move_dirs[paths[i + 1]][cur_state].is_some() {
                state_update_points.push(cur_pos);
                is_state_update_points[cur_pos] = true;
                cur_state += 1;
            }

            cur_pos = p;
        }
        let state_num = cur_state + 1;
        debug!("state_update_points: {state_update_points:?}");
        debug!("state_num: {state_num}");

        // 色数削減
        // - 一度も通過しない頂点があれば, 適当な色として扱う
        // - すべての二つの頂点の組について, 内部状態に重複がなければマージする
        // None 同士でマージ可能になるとかなり遅くなるので避ける
        let mut colors = (0..grid_size_1d).collect::<Vec<usize>>();
        let mut use_colors = vec![true; grid_size_1d];
        // マージ順は乱択できる
        let mut indices = (0..grid_size_1d).collect::<Vec<usize>>();
        indices.shuffle(&mut rng);
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
                if could_merge_move(&move_dirs[a], &move_dirs[b], state_num) {
                    let aa = a.min(b);
                    let bb = a.max(b);
                    merge_colors(&mut colors, &mut use_colors, &mut move_dirs, aa, bb);
                }
            }
        }
        debug!("colors: {colors:?}");
        debug!("use_colors: {use_colors:?}");

        // 色の座圧
        let mut color2outcolor = vec![None; grid_size_1d];
        let mut color_i = 0;
        for (i, &c) in use_colors.iter().enumerate() {
            if c {
                color2outcolor[i] = Some(color_i);
                color_i += 1;
            }
        }
        debug!("color2outcolor: {color2outcolor:?}");
        let color_num = color_i;

        // TODO: 異なる二つの内部状態について, 同じマスを通過しない場合にはマージする

        let cur_score = color_num + state_num;
        if cur_score < best_score {
            // 雑
            best_score = cur_score;
            move_dirs_ans = move_dirs;
            state_update_points_ans = state_update_points;
            colors_ans = colors;
            color2outcolor_ans = color2outcolor;
            color_num_ans = color_num;
            state_num_ans = state_num;
        }
    }

    // 遷移規則
    let mut rules = vec![];
    for i in 0..grid_size_1d {
        let cur_color = color2outcolor_ans[colors_ans[i]].unwrap();
        for j in 0..k {
            if let Some(dir) = move_dirs_ans[i][j] {
                rules.push((
                    // (色, 内部状態)
                    (cur_color, j),
                    // 塗り替える色, 新しい内部状態, 移動方向
                    TransitionRules {
                        // 色の塗り替えは現状考慮せず
                        new_color: cur_color,
                        new_state: if j < state_update_points_ans.len()
                            && i == state_update_points_ans[j]
                        {
                            j + 1
                        } else {
                            j
                        },
                        dir,
                    },
                ));
            }
        }
    }

    // 出力
    println!("{color_num_ans} {state_num_ans} {}", rules.len());
    // 色の出力
    for i in 0..n {
        for j in 0..n {
            print!(
                "{}",
                color2outcolor_ans[colors_ans[twod_to_oned((i, j), n)]].unwrap()
            );
            if j != n - 1 {
                print!(" ");
            } else {
                println!();
            }
        }
    }
    // 遷移規則の出力
    for ((c, q), t) in rules {
        println!("{c} {q} {t}");
    }
}
