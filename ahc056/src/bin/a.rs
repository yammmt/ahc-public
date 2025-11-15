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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RuleIn {
    color: usize,
    state: usize,
}

impl std::fmt::Display for RuleIn {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {}", self.color, self.state)
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

/// paths[i] と一致する要素 j>i を満たす最小の j を返す.
fn path_idx_same_vertex_next(
    i: usize,
    paths: &[usize],
    path_idx_to_vertex_pass_count: &[usize],
    vertex_pass_count_to_path_idx: &[Vec<usize>],
    vertex_pass_counts: &[usize],
) -> Option<usize> {
    // FIXME: なんか辻褄怪しい...
    if i >= path_idx_to_vertex_pass_count.len() {
        return None;
    }

    let vpc = path_idx_to_vertex_pass_count[i];
    if vpc == vertex_pass_counts[paths[i]] - 1 {
        return None;
    }

    // debug!("i: {i}");
    // debug!("paths: {paths:?}");
    // debug!("path_idx_to_vertex_pass_count: {path_idx_to_vertex_pass_count:?}");
    // debug!("vertex_pass_count_to_path_idx: {vertex_pass_count_to_path_idx:?}");
    // debug!("vertex_pass_counts: {vertex_pass_counts:?}");
    Some(vertex_pass_count_to_path_idx[paths[i]][vpc + 1])
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

/// 二つの行動に対し同じ遷移規則の入力を割り当てられるか試行し, 成功時には真を返しつつ,
/// 可変参照で受け取った配列をマージして返す.
// - 引数である二つの行動に割り振る遷移規則 (出力) は, 色の塗り替え及び内部状態の更新は
//   しないものとする.
// - マージにより state の削減が見込めない場合には, マージ不可として扱う.
//   - 由来: 内部状態数の削減を state > color の順で行っているため
// - vec で受け取っている引数は HashSet でもよいし, そっちの方が早いかも？
//   - でも vec の長さもたいしたことなさそうであり, 線形でもまぁなような
// - 地獄のような複雑度であり, なんとかしたい...
// 入力だけ巻き取っておけばよいよね？
//   - 新しい色: 同じマスを次に通る時の色
//   - 新しい内部状態: 次に進むマスの状態
// TODO: 色/状態数の削減に主観を置く
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
    path_rule_in: &mut [Option<RuleIn>],
) -> bool {
    if path_i_cur == 0
        || path_j_cur == 0
        || path_i_cur >= path_j_cur
        || path_j_cur >= paths.len() - 1
    {
        // 一旦実装で楽をする
        return false;
    }

    let color_num = status_assigned_per_color.len();
    let state_num = status_assigned_per_color[0].len();

    // // paths[i] と一致する要素 j<i を満たす最大の j を返す.
    // let _path_idx_same_vertex_prev = |i: usize| {
    //     let vpc = path_idx_to_vertex_pass_count[i];
    //     if vpc == 0 {
    //         return None;
    //     }
    //     Some(vertex_pass_count_to_path_idx[paths[i]][vpc - 1])
    // };

    // 進行方向
    let dir_i = move_dir_from_1d(paths[path_i_cur], paths[path_i_cur + 1], n);
    let dir_j = move_dir_from_1d(paths[path_j_cur], paths[path_j_cur + 1], n);
    if dir_i != dir_j {
        return false;
    }

    // 同じマスを次回通過時のインデックス
    let path_idx_same_vertex_i_next = path_idx_same_vertex_next(
        path_i_cur,
        paths,
        path_idx_to_vertex_pass_count,
        vertex_pass_count_to_path_idx,
        vertex_pass_counts,
    );
    let path_idx_same_vertex_j_next = path_idx_same_vertex_next(
        path_j_cur,
        paths,
        path_idx_to_vertex_pass_count,
        vertex_pass_count_to_path_idx,
        vertex_pass_counts,
    );

    // 一行目はインデント目的...
    if path_i_cur == path_j_cur
        // 直前に通過するマスの遷移規則出力が設定されていない
        //   => 出力は最後にまとめるでよいので書かない
        // 直前に通過するマスの遷移規則入力が設定されていない
        //   => 共通化済だと詰むことがあるので, ひとまず全部避ける...でもないはずなんだけれどなぁ
        || !(path_i_cur > 0 && path_rule_in[path_i_cur - 1].is_none())
        || !(path_j_cur > 0 && path_rule_in[path_j_cur - 1].is_none())

        // 今回通過するマスの入力が設定済でない
        || !(path_rule_in[path_i_cur].is_none() && path_rule_in[path_j_cur].is_none())
        // 今回通過するマスの出力が設定済でない
        //   => 出力は最後にまとめるでよいので書かない
        // 今回通過するマスについて, 前回通過時の遷移規則出力にて塗りつぶす色が設定されていない
        //   => 出力は最後にまとめるでよいので書かない
        // 今回通過するマスの次回通過時の遷移規則入力にて色が設定されていない
        || !(path_idx_same_vertex_i_next.is_none() || path_idx_same_vertex_i_next == Some(paths.len() - 1) || path_rule_in[path_idx_same_vertex_i_next.unwrap()].is_none())
        //   path_rules_in は最終パスを省いているので -1 判定
        || !(path_idx_same_vertex_j_next.is_none() || path_idx_same_vertex_j_next == Some(paths.len() - 1) || path_rule_in[path_idx_same_vertex_j_next.unwrap()].is_none())

        // 直後に通過するマスの遷移規則入力が設定されていない
        || !((path_i_cur >= paths.len() - 2 || path_rule_in[path_i_cur + 1].is_none())
                && (path_j_cur >= paths.len() - 2 || path_rule_in[path_j_cur + 1].is_none()))
    // 直後に通過するマスの前回通過時に塗りつぶす色が設定されていない
    //   => 出力は最後にまとめるでよいので書かない
    // TODO: 初期色？
    {
        return false;
    }

    // 以下の色/状態を選択する
    // - 今回割り当てる色と状態
    // - 今回のマス通過直後に通るマスの色を決める (内部状態は引き継ぐ)
    // - 今回のマスを次に通過する際の内部状態 (色は引き継ぐ)
    //   - TODO: 通らない場合はスルーできる
    // つまりは, 色 c, 内部状態 s を今回割り当てるのであれば,
    // - (c, s) が使用可能である
    // - 内部状態 s で使用可能な色が (c, s) 以外に二つ以上残っている
    // - 色 c で使用可能な内部状態が (c, s) 以外に二つ以上残っている
    let mut use_cur_color = None;
    let mut use_cur_state = None;
    let mut use_colors_after_ij = vec![];
    let mut use_status_same_vertex_next_ij = vec![];
    'use_color_state_loop: for i in 0..color_num {
        for j in 0..state_num {
            // (色 i, 内部状態 j) を今回のマスに割り当てられるか？
            // FIXME: 更新タイミングが遅いので重複し得る..
            if status_assigned_per_color[i][j] {
                continue;
            }

            // 同一色 i に対する内部状態が他に二つ余っているか
            let mut use_status_candidates = vec![];
            for jj in j + 1..state_num {
                if !status_assigned_per_color[i][jj] {
                    use_status_candidates.push(jj);
                    if use_status_candidates.len() == 2 {
                        break;
                    }
                }
            }
            if use_status_candidates.len() < 2 {
                // 今の色固定では不可
                break;
            }

            // 同一内部状態 j に対する色が他に二つ余っているか
            let mut use_colors_candidates = vec![];
            for k in 0..color_num {
                if k == i {
                    continue;
                }

                if !status_assigned_per_color[k][j] {
                    use_colors_candidates.push(k);
                    if use_colors_candidates.len() == 2 {
                        break;
                    }
                }
            }
            if use_colors_candidates.len() == 2 {
                // 採用
                use_cur_color = Some(i);
                use_cur_state = Some(j);
                use_colors_after_ij = use_colors_candidates;
                use_status_same_vertex_next_ij = use_status_candidates;
                break 'use_color_state_loop;
            }
        }
    }

