use itertools::Itertools;
use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

// 固定
const N: usize = 30;
const M: usize = 10;
const K: usize = 10;

#[derive(Clone, Copy, Debug)]
enum Operation {
    L,
    R,
    U,
    D,
    S,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Operation::L => write!(f, "L"),
            Operation::R => write!(f, "R"),
            Operation::U => write!(f, "U"),
            Operation::D => write!(f, "D"),
            Operation::S => write!(f, "S"),
        }
    }
}

impl Operation {
    fn dir(&self) -> (isize, isize) {
        match self {
            Operation::L => (0, -1),
            Operation::R => (0, 1),
            Operation::U => (-1, 0),
            Operation::D => (1, 0),
            Operation::S => (0, 0),
        }
    }
}

fn could_move(vcur: (usize, usize), dij: (isize, isize), vn: &Vec<Vec<char>>, hn: &Vec<Vec<char>>) -> bool {
    let (di, dj) = dij;
    let ni = vcur.0.wrapping_add_signed(di);
    let nj = vcur.1.wrapping_add_signed(dj);
    if ni >= N || nj >= N {
        return false;
    }

    let is = vcur.0.min(ni);
    let ib = vcur.0.max(ni);
    let js = vcur.1.min(nj);
    let jb = vcur.1.max(nj);
    if (is != ib && hn[is][js] == '1') || (js != jb && vn[is][js] == '1') {
        return false;
    }

    true
}

fn move_pos(vcur: (usize, usize), dij: (isize, isize), vn: &Vec<Vec<char>>, hn: &Vec<Vec<char>>) -> (usize, usize) {
    if !could_move(vcur, dij, vn, hn) {
        return vcur;
    }

    let ni = vcur.0.wrapping_add_signed(dij.0);
    let nj = vcur.1.wrapping_add_signed(dij.1);
    (ni, nj)
}

fn shortest_path(vbegin: (usize, usize), vn: &Vec<Vec<char>>, hn: &Vec<Vec<char>>) -> Vec<Vec<Vec<Operation>>> {
    let ops = [Operation::L, Operation::R, Operation::U, Operation::D];
    let mut ret = vec![vec![vec![]; N]; N];
    let mut que = VecDeque::new();
    que.push_back(vbegin);
    while let Some(vcur) = que.pop_front() {
        for op in &ops {
            let vnext = move_pos(vcur, op.dir(), vn, hn);
            if vnext == vcur || vnext == vbegin || !ret[vnext.0][vnext.1].is_empty() {
                continue;
            }

            let mut pathnext = ret[vcur.0][vcur.1].clone();
            pathnext.push(*op);
            ret[vnext.0][vnext.1] = pathnext;
            que.push_back(vnext);
        }
    }

    ret
}

fn goal_order(vbegin: (usize, usize), dir: (isize, isize)) -> Option<Vec<(usize, usize)>> {
    let ni = vbegin.0.wrapping_add_signed(dir.0);
    let nj = vbegin.1.wrapping_add_signed(dir.1);
    if ni >= N || nj >= N {
        return None;
    }

    let mut ret = Vec::with_capacity(N * N);

    // thank you Gemini 2.5 Pro...
    // i は進行方向に対して垂直な軸（レーン）のインデックス
    for i in 0..N {
        // j_raw は各レーン内の進行方向のインデックス
        for j_raw in 0..N {
            // i の偶奇に応じて進行方向を反転させる
            // i=0, 2, 4... (偶数) -> 順方向 (0, 1, ..., N-1)
            // i=1, 3, 5... (奇数) -> 逆方向 (N-1, N-2, ..., 0)
            let j = if i % 2 == 0 {
                j_raw
            } else {
                N - 1 - j_raw
            };

            let (r, c); // 最終的な座標 (row, column)

            // dir.0 が 0 の場合、主な進行方向は水平（左右）
            if dir.0 == 0 {
                // [進行方向でない側（行）の決定]
                // 開始点が上辺 (vbegin.0 == 0) なら、行は i の昇順 (0, 1, 2...)
                // 開始点が下辺 (vbegin.0 == N-1) なら、行は i の降順 (N-1, N-2, ...)
                r = if vbegin.0 == 0 { i } else { N - 1 - i };

                // [進行方向側（列）の決定]
                // 初期の進行方向が右 (dir.1 > 0) なら、列は j の昇順 (0, 1, 2...)
                // 初期の進行方向が左 (dir.1 < 0) なら、列は j の降順 (N-1, N-2, ...)
                c = if dir.1 > 0 { j } else { N - 1 - j };
            }
            // dir.1 が 0 の場合、主な進行方向は垂直（上下）
            else {
                // [進行方向でない側（列）の決定]
                // 開始点が左辺 (vbegin.1 == 0) なら、列は i の昇順 (0, 1, 2...)
                // 開始点が右辺 (vbegin.1 == N-1) なら、列は i の降順 (N-1, N-2, ...)
                c = if vbegin.1 == 0 { i } else { N - 1 - i };

                // [進行方向側（行）の決定]
                // 初期の進行方向が下 (dir.0 > 0) なら、行は j の昇順 (0, 1, 2...)
                // 初期の進行方向が上 (dir.0 < 0) なら、行は j の降順 (N-1, N-2, ...)
                r = if dir.0 > 0 { j } else { N - 1 - j };
            }
            ret.push((r, c));
        }
    }

    Some(ret)
}

