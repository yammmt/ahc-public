use proconio::{input, source::line::LineSource};
use std::collections::VecDeque;
use std::io::{Write, stdout};

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)+);
        }
    };
}

const CELL_NO_OWNER: isize = -1;

const CELL_FREE: usize = 0;
const CELL_MINE: usize = 1;
const CELL_ENEMY_LVL1: usize = 2;
const CELL_ENEMY_LVL2: usize = 3;

const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

// 2 s
#[allow(dead_code)]
const TIME_LIMIT_MS: u64 = 1980;

const N: usize = 10;
const T: usize = 100;

#[derive(Debug)]
struct Board {
    owners: Vec<Vec<isize>>,
    levels: Vec<Vec<usize>>,
    values: Vec<Vec<usize>>,
    max_level: usize,
    player_num: usize,
}

#[derive(Clone, Debug)]
struct Weight {
    free_cell: f64,
    my_cell: f64,
    enemy_cell_lvl1: f64,
    enemy_cell_lvl2: f64,
}

impl Default for Weight {
    fn default() -> Self {
        Self {
            free_cell: 1.0,
            my_cell: 1.0,
            enemy_cell_lvl1: 1.0,
            enemy_cell_lvl2: 1.0,
        }
    }
}

/// AI スコアを計算して返す.
fn ai_move_candidates(
    ps: (usize, usize),
    board: &Board,
    weights: &[Weight],
) -> Vec<Vec<(f64, (usize, usize))>> {
    let me = board.owners[ps.0][ps.1] as usize;
    let mut candidates = vec![vec![]; 4];

    // TODO: 一々メモリ確保が走るので遅い
    let mut visited = vec![vec![false; N]; N];
    let mut que = VecDeque::new();
    que.push_back(ps);
    while let Some((pi, pj)) = que.pop_front() {
        if visited[pi][pj] {
            continue;
        }

        visited[pi][pj] = true;
        if board.owners[pi][pj] == me as isize {
            if board.levels[pi][pj] < board.max_level {
                candidates[CELL_MINE]
                    .push((board.values[pi][pj] as f64 * weights[me].my_cell, (pi, pj)));
            }
        } else if board.owners[pi][pj] == CELL_NO_OWNER {
            candidates[CELL_FREE].push((
                board.values[pi][pj] as f64 * weights[me].free_cell,
                (pi, pj),
            ));
        } else if board.levels[pi][pj] == 1 {
            candidates[CELL_ENEMY_LVL1].push((
                board.values[pi][pj] as f64 * weights[me].enemy_cell_lvl1,
                (pi, pj),
            ));
        } else {
            candidates[CELL_ENEMY_LVL2].push((
                board.values[pi][pj] as f64 * weights[me].enemy_cell_lvl2,
                (pi, pj),
            ));
        }

        // 今が自身領土でなければ移動不可
        if board.owners[pi][pj] != me as isize {
            continue;
        }

        for d in &DIRS {
            let ni = pi.wrapping_add_signed(d.0);
            let nj = pj.wrapping_add_signed(d.1);
            if ni >= N
                || nj >= N
                || visited[ni][nj]
                || (board.owners[ni][nj] != me as isize && board.owners[ni][nj] != CELL_NO_OWNER)
            {
                continue;
            }

            que.push_back((ni, nj));
        }
    }

    // desc
    candidates[CELL_FREE].sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    candidates[CELL_MINE].sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    candidates[CELL_ENEMY_LVL1].sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
    candidates[CELL_ENEMY_LVL2].sort_unstable_by(|a, b| b.0.total_cmp(&a.0));

    candidates
}

/// 与えられた行動候補からもっともそれらしい行動を返す.
fn actual_ai_move(candidates: &Vec<Vec<(f64, (usize, usize))>>) -> (usize, usize) {
    let mut ret = (-1.0, (0, 0));
    // 二次元配列の参照順を逆にすると定数倍高速化
    for i in 0..4 {
        if let Some(&c) = candidates[i].first()
            && c.0 > ret.0
        {
            ret = c;
        }
    }

    ret.1
}

/// 現在の盤面のスコアを O(N^2) で計算して返す.
fn calc_scores(board: &Board) -> Vec<usize> {
    let mut ret = vec![0; board.player_num];

    for i in 0..N {
        for j in 0..N {
            if board.owners[i][j] == CELL_NO_OWNER {
                continue;
            }

            ret[board.owners[i][j] as usize] += board.levels[i][j] * board.values[i][j];
        }
    }

    ret
}