    if use_cur_color.is_none() {
        return false;
    }

    let mut used_cs = vec![
        (use_cur_color.unwrap(), use_cur_state.unwrap()),
        (use_cur_color.unwrap(), use_status_same_vertex_next_ij[0]),
        (use_cur_color.unwrap(), use_status_same_vertex_next_ij[1]),
        (use_colors_after_ij[0], use_cur_state.unwrap()),
        (use_colors_after_ij[1], use_cur_state.unwrap()),
    ];
    used_cs.sort_unstable();
    used_cs.dedup();
    assert!(used_cs.len() == 5);

    // マージ対象の二つのマス **直前の** マスに対し,
    // - 今回通過時の遷移規則の出力に, 割り当てる内部状態を割り振る
    // (出力は書かない)

    // マージ対象の二つのマスに対し,
    // - 今回の行動に, 同じ遷移規則の入力を割り振る
    path_rule_in[path_i_cur] = Some(RuleIn {
        color: use_cur_color.unwrap(),
        state: use_cur_state.unwrap(),
    });
    path_rule_in[path_j_cur] = Some(RuleIn {
        color: use_cur_color.unwrap(),
        state: use_cur_state.unwrap(),
    });
    assert!(!status_assigned_per_color[use_cur_color.unwrap()][use_cur_state.unwrap()]);
    status_assigned_per_color[use_cur_color.unwrap()][use_cur_state.unwrap()] = true;