#[fastout]
fn main() {
    const TURN_MAX: usize = 2 * 30 * 30;

    // < 2 sec
    const RUN_TIME_MAX_MS: u64 = 1930;
    const SWAP_COUNT_MAX: usize = 10;
    const ORDER_CANDIDATE_NUM: usize = 10;

    let start_time = Instant::now();
    let break_time = Duration::from_millis(RUN_TIME_MAX_MS);
    let mut rng = SmallRng::from_entropy();

    input! {
        // 30 固定, 正方形の一辺
        n: usize,
        // 10 固定, ロボット台数
        m: usize,
        // 10 固定, ロボット初期位置
        k: usize,
        ijm: [(usize, usize); m],
        // (i, j) と (i, j+1) に壁があると 1
        vn: [Chars; n],
        // (i, j) と (i+1, j) に壁があると 1
        hn: [Chars; n - 1],
    }

    // とりあえずやるだけだと, 一つのロボットに全マスを掃除させる
    // 最大操作回数で全マス舐めたときのスコアは 3*(30^2)-2*(30^2)=900
    // 1 マスだけ舐められなかったときのスコアは 30^2-1=899
    // 結局は全マス舐めるを優先すべきではある

    // 愚直に小手先の改善を入れるなら, 一つのロボットに目標への最短経路を指示しつつ,
    // 毎ターン目標が他のロボットに偶然掃除されたか否かを判定して枝刈り, とか
    // 最適化っぽく捉えるなら, 一つのロボットに全点掃除させる経路を作って,
    // 以後他のロボットに対する指示をランダムに変えて焼き鈍す or 山登る

    // 理想的には, 各ロボットから近いマスをそのロボットに担当させると決め打ちするとか

    // 壁はそれほど厄介な形にはならなさそう？
    // ボタン割り当てを乱択するだけでそこそこの効率になるような気はするが, 芸がないような…
    // 全マスへの最短経路を一操作の度にもとうとすると, 操作回数 1800 回に辺数が 1200 で…できなくはない？
    // ランダムに動かすと端に塗り残しがあると詰むので, 外周を埋める感覚で動けるだけ動いた方がマシそう

    // というか, 一つ決め打ちして外周を回るだけでよかったのでは...
    // それぞれのロボットを端につけて時計回りして内に入っていくとか
    // 初期配置から近い壁につける分にボタン割り振るとか
    // なんでもかんでも乱択はよくない

    // TODO: サイズ固定で高速化になる部分がありそう
    let mut ans_score = 0;
    // c[i][j]: i 番目のボタン押下時のロボット j の動作
    let mut ans_button = vec![vec![Operation::S; M]; K];
    let mut ans_operation = vec![];
    let mut ans_order = vec![];

    let mut buttons = vec![vec![Operation::S; M]; K];
    for i in 0..M {
        buttons[0][i] = Operation::L;
        buttons[1][i] = Operation::R;
        buttons[2][i] = Operation::U;
        buttons[3][i] = Operation::D;
    }
    let op_to_num = |op: Operation| {
        match op {
            Operation::L => 0,
            Operation::R => 1,
            Operation::U => 2,
            Operation::D => 3,
            _ => unreachable!(),
        }
    };

    let mut operations: Vec<usize> = vec![];
    let mut robots_pos = vec![(0, 0); m];
    let mut unvisited = HashSet::new();
    let mut visited = vec![vec![false; N]; N];

    // 処理時間よい？
    let mut shortest_paths = vec![vec![vec![vec![]; n]; n]; n];
    for i in 0..N {
        for j in 0..N {
            shortest_paths[i][j] = shortest_path((i, j), &vn, &hn);
        }
    }
    // debug で 822 ms, release で 170 ms くらいかかっている, 遅い
    // println!("{:?}", start_time.elapsed());
    // return;

    let mut goal_orders = vec![];
    let vbegins = [(0, 0), (0, N - 1), (N - 1, 0), (N - 1, N - 1)];
    for vbegin in vbegins {
        for op in &[Operation::L, Operation::R, Operation::U, Operation::D] {
            // Option 使わず判定できるが, 大して時間かからないので実装の楽さを優先する
            if let Some(goal_order) = goal_order(vbegin, op.dir()) {
                goal_orders.push(goal_order);
            }
        }
    }
    // ここまで 160 ms くらい, つまりほとんど時間くってない
    // println!("{:?}", start_time.elapsed());
    // return;

    while start_time.elapsed() < break_time {
        for goal_order in &goal_orders {
            // 変数初期化
            // ボタン割り当てを決めつけ
            operations = vec![];
            unvisited.clear();
            for i in 0..N {
                for j in 0..N {
                    unvisited.insert((i, j));
                }
            }
            for i in 0..m {
                robots_pos[i] = (ijm[i].0, ijm[i].1);
            }
            for i in 0..N {
                for j in 0..N {
                    visited[i][j] = false;
                }
            }
            for &(i, j) in &ijm {
                unvisited.remove(&(i, j));
                visited[i][j] = true;
            }

            // println!("goal_order: {goal_order:?}");
            let mut idx_goal = 0;
            let mut turn_current = 0;
            while !unvisited.is_empty() && turn_current < TURN_MAX {
                // println!("  turn: {turn_current}/{TURN_MAX}");

                // 目標位置を決める
                // println!("  idx_goal: {idx_goal}");
                // println!("  unvisited: {unvisited:?}");
                while visited[goal_order[idx_goal].0][goal_order[idx_goal].1] {
                    idx_goal += 1;
                }
                let i_goal = goal_order[idx_goal].0;
                let j_goal = goal_order[idx_goal].1;

                // 目標に最も近いロボットを求める
                let mut shortest_path_robot = 0;
                let mut shortest_path_len = shortest_paths[robots_pos[0].0][robots_pos[0].1][i_goal][j_goal].len();
                for i in 1..M {
                    let cur_len = shortest_paths[robots_pos[i].0][robots_pos[i].1][i_goal][j_goal].len();
                    if cur_len < shortest_path_len {
                        shortest_path_robot = i;
                        shortest_path_len = cur_len;
                    }
                }
                // println!("goal: {:?}", goal_order[idx_goal]);
                // println!("  from robot[{}]", shortest_path_robot);
                // println!("  path: {:?}", shortest_paths[robots_pos[shortest_path_robot].0][robots_pos[shortest_path_robot].1][i_goal][j_goal]);

                for cur_op in & shortest_paths[robots_pos[shortest_path_robot].0][robots_pos[shortest_path_robot].1][i_goal][j_goal] {
                    operations.push(op_to_num(*cur_op));

                    // ロボット現在位置の更新
                    for i in 0..M {
                        robots_pos[i] = move_pos(robots_pos[i], cur_op.dir(), &vn, &hn);
                        let ii = robots_pos[i].0;
                        let jj = robots_pos[i].1;
                        visited[ii][jj] = true;
                        unvisited.remove(&(ii, jj));
                    }

                    turn_current += 1;
                    if turn_current == TURN_MAX {
                        // 手数超過により強制終了
                        break;
                    }
                }
            }
            // println!("{:?}", unvisited);
            // println!("{:?}", robots_pos);

            // 記録更新判定
            let score = if unvisited.is_empty() {
                3 * N * N - operations.len()
            } else {
                N * N - unvisited.len()
            };
            if score > ans_score {
                ans_score = score;
                ans_button = buttons.clone();
                ans_operation = operations.clone();
                ans_order = goal_order.clone();
            }

            // TODO: DEBUG
            // break;
        }

        // 訪問順を適当に並べ替えて登山
        // TODO: 近傍とか壁沿いとかを入れ替えたほうがよいのだろうが, だるい
        goal_orders.clear();
        for _ in 0..ORDER_CANDIDATE_NUM {
            let mut goal_order = ans_order.clone();
            for _ in 0..rng.gen::<usize>() % SWAP_COUNT_MAX + 1 {
                let i = rng.gen::<usize>() % goal_order.len();
                let j = rng.gen::<usize>() % goal_order.len();
                goal_order.swap(i, j);
            }
            goal_orders.push(goal_order);
        }
    }

    for ac in ans_button {
        println!("{}", ac.iter().join(" "));
    }
    for ao in ans_operation {
        println!("{ao}");
    }
}
