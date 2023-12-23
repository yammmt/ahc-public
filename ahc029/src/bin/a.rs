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

    // 勘
    let use_increase_turn_last = t * 9 / 10;

    // とりあえず貪欲に, 効率 (価値/残務量) 最大のものに挑む, を繰り返す
    // 増資があるなら使う. 残務量と労力を同じ数倍するわけで, ターン消費に対する獲得価値は上がるはず
    // TODO: 全力労働で潰せるなら潰す, そうでなければ通常労働のうちオーバーキルしない程度のもの？
    // TODO: 価値/労働力 < 1 はとっとと捨てたほうがよさそう, 期待値が 1 になりそう

    for ti in 0..t {
        println!("# turn: {ti}");

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
        let mut increase_cards = vec![];
        for (i, (t, w)) in twn.iter().enumerate() {
            match *t {
                NORMAL_WORK => {
                    let cur = work_cost(hvm[wi_do].0, *w);
                    work_cards.push((cur, 1, i))
                },
                SUPER_WORK => {
                    let cur = work_cost(hvm[wi_do].0, *w);
                    work_cards.push((cur, 0, i))
                },
                INCREASE => {
                    increase_cards.push(i);
                }
                _ => {},
            }
        }

        let mut card_i_used = 0;
        if !increase_cards.is_empty() {
            println!("# increase");
            card_i_used = increase_cards[0];
            println!("{card_i_used} 0");
        } else if work_cards.is_empty() {
            // 労働カードと増資カードが手元にないので適当に流す
            // 現在最高効率の仕事を捨てるのはもったいないので, 最悪効率の仕事を捨てられるなら捨てる
            let mut could_cancel = false;
            for (i, (t, _w)) in twn.iter().enumerate() {
                match *t {
                    CANCEL => {
                        println!("# cancel");
                        card_i_used = i;
                        println!("{i} {wi_cancel}");
                        could_cancel = true;
                        break;
                    },
                    _ => {},
                }
            }
            if !could_cancel {
                // 祈る
                println!("# pray");
                // card_i_used = 0;
                println!("0 0");
            }
        } else {
            println!("# work");
            card_i_used = work_cards[0].2;
            println!(
                "{card_i_used} {}",
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
            money: usize,
            twpk_nxt: [(usize, usize, usize); k],
        }
        hvm = hvm_nxt;

        // 取得方針
        // - 増資があれば取る
        // - 労働力 >= 費用の札があれば取る
        // - 今のプロジェクトが悪効率ならキャンセル取りたいけど
        let mut increases = vec![];
        let mut works = vec![];
        for (i, (t, w, p)) in twpk_nxt.iter().enumerate() {
            match *t {
                NORMAL_WORK => {
                    if *p <= money && w >= p {
                        works.push((w - p, 1, i));
                    }
                },
                SUPER_WORK => {
                    if *p <= money && w * n >= *p {
                        works.push((w * n - p, 0, i));
                    }
                }
                INCREASE => {
                    if *p <= money {
                        increases.push((p, i));
                    }
                }
                _ => {},
            }
        }
        // 価格昇順
        increases.sort_unstable();
        // w-p 降順
        works.sort_unstable();
        works.reverse();

        let card_i_get = if ti <= use_increase_turn_last && !increases.is_empty() {
            increases[0].1
        } else {
            // "0" は労働力 1 という最弱手であり避けられるなら避けたい
            works[0].2
        };

        println!("{card_i_get}");
        twn[card_i_used] = (twpk_nxt[card_i_get].0, twpk_nxt[card_i_get].1);
        // println!("{:?}", twpk);
    }
}
