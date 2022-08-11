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
    // 右/下が同じ種類のときだけ結ぶ
    // N <= 48 より O(N^4) なら間に合うし結ぶ際には距離は不問なので右/下全部見ても平均的には良化しそう
    let max_k = 100 * k;
    let mut cur_k = 0;
    let x_move: Vec<Move> = vec![];
    let mut y_connect: Vec<Connect> = vec![];
    let mut is_cable_area = vec![vec![false; n]; n];

    // 始点を固定して右/下方向を検索して同じやつが出る限り結ぶ
    // -> 右
    'search_r: for i in 0..n {
        let mut prev_c = cnn[i][0];
        let mut prev_j = 0;
        for j in 1..n - 1 {
            if cnn[i][j] == '0' {
                continue;
            } else if cnn[i][j] == prev_c {
                y_connect.push(Connect(i, prev_j, i, j));
                for jj in prev_j + 1..j {
                    is_cable_area[i][jj] = true;
                }
                cur_k += 1;
                if cur_k == max_k {
                    break 'search_r;
                }

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
        for i in 1..n - 1 {
            if is_cable_area[i][j] {
                prev_c = '0';
                prev_i = i;
            } else if cnn[i][j] == '0' {
                continue;
            } else if cnn[i][j] == prev_c {
                y_connect.push(Connect(prev_i, j, i, j));
                cur_k += 1;
                if cur_k == max_k {
                    break 'search_b;
                }

                prev_i = i;
            } else {
                prev_c = cnn[i][j];
                prev_i = i;
            }
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
