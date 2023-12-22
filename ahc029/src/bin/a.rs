use proconio::{input, source::line::LineSource};
use std::io::{stdout, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        m: usize,
        k: usize,
        t: usize,
        twn: [(usize, isize); n],
        hvm: [(usize, isize); m],
    }

    // サンプルを真似して常に 0 を選ぶ
    // TODO: とりあえず貪欲に, 効率 (価値/残務量) 最大のものに挑む, を繰り返す

    for i in 0..t {
        // println!("turn: {i}");
        println!("0 0");

        input! {
            from &mut source,
            hvm: [(usize, isize); m],
            money: usize,
            twpk: [(usize, isize, isize); k],
        }
        // println!("{:?}", twpk);
        println!("0");
        stdout().flush().unwrap();
    }
}