    // - 今回の行動に, 同じ遷移規則の出力を割り振る
    // (出力は書かない)
    // - 前回通過時の遷移規則出力に, 今回割り当てる色を割り振る
    // (出力は書かない)
    // - 次回通過時の遷移規則入力に, 今回割り当てる色及び独立した内部状態を割り振る
    //   - この頂点を二度と通らない場合は, これらの if にかからない
    if let Some(i) = path_idx_same_vertex_i_next
        && i < path_rule_in.len() - 1
    {
        path_rule_in[i] = Some(RuleIn {
            color: use_cur_color.unwrap(),
            state: use_status_same_vertex_next_ij[0],
        });
        assert!(!status_assigned_per_color[use_cur_color.unwrap()][use_status_same_vertex_next_ij[0]]);
        status_assigned_per_color[use_cur_color.unwrap()][use_status_same_vertex_next_ij[0]] = true;
    }
    if let Some(i) = path_idx_same_vertex_j_next
        && i < path_rule_in.len() - 1
    {
        path_rule_in[i] = Some(RuleIn {
            color: use_cur_color.unwrap(),
            state: use_status_same_vertex_next_ij[1],
        });
        assert!(!status_assigned_per_color[use_cur_color.unwrap()][use_status_same_vertex_next_ij[1]]);
        status_assigned_per_color[use_cur_color.unwrap()][use_status_same_vertex_next_ij[1]] = true;
    }

