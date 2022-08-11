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

    // とりあえず正の得点を取る
    // 右/下が同じ種類のときだけ結ぶ
    // N <= 48 より O(N^4) なら間に合う
    let mut cur_k = 0;
    let x_move: Vec<Move> = vec![];
    let mut y_connect: Vec<Connect> = vec![];

    // -> 右
    'outer_r: for i in 0..n {
        for j in 0..n - 1 {
            // もう少し早く枝刈りできるが捨て方針を言い訳に
            if cur_k == 100 * k {
                break 'outer_r;
            }

            if cnn[i][j] != '0' && cnn[i][j] == cnn[i][j + 1] {
                y_connect.push(Connect(i, j, i, j + 1));
                cur_k += 1;
            }
        }
    }

    // -> 下
    'outer_d: for j in 0..n {
        for i in 0..n - 1 {
            if cur_k == 100 * k {
                break 'outer_d;
            }

            if cnn[i][j] != '0' && cnn[i][j] == cnn[i + 1][j] {
                y_connect.push(Connect(i, j, i + 1, j));
                cur_k += 1;
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
