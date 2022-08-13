use proconio::input;
use proconio::marker::Chars;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

// 種類数は小さめ
const MAX_KIND_NUM: usize = 6;
// 実行制限 3000ms に対し入出力の手間を省いてこれだけあれば余裕あるはず
const LONGEST_EXEC_TIME_MS: u64 = 2700;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Move(usize, usize, usize, usize);

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {} {}", self.0, self.1, self.2, self.3)
    }
}

#[derive(Clone, Debug)]
struct Connect(usize, usize, usize, usize);

impl fmt::Display for Connect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {} {}", self.0, self.1, self.2, self.3)
    }
}

// BFS して今のスコアを計算する
// スコアは負数となりうる
fn calc_score(cnn: &[Vec<char>], conns: &[Connect]) -> i64 {
    let mut ret = 0;
    let n = cnn[0].len();

    let mut edges = vec![vec![]; n * n];
    for e in conns {
        let v0 = e.0 * n + e.1;
        let v1 = e.2 * n + e.3;
        edges[v0].push((e.2, e.3));
        edges[v1].push((e.0, e.1));
    }

    let mut visited = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            if visited[i][j] || cnn[i][j] == '0' {
                continue;
            }

            let mut member_num = vec![0; MAX_KIND_NUM];
            let mut que = VecDeque::new();
            que.push_back((i, j));
            visited[i][j] = true;
            while let Some(cur) = que.pop_front() {
                member_num[(cnn[cur.0][cur.1] as u8 - b'0') as usize] += 1;

                for &v in &edges[cur.0 * n + cur.1] {
                    if visited[v.0][v.1] {
                        continue;
                    }

                    que.push_back((v.0, v.1));
                    visited[v.0][v.1] = true;
                }
            }

            for &m in &member_num {
                if m > 1 {
                    ret += (m * (m - 1)) / 2;
                }
            }
            for m0 in 0..MAX_KIND_NUM {
                for m1 in m0 + 1..MAX_KIND_NUM {
                    ret -= member_num[m0] * member_num[m1];
                }
            }
        }
    }

    ret
}

// 始点を固定して右/下方向を検索して同じやつが出る限り結ぶ
fn greedy_ans(available_k: usize, cnn: &[Vec<char>]) -> (Vec<Connect>, Vec<Vec<char>>) {
    let n = cnn[0].len();
    let mut y_connect = vec![];
    let mut cable = vec![vec!['0'; n]; n];
    let mut cur_k = 0;

    // -> 右
    'search_r: for i in 0..n {
        let mut prev_c = cnn[i][0];
        let mut prev_j = 0;
        for j in 1..n {
            if cnn[i][j] == '0' {
                continue;
            } else if cnn[i][j] == prev_c {
                if cur_k + 1 > available_k {
                    break 'search_r;
                }

                y_connect.push(Connect(i, prev_j, i, j));
                for jj in prev_j..j + 1 {
                    assert_ne!(cnn[i][j], '0');
                    cable[i][jj] = cnn[i][j];
                }
                cur_k += 1;
                prev_j = j;
            } else {
                prev_c = cnn[i][j];
                prev_j = j;
            }
        }
    }

    // -> 下はクロスしない程度に
    'search_b: for j in 0..n {
        let mut prev_c = cnn[0][j];
        let mut prev_i = 0;
        for i in 1..n {
            // 今のマスが 0 かつ cable なし => 継続
            // 今のマスが 0 かつ cable あり => 基点リセット
            // 今のマスが非 0 かつ基点と等しい => 接続
            // 今のマスが非 0 かつ基点と等しくない => 基点リセット
            if cnn[i][j] == '0' {
                if cable[i][j] == '0' {
                    continue;
                } else {
                    prev_c = '0';
                    prev_i = i;
                }
            } else {
                if cnn[i][j] == prev_c {
                    if cur_k + 1 > available_k {
                        break 'search_b;
                    }

                    y_connect.push(Connect(prev_i, j, i, j));
                    cur_k += 1;

                    // 種類は更新なし
                    prev_i = i;
                } else {
                    prev_c = cnn[i][j];
                    prev_i = i;
                }
            }
        }
    }

    (y_connect, cable)
}

fn main() {
    let start_time = Instant::now();

    input! {
        n: usize,
        k: usize,
        mut cnn: [Chars; n],
    }

    let dir = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    // 小さなクラスタを乱立させるより巨大なクラスタにまとめた方が良い
    // 同種の a 個のクラスタと b 個のクラスタをマージさせると得られる点は +ab
    // その際に c 個の異種クラスタが入ると -(a+b)c
    // 異種クラスタを認める条件は最低限 ab > (a+b)c になる

    // とりあえず正の得点を取る
    // N <= 48 より O(N^4) なら間に合うし結ぶ際には距離は不問なので右/下全部見ても平均的には良化しそう

    let max_k = 100 * k;
    let mut x_move: Vec<Move> = vec![];

    // 初期状態
    let (mut y_connect, _cable) = greedy_ans(max_k, &cnn);
    let mut ans_score = calc_score(&cnn, &y_connect);

    // 山登り法: 適当に移動させてスコアが上がるようなら上げてやる
    // TODO: 無駄な移動を積み重ねてマージさせたほうが良くなる場合がある (焼きなまし)
    let time_limit_ms = Duration::from_millis(LONGEST_EXEC_TIME_MS);
    let mut rng = SmallRng::from_entropy();
    while start_time.elapsed() < time_limit_ms {
        let mut cur_cnn = cnn.clone();
        let mut non_zeros: Vec<(usize, usize)> = vec![];
        for i in 0..n {
            for j in 0..n {
                if cur_cnn[i][j] != '0' {
                    non_zeros.push((i, j));
                }
            }
        }

        // 任意の非 0 マスを任意の 0 マスに動かす
        // 制約より non_zeros は空でない
        let move_from: (usize, usize) = non_zeros[rng.gen::<usize>() % non_zeros.len()];
        let cur_dir: (isize, isize) = dir[rng.gen::<usize>() % dir.len()];
        let next_i_i = move_from.0 as isize + cur_dir.0;
        let next_j_i = move_from.1 as isize + cur_dir.1;
        if next_i_i < 0 || next_i_i >= n as isize || next_j_i < 0 || next_j_i >= n as isize {
            // 範囲外で移動不可
            continue;
        }

        let next_i_u = next_i_i as usize;
        let next_j_u = next_j_i as usize;
        if cur_cnn[next_i_u][next_j_u] != '0' {
            // 移動不可
            continue;
        }

        let mut cur_x_move = x_move.clone();
        cur_x_move.push(Move(move_from.0, move_from.1, next_i_u, next_j_u));
        cur_cnn[next_i_u][next_j_u] = cur_cnn[move_from.0][move_from.1];
        cur_cnn[move_from.0][move_from.1] = '0';

        // スコアの計算と比較
        let (cur_y_connect, _cable) = greedy_ans(max_k - cur_x_move.len(), &cur_cnn);
        let cur_score = calc_score(&cur_cnn, &cur_y_connect);
        if cur_score > ans_score {
            ans_score = cur_score;
            cnn = cur_cnn;
            x_move = cur_x_move;
            y_connect = cur_y_connect;
        }
    }

    println!("{}", x_move.len());
    for x in &x_move {
        println!("{}", x);
    }
    println!("{}", y_connect.len());
    for y in &y_connect {
        println!("{}", y);
    }
}
