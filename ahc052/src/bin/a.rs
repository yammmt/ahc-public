use itertools::Itertools;
use proconio::fastout;
use proconio::input;
use proconio::marker::Chars;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::time::{Duration, Instant};

// 固定
const N: usize = 30;
const M: usize = 10;
const K: usize = 10;

#[derive(Clone, Debug)]
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

fn dir_has_unvisited(vcur: (usize, usize), dij: (isize, isize), visited: &Vec<Vec<bool>>, vn: &Vec<Vec<char>>, hn: &Vec<Vec<char>>) -> bool {
    let mut cur = vcur;
    loop {
        if !could_move(cur, dij, vn, hn) {
            return false;
        }

        let ni = cur.0.wrapping_add_signed(dij.0);
        let nj = cur.1.wrapping_add_signed(dij.1);

        if !visited[ni][nj] {
            return true;
        }

        cur = (ni, nj);
    }
}

fn move_pos(vcur: (usize, usize), dij: (isize, isize), vn: &Vec<Vec<char>>, hn: &Vec<Vec<char>>) -> (usize, usize) {
    if !could_move(vcur, dij, vn, hn) {
        return vcur;
    }

    let ni = vcur.0.wrapping_add_signed(dij.0);
    let nj = vcur.1.wrapping_add_signed(dij.1);
    (ni, nj)
}

#[fastout]
fn main() {
    const TURN_MAX: usize = 2 * 30 * 30;

    // 2 sec
    const RUN_TIME_MAX_MS: u64 = 1990;

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

    let mut buttons = vec![vec![Operation::S; M]; K];
    let mut operations: Vec<usize> = vec![];
    buttons[0][0] = Operation::L;
    buttons[1][0] = Operation::R;
    buttons[2][0] = Operation::U;
    buttons[3][0] = Operation::D;
    let mut robots_pos = vec![(0, 0); m];
    let mut unvisited = HashSet::new();
    let mut visited = vec![vec![false; N]; N];
    while start_time.elapsed() < break_time {
        // 変数初期化
        // ボタン割り当てを決めつけ
        for i in 0..4 {
            for j in 1..M {
                buttons[i][j] = match rng.gen::<usize>() % 5 {
                    0 => Operation::L,
                    1 => Operation::R,
                    2 => Operation::U,
                    3 => Operation::D,
                    4 => Operation::S,
                    _ => unreachable!(),
                };
            }
        }
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
        for &(i, j) in &ijm {
            unvisited.remove(&(i, j));
            visited[i][j] = true;
        }

        let mut turn_current = 0;
        let mut current_dir_idx = 0;
        while !unvisited.is_empty() && turn_current < TURN_MAX {
            // println!("turn: {turn_current}/{TURN_MAX}");

            if dir_has_unvisited(robots_pos[0], buttons[current_dir_idx][0].dir(), &visited, &vn, &hn) {
                operations.push(current_dir_idx);
            } else {
                let mut found= false;
                for i in 0..4 {
                    if dir_has_unvisited(robots_pos[0], buttons[current_dir_idx][0].dir(), &visited, &vn, &hn) {
                        found = true;
                        operations.push(i);
                        break;
                    }
                }
                if !found {
                    // 現在位置から上下左右方向には空きマスがない, 斜め方向やら壁回った先にはあり得る
                    // その場でくるくる回りうる事象への雑対策としての rand
                    current_dir_idx = rng.gen::<usize>() % 4;
                    operations.push(current_dir_idx);
                }
            }

            let &operation_selected = operations.last().unwrap();
            // 現在位置と到達更新
            for i in 0..m {
                robots_pos[i] = move_pos(robots_pos[i], buttons[operation_selected][i].dir(), &vn, &hn);
                let ii = robots_pos[i].0;
                let jj = robots_pos[i].1;
                visited[ii][jj] = true;
                unvisited.remove(&(ii, jj));
            }

            turn_current += 1;
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
        }

        // TODO: debug
        // break;
    }

    for ac in ans_button {
        println!("{}", ac.iter().join(" "));
    }
    for ao in ans_operation {
        println!("{ao}");
    }
}