/// 移動を反映したスコアを返す.
/// ここで返すスコアは厳密なスコアでなく, 本来のスコア計算に用いられる S_0/S_A を返す.
/// 不正な移動は弾かない.
/// 現状, スコアを計算するだけであり, 盤面の更新は行っていない. このため複数手先を予想することはできない.
fn simulate_update(moves_to: &[(usize, usize)], scores: &[usize], board: &Board) -> f64 {
    // 競合解決
    let mut actual_moves = vec![None; board.player_num];
    for i in 0..board.player_num {
        actual_moves[i] = Some(moves_to[i]);
        let (mi, mj) = moves_to[i];
        for j in 0..i {
            if actual_moves[i] == actual_moves[j] {
                if board.owners[mi][mj] == i as isize {
                    actual_moves[j] = None;
                } else if board.owners[mi][mj] == j as isize {
                    actual_moves[i] = None;
                } else {
                    actual_moves[i] = None;
                    actual_moves[j] = None;
                }
            }
        }
    }

    // スコア差分の計算
    let mut new_scores = scores.to_vec();
    for i in 0..board.player_num {
        let Some((mi, mj)) = actual_moves[i] else {
            continue;
        };

        if board.owners[mi][mj] == CELL_NO_OWNER {
            // 占領: 誰のマスでもない
            new_scores[i] += board.values[mi][mj];
        } else if board.owners[mi][mj] == i as isize && board.levels[mi][mj] < board.max_level {
            // 強化
            new_scores[i] += board.values[mi][mj];
        } else {
            // 攻撃
            let j = board.owners[mi][mj] as usize;
            new_scores[j] -= board.values[mi][mj];
            if board.levels[mi][mj] == 1 {
                new_scores[i] += board.values[mi][mj];
            }
        }
    }

    let enemy_max = *new_scores.iter().skip(1).max().unwrap();
    new_scores[0] as f64 / enemy_max as f64
}

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        // ここは不要かも
        from &mut source,
        _n: usize,
        player_num: usize, // m
        _t: usize,
        max_level: usize,
        values: [[usize; N]; N], // v
        // 初期領土は一マスのみ
        sxym: [(usize, usize); player_num],
    }

    // player_pos これで保存されたっけか
    let player_pos = sxym.clone();
    let owners = vec![vec![CELL_NO_OWNER; N]; N];
    let levels = vec![vec![0; N]; N];
    let mut board = Board {
        owners,
        levels,
        values,
        max_level,
        player_num,
    };
    for (i, p) in player_pos.iter().enumerate() {
        board.owners[p.0][p.1] = i as isize;
        board.levels[p.0][p.1] = 1;
    }

    let weights = vec![Weight::default(); player_num];

    for _ in 0..T {
        let score = calc_scores(&board);

        // TODO: 一々メモリ確保走るので遅い
        let mut move_candidates = vec![vec![]; player_num];
        // 敵
        for i in 1..player_num {
            // BFS でおけるマスを全部調べる
            move_candidates[i] = ai_move_candidates(player_pos[i], &board, &weights)
        }

        // AI の行動を決め打ちする
        let mut all_moves = vec![(0, 0); player_num];
        for i in 1..player_num {
            all_moves[i] = actual_ai_move(&move_candidates[i]);
        }

        // 自身の行動すべてに対してスコア変化を計算する
        // TODO: BFS また書くの？
        let mut my_move = (0.0, (0, 0));
        {
            let mut que = VecDeque::new();
            let mut visited = vec![vec![false; N]; N];
            que.push_back(player_pos[0]);
            while let Some((pi, pj)) = que.pop_front() {
                if visited[pi][pj] {
                    continue;
                }

                visited[pi][pj] = true;

                all_moves[0] = (pi, pj);
                let score_cur = simulate_update(&all_moves, &score, &board);
                if score_cur > my_move.0 {
                    my_move = (score_cur, (pi, pj));
                }

                // 今が自身領土でなければ移動不可
                if board.owners[pi][pj] != 0 {
                    continue;
                }

                for d in &DIRS {
                    let ni = pi.wrapping_add_signed(d.0);
                    let nj = pj.wrapping_add_signed(d.1);
                    if ni >= N
                        || nj >= N
                        || visited[ni][nj]
                        || (board.owners[ni][nj] != 0 && board.owners[ni][nj] != CELL_NO_OWNER)
                    {
                        continue;
                    }

                    que.push_back((ni, nj));
                }
            }
        }

        // TODO: なにか出す
        println!("{} {}", my_move.1.0, my_move.1.1);
        stdout().flush().unwrap();

        // 読み込む
        // TODO: 一々メモリ確保走るので遅い
        input! {
            from &mut source,
            // 駒の移動先は捨てる, 現在位置とマスの現況さえわかればよいので
            _txym: [(usize, usize); player_num],
            // ターン終了時駒位置
            player_pos: [(usize, usize); player_num],
            // 各マスの所有者
            owners: [[isize; N]; N],
            // レベル
            levels: [[usize; N]; N],
        }
        board.owners = owners;
        board.levels = levels;
    }
}