    // マージ対象の二つのマス **直後の** マスに対し,
    // - 前回通過時の遷移規則出力に, それぞれ異なる色を割り振る
    // (出力は書かない)
    // - 今回行動直後の遷移規則の入力に, それぞれ異なる色及び内部状態を割り振る
    if path_i_cur < paths.len() - 2 {
        path_rule_in[path_i_cur + 1] = Some(RuleIn {
            color: use_colors_after_ij[0],
            state: use_cur_state.unwrap(),
        });
        assert!(!status_assigned_per_color[use_colors_after_ij[0]][use_cur_state.unwrap()]);
        status_assigned_per_color[use_colors_after_ij[0]][use_cur_state.unwrap()] = true;
    }
    if path_j_cur < paths.len() - 2 {
        path_rule_in[path_j_cur + 1] = Some(RuleIn {
            color: use_colors_after_ij[1],
            state: use_cur_state.unwrap(),
        });
        assert!(!status_assigned_per_color[use_colors_after_ij[1]][use_cur_state.unwrap()]);
        status_assigned_per_color[use_colors_after_ij[1]][use_cur_state.unwrap()] = true;
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

        // 全経路に固有の (色, 内部状態) の組を割り当てるとした場合の入力条件数
        let mut ptrn_sqrt = 10;
        while ptrn_sqrt * ptrn_sqrt < paths.len() {
            ptrn_sqrt += 1;
        }
        // let (color_num, state_num) = if (ptrn_sqrt - 1) * ptrn_sqrt >= paths.len() {
        //     (ptrn_sqrt - 1, ptrn_sqrt)
        // } else {
        //     (ptrn_sqrt, ptrn_sqrt)
        // };
        let (color_num, state_num) = (ptrn_sqrt, ptrn_sqrt);

        // マス i の通過回数
        let mut vertex_pass_counts = vec![0; grid_size_1d];
        // i 番目のパスがマス j の k 度目の通過 (0-origin)
        let mut path_idx_to_vertex_pass_count = vec![];
        // マス i の j 回目の通過時のパスは全体で何番目か
        let mut vertex_pass_count_to_path_idx = vec![vec![]; grid_size_1d];
        for (i, &p) in paths.iter().enumerate() {
            path_idx_to_vertex_pass_count.push(vertex_pass_counts[p]);
            vertex_pass_count_to_path_idx[p].push(i);
            vertex_pass_counts[p] += 1;
        }

        let mut status_assigned_per_color = vec![vec![false; state_num]; color_num];
        let mut path_rule_in = vec![None; paths.len() - 1];
        // 初手は内部状態が固定
        path_rule_in[0] = Some(RuleIn { color: 0, state: 0 });
        status_assigned_per_color[0][0] = true;
        // TODO: マージ判定, とりあえず貪欲にするが実際は乱択可能
        // 最後のパスは最終目的地であり, 指示は不要であるので省く
        for i in 0..paths.len() - 1 {
            for j in i + 2..paths.len() - 1 {
                // println!("begin: {}", status_assigned_per_color[9][11]);
                // println!("path_rule_in: {path_rule_in:?}");
                // let a = path_rule_in.clone();
                if merge_moves(
                    n,
                    i,
                    j,
                    &paths,
                    &vertex_pass_counts,
                    &path_idx_to_vertex_pass_count,
                    &mut vertex_pass_count_to_path_idx,
                    &mut status_assigned_per_color,
                    &mut path_rule_in,
                ) {
                    // debug!("i: {i}, j: {j}, path_rule_in_after_merge: {path_rule_in:?}");
                    // 三点以上のマージは, 簡単のために一旦考えない
                    // println!("end: {}", status_assigned_per_color[9][11]);
                    // (9, 11) が重複するのではなく, 重複後のパスがなんかおかしいのでは？
                    // if status_assigned_per_color[9][11] {
                    //     println!("before: {a:?}");
                    //     println!("after: {path_rule_in:?}");
                    //     return;
                    // }
                    break;
                }
            }
        }
        debug!("paths: {paths:?}");
        debug!("path_rule_in_after_merge: {path_rule_in:?}");

        // マージされていない頂点に, 独立した色と内部状態とを割り振る
        let mut grid_i = 0;
        for i in 0..paths.len() - 1 {
            if path_rule_in[i].is_some() {
                continue;
            }

            loop {
                let (c, s) = ith_square_coord(grid_i);
                if !status_assigned_per_color[c][s] {
                    break;
                }

                grid_i += 1;
            }

            let (color, state) = ith_square_coord(grid_i);
            assert!(!status_assigned_per_color[color][state]);
            path_rule_in[i] = Some(RuleIn { color, state });
            status_assigned_per_color[color][state] = true;
        }

        // 初期色は 0, 一度も通過しないマスは 0 のまま
        let mut init_colors = vec![0; grid_size_1d];
        let mut rules: Vec<Vec<Option<TransitionRules>>> = vec![vec![None; state_num]; color_num];
        let mut vertex_pass_count = vec![0; grid_size_1d];
        for (i, &p) in paths.iter().take(paths.len() - 1).enumerate() {
            if vertex_pass_count[p] == 0 {
                // 初回通過時には初期色を決める
                init_colors[p] = if let Some(r) = path_rule_in[i] {
                    r.color
                } else {
                    unreachable!();
                }
            }

            // 自身のマスを次に通過した際の色
            // FIXME: 0001 他で重複
            // 7 30 7 30 L
            // 7 30 9 30 L
            let new_color = if let Some(i) = path_idx_same_vertex_next(
                i,
                &paths,
                &path_idx_to_vertex_pass_count,
                &vertex_pass_count_to_path_idx,
                &vertex_pass_counts,
            ) && i < path_rule_in.len()
            {
                path_rule_in[i].unwrap().color
            } else {
                // 自身のマスは通らずとも, 共通化した他のマスが通る可能性がある
                // とりあえずは値を入れるが, 後で上書きされるかも
                // というよかこれだけだとだめ
                path_rule_in[i].unwrap().color
            };
            let new_state = if i == paths.len() - 2 {
                0
            } else {
                path_rule_in[i + 1].unwrap().state
            };
            let dir = move_dir_from_1d(paths[i], paths[i + 1], n);

            let cin = path_rule_in[i].unwrap().color;
            let sin = path_rule_in[i].unwrap().state;
            if let Some(t) = rules[cin][sin] {
                if t.new_color == cin {
                    rules[cin][sin] = Some(TransitionRules {
                        new_color,
                        new_state,
                        dir
                    });
                }
            } else {
                rules[cin][sin] = Some(TransitionRules {
                    new_color,
                    new_state,
                    dir,
                });
            }

            vertex_pass_count[p] += 1;
        }

        let mut rules = vec![];
        for {
            rules.push();
        }

        debug!("rules: {rules:?}");
        let mut color_num = 0;
        let mut state_num = 0;
        for &c in &init_colors {
            color_num = color_num.max(c);
        }
        for &r in &rules {
            color_num = color_num.max(r.0.color);
            state_num = state_num.max(r.0.state);
        }
        color_num += 1;
        state_num += 1;
        debug!("color_num: {color_num}");
        debug!("state_num: {state_num}");
        debug!("init_colors: {:?}", init_colors);
        debug!("rules: {rules:?}");

        let cur_score = color_num + state_num;
        if cur_score < best_score {
            best_score = cur_score;
            color_num_ans = color_num;
            state_num_ans = state_num;
            init_colors_ans = init_colors;
            rules_ans = rules;
        }

        // 乱択ができていないので
        break;
    }

    println!("{color_num_ans} {state_num_ans} {}", rules_ans.len());
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
    for (rin, rout) in rules_ans {
        println!("{rin} {rout}");
    }
}
