use proconio::{input, source::line::LineSource};
use std::io::{stdout, Write};

// enum だと入力との変換がめんどそうだから妥協
const NORMAL_WORK: usize = 0;
const SUPER_WORK: usize = 1;
const CANCEL_ONE: usize = 2;
const CANCEL_ALL: usize = 3;
const BOOST: usize = 4;

const BOOST_USE_MAX: usize = 20;

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
    // そもそも一切のリスクを取らないほうが良いタイミングがありそうなもの
    // 手札を先読みできないので貪欲で回す方針は合理的なはず
    let use_boost_turn_last = t * 9 / 10;
    let use_cancel_turn_last = t * 80 / 100;

    // とりあえず貪欲に, 効率 (価値/残務量) 最大のものに挑む, を繰り返す
    // 増資があるなら使う. 残務量と労力を同じ数倍するわけで, ターン消費に対する獲得価値は上がるはず
    // TODO: 全力労働で潰せるなら潰す, そうでなければ通常労働のうちオーバーキルしない程度のもの？

    let mut boost_use_cnt = 0;
    for ti in 0..t {
        println!("# turn: {ti}");

        // 効率順
        // (価値/残務量, 残務量, i)
        let mut work_efficiency = vec![];
        for (i, (h, v)) in hvm.iter().enumerate() {
            work_efficiency.push((*v as f32 / *h as f32, *h, i));
        }
        // 効率が変わらなければ, 残務量が小さいほどよい仕事
        work_efficiency.sort_unstable_by(|a, b| {
            if a.0 != b.0 {
                a.0.partial_cmp(&b.0).unwrap()
            } else {
                a.1.cmp(&b.1)
            }
        });
        work_efficiency.reverse();
        let wi_do = work_efficiency[0].2;
        let wi_cancel = work_efficiency[m - 1].2;

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
        let mut cancel_cards = vec![];
        let mut boost_cards = vec![];
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
                CANCEL_ONE => {
                    cancel_cards.push(i);
                }
                CANCEL_ALL => {
                    // /2: 勘
                    cancel_cards.push(i);
                }
                BOOST => {
                    boost_cards.push(i);
                }
                _ => {},
            }
        }

        let mut card_i_used = 0;
        if !boost_cards.is_empty() && boost_use_cnt < BOOST_USE_MAX {
            println!("# boost");
            card_i_used = boost_cards[0];
            boost_use_cnt += 1;
            println!("{card_i_used} 0");
        } else if work_efficiency[0].0 < 1.0 && !cancel_cards.is_empty() {
            println!("# cancel");
            card_i_used = cancel_cards[0];
            println!(
                "{card_i_used} {}",
                if twn[cancel_cards[0]].1 == 0 {
                    0
                } else {
                    wi_cancel
                }
            );
        } else if work_cards.is_empty() {
            // 労働カードと増資カードとキャンセルカードが手元にないので適当に流す
            // そんなことある？
            // 現在最高効率の仕事を捨てるのはもったいないので, 最悪効率の仕事を捨てられるなら捨てる
            let mut could_cancel = false;
            for (i, (t, _w)) in twn.iter().enumerate() {
                match *t {
                    CANCEL_ONE => {
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

        // プロジェクトが全部非効率か？
        let mut prj_all_bad = true;
        for (h, v) in &hvm {
            if v > h {
                prj_all_bad = false;
                break;
            }
        }

        // 取得方針はこの順
        // - 増資があれば取る
        // - 今のプロジェクトがすべて悪効率ならキャンセルを取る
        //    - 全キャンセル > 個別キャンセル
        // - 労働力 >= 費用の札があれば取る
        //    - 必ず消費 0 労力 1 が配られる
        let mut boosts = vec![];
        let mut cancels = vec![];
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
                CANCEL_ONE => {
                    if *p <= money {
                        cancels.push((*w, 1, i));
                    }
                }
                CANCEL_ALL => {
                    if *p <= money {
                        // /2: 勘
                        cancels.push((*w / 2, 0, i));
                    }
                }
                BOOST => {
                    if *p <= money {
                        boosts.push((p, i));
                    }
                }
                _ => {}
            }
        }
        // 価格昇順
        boosts.sort_unstable();
        // 費用安い順
        cancels.sort_unstable();
        // w-p 降順
        works.sort_unstable();
        works.reverse();

        let card_i_get = if ti <= use_boost_turn_last
            && !boosts.is_empty()
            && boost_use_cnt < BOOST_USE_MAX
        {
            boosts[0].1
        } else if ti <= use_cancel_turn_last && !cancels.is_empty() && prj_all_bad {
            cancels[0].2
        } else {
            // "0" は労働力 1 という最弱手であり避けられるなら避けたい
            works[0].2
        };

        println!("{card_i_get}");
        twn[card_i_used] = (twpk_nxt[card_i_get].0, twpk_nxt[card_i_get].1);
        // println!("{:?}", twpk);
    }
}
