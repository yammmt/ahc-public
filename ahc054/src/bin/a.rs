use proconio::marker::Chars;
use proconio::{input, source::line::LineSource};
use std::io::{stdout, Write};

// 2 s
#[allow(dead_code)]
const TIME_LIMIT_MS: usize = 1980;

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        tij: (usize, usize),
        bnn: [Chars; n],
    }

    let mut adventurer = (0, n / 2);
    let mut is_founded = vec![vec![false; n]; n];

    loop {
        input! {
            from &mut source,
            pij: (usize, usize),
            n: usize,
            xyn: [(usize, usize); n],
        }
        adventurer = pij;
        if adventurer == tij {
            break;
        }

        for (x, y) in xyn {
            is_founded[x][y] = true;
        }

        // do nothing: simplest solution to get AC
        println!("0");
        stdout().flush().unwrap();
    }
}
