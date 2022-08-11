use proconio::input;
use proconio::marker::Chars;
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
        cnn: [Chars; n],
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
    let x_move: Vec<Move> = vec![];

    let (y_connect, _cable) = greedy_ans(max_k, &cnn);

    println!("{}", x_move.len());
    for x in &x_move {
        println!("{}", x);
    }
    println!("{}", y_connect.len());
    for y in &y_connect {
        println!("{}", y);
    }
}
