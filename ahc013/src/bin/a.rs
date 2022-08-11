use proconio::input;
use proconio::marker::Chars;
use std::collections::{HashSet, VecDeque};
use std::fmt;

#[derive(Debug)]
struct Move(usize, usize, usize, usize);

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {} {}", self.0, self.1, self.2, self.3)
    }
}

#[derive(Debug)]
struct Connect(usize, usize, usize, usize);

impl fmt::Display for Connect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {} {}", self.0, self.1, self.2, self.3)
    }
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
    input! {
        n: usize,
        k: usize,
        mut cnn: [Chars; n],
    }

    // 小さなクラスタを乱立させるより巨大なクラスタにまとめた方が良い
    // 同種の a 個のクラスタと b 個のクラスタをマージさせると得られる点は +ab
    // その際に c 個の異種クラスタが入ると -(a+b)c
    // 異種クラスタを認める条件は最低限 ab > (a+b)c になる

    // とりあえず正の得点を取る
    // N <= 48 より O(N^4) なら間に合うし結ぶ際には距離は不問なので右/下全部見ても平均的には良化しそう

    // 一度結べるだけ結んだ後に孤児を移動して近いグループに入れてみる
    // 接続判定が微妙に変わるので解答作成もやり直す
    // ちまちませず巨大なグループを作るべきではあるが賢い方法が浮かばないので保留

    let max_k = 100 * k;
    let mut x_move: Vec<Move> = vec![];

    // 一度愚直結合
    let (y_connect, cable) = greedy_ans(max_k, &cnn);

    // 孤児をどこかの組に混ぜるために移動する (BFS)
    // 移動後に再度愚直結合してやる
    // HACK: 手数制限のために "移動したのにマージできない" 場合にはスコアが悪化する
    let mut is_orphaned = vec![vec![true; n]; n];
    for y in &y_connect {
        is_orphaned[y.0][y.1] = false;
        is_orphaned[y.2][y.3] = false;
    }

    let dir = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for i in 0..n {
        for j in 0..n {
            if !is_orphaned[i][j] || cnn[i][j] == '0' {
                continue;
            }

            let mut visited = HashSet::new();
            // ((今の座標), これまでの経路)
            // 経路クローンが重いが盤面サイズ N が小さいので間に合うはず
            let mut que = VecDeque::new();
            que.push_back(((i, j), vec![(i, j)]));
            visited.insert((i, j));
            'x_loop: while let Some(cur) = que.pop_front() {
                let ci = (cur.0).0;
                let cj = (cur.0).1;

                for d in &dir {
                    let next_i_i = ci as isize + d.0;
                    let next_j_i = cj as isize + d.1;
                    if next_i_i < 0 || next_i_i >= n as isize || next_j_i < 0 || next_j_i >= n as isize {
                        continue;
                    }

                    // TODO: 上下左右に直進する方向であればもう結べる
                    //       愚直移動は手数を圧迫するので移動数は減らしたい
                    let next_i_u = next_i_i as usize;
                    let next_j_u = next_j_i as usize;
                    let next_ij = (next_i_u, next_j_u);
                    if cable[next_i_u][next_j_u] == cnn[i][j] {
                        // goal
                        for ei in 0..cur.1.len() - 1 {
                            x_move.push(Move(cur.1[ei].0, cur.1[ei].1, cur.1[ei+1].0, cur.1[ei+1].1));
                        }
                        let tmp = cnn[i][j];
                        cnn[i][j] = cnn[ci][cj];
                        cnn[ci][cj] = tmp;
                        is_orphaned[ci][cj] = false;
                        break 'x_loop;
                    }

                    if visited.contains(&next_ij) || cnn[next_i_u][next_j_u] != '0' {
                        continue;
                    }

                    if 2 * (cur.1.len() + x_move.len()) + 1 >= max_k - y_connect.len() {
                        // 移動制限かかりそうなのでとりあえず諦める
                        break 'x_loop;
                    }

                    visited.insert(next_ij);
                    let mut edges = cur.1.clone();
                    edges.push(next_ij);
                    que.push_back((next_ij, edges));
                }
            }
        }
    }
    let (y_connect, _cable) = greedy_ans(max_k - x_move.len(), &cnn);
    assert!(x_move.len() + y_connect.len() <= max_k);

    println!("{}", x_move.len());
    for x in &x_move {
        println!("{}", x);
    }
    println!("{}", y_connect.len());
    for y in &y_connect {
        println!("{}", y);
    }
}
