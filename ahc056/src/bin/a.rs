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

// TODO: in もまとめるべきような
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

#[derive(Clone, Copy, Debug)]
struct RuleInWIP {
    cur_color: Option<usize>,
    cur_state: Option<usize>,
}

impl RuleInWIP {
    fn is_empty(&self) -> bool {
        self.cur_color.is_none() && self.cur_state.is_none()
    }
}

#[derive(Clone, Copy, Debug)]
struct RuleOutWIP {
    new_color: Option<usize>,
    new_state: Option<usize>,
    dir: Option<Dir>,
}

impl RuleOutWIP {
    fn is_empty(&self) -> bool {
        self.new_color.is_none() && self.new_state.is_none() && self.dir.is_none()
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

/// 二つの行動に対し同じ遷移規則の入出力を割り当てられるか試行し, 成功時には真を返しつつ,
/// 可変参照で受け取った配列をマージして返す.
/// - 引数である二つの行動に割り振る遷移規則 (出力) は, 色の塗り替え及び内部状態の更新は
///   しないものとする.
/// - マージにより state の削減が見込めない場合には, マージ不可として扱う.
///   - 由来: 内部状態数の削減を state > color の順で行っているため
// - vec で受け取っている引数は HashSet でもよいし, そっちの方が早いかも？
//   - でも vec の長さもたいしたことなさそうであり, 線形でもまぁなような
// - 地獄のような複雑度であり, なんとかしたい...
// TODO: 入力だけ巻き取っておけばよいよね？
//       - 新しい色: 同じマスを次に通る時の色
//       - 新しい内部状態: 次に進むマスの状態
fn merge_moves(
    n: usize,
    path_i_cur: usize,
    path_j_cur: usize,
    paths: &[usize],
    vertex_pass_counts: &[usize],
    // paths[i] はそのマスの何度目の通過か
    path_idx_to_vertex_pass_count: &[usize],
    // マス i の j 度目の通過に対応する paths[k] の k を返す
    vertex_pass_count_to_path_idx: &[Vec<usize>],
    status_assigned_per_color: &mut [Vec<bool>],

    path_rule_in: &mut [RuleInWIP],
    path_rule_out: &mut [RuleOutWIP],
    init_colors: &mut [Option<usize>],
) -> bool {
    if path_i_cur == 0 || path_j_cur == 0 {
        // 一旦実装で楽をする
        return false;
    }

    let color_num = status_assigned_per_color.len();
    let state_num = status_assigned_per_color[0].len();

    // paths[i] と一致する要素 j<i を満たす最大の j を返す.
    let path_idx_same_vertex_prev = |i: usize| {
        let vpc = path_idx_to_vertex_pass_count[i];
        if vpc == 0 {
            return None;
        }

        Some(vertex_pass_count_to_path_idx[paths[i]][vpc - 1])
    };
    // paths[i] と一致する要素 j>i を満たす最小の j を返す.
    let path_idx_same_vertex_next = |i: usize| {
        // FIXME: なんか辻褄怪しい...
        if i > path_idx_to_vertex_pass_count.len() {
            return None;
        }

        let vpc = path_idx_to_vertex_pass_count[i];
        if vpc == vertex_pass_counts[paths[i]] - 1 {
            return None;
        }

        Some(vertex_pass_count_to_path_idx[paths[i]][vpc + 1])
    };

    let path_idx_same_vertex_i_prev = path_idx_same_vertex_prev(path_i_cur);
    let path_idx_same_vertex_j_prev = path_idx_same_vertex_prev(path_j_cur);
    let path_idx_same_vertex_i_next = path_idx_same_vertex_next(path_i_cur);
    let path_idx_same_vertex_j_next = path_idx_same_vertex_next(path_j_cur);

    let path_idx_next_vertex_i_prev = path_idx_same_vertex_prev(path_i_cur + 1);
    let path_idx_next_vertex_j_prev = path_idx_same_vertex_prev(path_j_cur + 1);

    if path_i_cur == path_j_cur
        // 直前に通過するマスの遷移規則出力が設定されていない
        || !((path_i_cur == 0 && init_colors[path_i_cur].is_none() || path_rule_out[path_i_cur - 1].is_empty())
                && ((path_j_cur == 0 && init_colors[path_j_cur].is_none()) || path_rule_out[path_j_cur - 1].is_empty()))

        // 今回通過するマスの入力が設定済でない
        || !(path_rule_in[path_i_cur].is_empty() && path_rule_in[path_j_cur].is_empty())
        // 今回通過するマスの出力が設定済でない
        || !(path_rule_out[path_i_cur].is_empty() && path_rule_out[path_j_cur].is_empty())
        // 今回通過するマスについて, 前回通過時の遷移規則出力にて塗りつぶす色が設定されていない
        || !((path_idx_same_vertex_i_prev.is_none() && init_colors[path_i_cur].is_none()) || path_rule_out[path_idx_same_vertex_i_prev.unwrap()].new_color.is_none())
        || !((path_idx_same_vertex_j_prev.is_none() && init_colors[path_j_cur].is_none()) || path_rule_out[path_idx_same_vertex_j_prev.unwrap()].new_color.is_none())
        // 今回通過するマスの次回通過時の遷移規則入力にて色が設定されていない
        || !(path_idx_same_vertex_i_next.is_none() || path_rule_in[path_idx_same_vertex_i_next.unwrap()].cur_color.is_none())
        || !(path_idx_same_vertex_j_next.is_none() || path_rule_in[path_idx_same_vertex_j_next.unwrap()].cur_color.is_none())

        // 直後に通過するマスの遷移規則入力が設定されていない
        || !((path_i_cur == paths.len() - 1 || path_rule_in[path_i_cur + 1].is_empty())
                && (path_j_cur == paths.len() - 1 || path_rule_in[path_j_cur + 1].is_empty()))
        // 直後に通過するマスの前回通過時に塗りつぶす色が設定されていない
        // TODO: 初期色
        || !((path_i_cur == paths.len() - 1 || (path_idx_next_vertex_i_prev.is_none() && init_colors[path_idx_next_vertex_i_prev.unwrap()].is_none()) || path_rule_out[path_idx_next_vertex_i_prev.unwrap()].new_color.is_none())
            && (path_j_cur == paths.len() - 1 || (path_idx_next_vertex_j_prev.is_none() && init_colors[path_idx_next_vertex_j_prev.unwrap()].is_none()) || path_rule_out[path_idx_next_vertex_j_prev.unwrap()].new_color.is_none()))
    {
        return false;
    }

    // 今回割り当てる色と状態, 及び今回のマスを通過後に通るマスの色を決める
    let mut use_cur_color = None;
    let mut use_cur_state = None;
    let mut use_colors_after_ij = [None, None];
    let mut used_status_min = usize::MAX;
    // 今回割り当てる (色 c, 内部状態 s) の条件:
    // - (c, s) が未使用であること
    // - (a, s) 及び (b, s) が未使用である色 a, b が存在すること
    // かつ, 削減幅を大きくするため, 使用済み内部状態数が最小のものを使う.
    for i in 0..color_num {
        let mut use_status_candidate = None;
        let mut used_status_count = 0;
        let mut use_colors_after_candidate = [None, None];
        for j in 0..state_num {
            if !status_assigned_per_color[i][j] {
                used_status_count += 1;
                if use_status_candidate.is_none() {
                    // (色 i, 内部状態 j) を使うとして, 内部状態を区別するために,
                    // 別の色二色を割り当てられるかを確認する.
                    for k in 0..color_num {
                        if k == i {
                            continue;
                        }

                        if !status_assigned_per_color[k][j] {
                            if use_colors_after_candidate[0].is_none() {
                                use_colors_after_candidate[0] = Some(k);
                            } else {
                                use_colors_after_candidate[1] = Some(k);
                                break;
                            }
                        }
                    }

                    use_status_candidate = Some(j);
                }
            }
        }

        if used_status_count < used_status_min {
            use_cur_color = Some(i);
            use_cur_state = use_status_candidate;
            use_colors_after_ij = use_colors_after_candidate;
            used_status_min = used_status_count;
        }
    }

    // 冗長だと思うんだが
    if use_cur_color.is_none()
        || use_cur_state.is_none()
        || use_colors_after_ij.iter().any(|&a| a.is_none())
    {
        return false;
    }

    status_assigned_per_color[use_cur_color.unwrap()][use_cur_state.unwrap()] = true;
    status_assigned_per_color[use_colors_after_ij[0].unwrap()][use_cur_state.unwrap()] = true;
    status_assigned_per_color[use_colors_after_ij[1].unwrap()][use_cur_state.unwrap()] = true;

    // マージ対象の二つのマス **直前の** マスに対し,
    // - 今回通過時の遷移規則の出力に, 割り当てる内部状態を割り振る
    // TODO: 初手
    path_rule_out[path_j_cur - 1].new_state = use_cur_state;
    path_rule_out[path_i_cur - 1].new_state = use_cur_state;

    // マージ対象の二つのマスに対し,
    // - 今回の行動に, 同じ遷移規則の入力を割り振る
    path_rule_in[path_i_cur].cur_color = use_cur_color;
    path_rule_in[path_j_cur].cur_state = use_cur_state;
    // - 今回の行動に, 同じ遷移規則の出力を割り振る
    let dir = move_dir_from_1d(path_i_cur, path_j_cur, n);
    path_rule_out[path_i_cur] = RuleOutWIP {
        new_color: use_cur_color,
        new_state: use_cur_state,
        dir: Some(dir),
    };
    path_rule_out[path_j_cur] = RuleOutWIP {
        new_color: use_cur_color,
        new_state: use_cur_state,
        dir: Some(dir),
    };
    // - 前回通過時の遷移規則出力に, 今回割り当てる色を割り振る
    if let Some(i) = path_idx_same_vertex_i_prev {
        path_rule_out[i].new_color = use_cur_color;
    } else {
        init_colors[paths[path_i_cur]] = use_cur_color;
    }
    if let Some(i) = path_idx_same_vertex_j_prev {
        path_rule_out[i].new_color = use_cur_color;
    } else {
        init_colors[paths[path_j_cur]] = use_cur_color;
    }
    // - 次回通過時の遷移規則入力に, 今回割り当てる色を割り振る
    if let Some(i) = path_idx_same_vertex_i_next {
        path_rule_in[i].cur_color = use_cur_color;
    } else {
        unreachable!();
    }
    if let Some(i) = path_idx_same_vertex_j_next {
        path_rule_in[i].cur_color = use_cur_color;
    } else {
        unreachable!();
    }

    // マージ対象の二つのマス **直後の** マスに対し,
    // - 前回通過時の遷移規則出力に, それぞれ異なる色を割り振る
    if let Some(i) = path_idx_next_vertex_i_prev {
        path_rule_out[i].new_color = use_colors_after_ij[0];
    }
    if let Some(i) = path_idx_next_vertex_j_prev {
        path_rule_out[i].new_color = use_colors_after_ij[1];
    }
    // - 今回行動直後の遷移規則の入力に, それぞれ異なる色及び内部状態を割り振る
    if path_i_cur < paths.len() - 1 {
        path_rule_in[path_i_cur + 1].cur_color = use_colors_after_ij[0];
        path_rule_in[path_i_cur + 1].cur_state = use_cur_state;
    }
    if path_j_cur < paths.len() - 1 {
        path_rule_in[path_j_cur + 1].cur_color = use_colors_after_ij[1];
        path_rule_in[path_j_cur + 1].cur_state = use_cur_state;
    }

    true
}

/// 二つの色に割り当てられた行動を確認し, 相反する行動がなければ `true` を返す.
#[allow(dead_code)]
fn could_merge_color(dir_i: &Vec<Option<Dir>>, dir_j: &Vec<Option<Dir>>, state_num: usize) -> bool {
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

        // マス i の通過回数
        let mut vertex_pass_counts = vec![0; grid_size_1d];
        // i 番目のパスがマス j の k 度目の通過 (0-origin)
        let mut path_idx_to_vertex_pass_count = vec![];
        // マス i の j 回目の通過時のパスは全体で何番目か
        let mut vertex_pass_count_to_path_idx = vec![vec![]; grid_size_1d];
        for (i, &p) in paths.iter().enumerate() {
            path_idx_to_vertex_pass_count[i] = vertex_pass_counts[p];
            vertex_pass_count_to_path_idx[vertex_pass_counts[p]].push(i);
            vertex_pass_counts[p] += 1;
        }
        // moves[色][内部状態] = (塗り替える色, 新しい内部状態, 移動方向)
        let mut ans_moves = vec![vec![None; state_num]; color_num];
        let mut status_assigned_per_color = vec![vec![false; state_num]; color_num];
        // 各マスの通過回数を個別に持たせると時間が怪しい
        vertex_pass_count.fill(0);
        // TODO: マージ判定, とりあえず貪欲にするが実際は乱択可能
        for i in 0..paths.len() {
            let mut vertex_pass_count_cur = vertex_pass_count.clone();
            // FIXME: 通過回数の扱いを考える, 特に i と j の間
            for j in i + 2..paths.len() {
                // 三点以上のマージは, 簡単のために一旦考えない
                // if merge_moves(
                //     i,
                //     j,
                //     &paths,
                //     &vertex_pass_counts,
                //     &path_idx_to_vertex_pass_count,
                //     &vertex_pass_count_to_path_idx,
                //     &mut ans_moves,
                //     &mut moves_each_vertex,
                //     &mut status_assigned_per_color,
                // ) {
                //     break;
                // }
                // vertex_pass_count_cur[j] += 1;
            }
            // vertex_pass_count[i] += 1;
        }

        // 全経路に固有の (色, 内部状態) の組を割り当てる
        // 解を出力するには, 次の移動先, 内部状態と自身のマスの次の色がわかればよい
        let mut ptrn_sqrt = 10;
        while ptrn_sqrt * ptrn_sqrt < paths.len() {
            ptrn_sqrt += 1;
        }
        let (mut color_num, mut state_num) = if (ptrn_sqrt - 1) * ptrn_sqrt >= paths.len() {
            (ptrn_sqrt - 1, ptrn_sqrt)
        } else {
            (ptrn_sqrt, ptrn_sqrt)
        };

        // moves_each_vertex[i][j]: マス i を j 回目に通過した際の (色, 内部状態)
        // TODO: 初期割り当てに状態削減を考えると消すよ
        let mut moves_each_vertex = vec![vec![None; k]; grid_size_1d];
        let mut vertex_pass_count = vec![0; grid_size_1d];
        let mut init_colors = vec![0; grid_size_1d];
        for (i, &p) in paths.iter().enumerate() {
            let cur_color = i / state_num;
            let cur_state = i % state_num;
            moves_each_vertex[p][vertex_pass_count[p]] = Some((cur_color, cur_state));
            if vertex_pass_count[p] == 0 {
                init_colors[p] = cur_color;
            }
            vertex_pass_count[p] += 1;
        }

        vertex_pass_count.fill(0);
        // 最後のマスに指示は不要であるので最初から省く
        // TODO: 削減結果の反映と color/state 数の整理
        for (i, &p) in paths.iter().take(paths.len() - 1).enumerate() {
            let cur_color = i / state_num;
            let cur_state = i % state_num;
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

        let cur_score = color_num + state_num;
        if cur_score < best_score {
            best_score = cur_score;
            color_num_ans = color_num;
            state_num_ans = state_num;
            init_colors_ans = init_colors;

            let mut rules_num = 0;
            for i in 0..color_num {
                for j in 0..state_num {
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
