use proconio::{input, source::line::LineSource};
use std::io::{stdout, Write};

// enum だと入力との変換がめんどそうだから妥協
const NORMAL_WORK: usize = 0;
const SUPER_WORK: usize = 1;
const CANCEL: usize = 2;
const CHANGE_ALL: usize = 3;
const INCREASE: usize = 4;

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        m: usize,
        k: usize,
        t: usize,
        mut twn: [(usize, usize); n],
        mut hvm: [(usize, usize); m],
    }

    // とりあえず貪欲に, 効率 (価値/残務量) 最大のものに挑む, を繰り返す
    // TODO: 全力労働で潰せるなら潰す, そうでなければ通常労働のうちオーバーキルしない程度のもの？
    // 増資はとっとと使ったほうがよい？残務量と労力を同じ数倍するわけで,
    // ターン消費に対する獲得価値は上がるはず

    for _i in 0..t {
        // println!("turn: {i}");

        // 効率順
        // (価値/残務量, i)
        let mut work_efficiency = vec![];
        for (i, (h, v)) in hvm.iter().enumerate() {
            work_efficiency.push((*v as f32 / *h as f32, i));
        }
        work_efficiency.sort_unstable_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap()
        });
        work_efficiency.reverse();
        let wi_do = work_efficiency[0].1;
        let wi_cancel = work_efficiency[m - 1].1;

        // (最高効率の残務量 - w に重みをつけたもの, 全力？, i)
        // 重み: 絶対値を取った値に対し, 絶対値を取る前の値が非負であれば x2 する
        //      過労働をちょっと防ぐ
        let work_cost = |w_target, w_cur| {
            if w_target >= w_cur {
                (w_target - w_cur) * 2
            } else {
                w_cur - w_target
            }
        };
        let mut work_cards = vec![];
        for (i, (t, w)) in twn.iter().enumerate() {
            match *t {
                NORMAL_WORK => {
                    let cur = work_cost(hvm[wi_do].0, *w);
                    work_cards.push((cur, 1, i))
                },
                SUPER_WORK => {
                    let cur = work_cost(hvm[wi_do].0, *w);
                    work_cards.push((cur, 0, i))
                }
                _ => {},
            }
        }
        if work_cards.is_empty() {
            // TODO: 労働カードが手元にないので適当に流す
            // 常に 0 を選択している以上は初回以外ではここを通り得ない
            // 現在最高効率の仕事を捨てるのはもったいないので, 最悪効率の仕事を捨てられるなら捨てる
            let mut could_cancel = false;
            for (i, (t, _w)) in twn.iter().enumerate() {
                match *t {
                    CANCEL => {
                        println!("{i} {wi_cancel}");
                        could_cancel = true;
                        break;
                    },
                    _ => {},
                }
            }
            if !could_cancel {
                // 祈る
                println!("0 0");
            }
        } else {
            println!(
                "{} {}",
                work_cards[0].2,
                if work_cards[0].1 == 0 {
                    0
                } else {
                    wi_do
                }
            );
        }
        stdout().flush().unwrap();

        input! {
            from &mut source,
            hvm_nxt: [(usize, usize); m],
            _money: usize,
            _twpk_nxt: [(usize, usize, usize); k],
        }
        hvm = hvm_nxt;
        // println!("{:?}", twpk);
        // TODO: "0" は労働力 1 という最弱手
        println!("0");
    }
}
