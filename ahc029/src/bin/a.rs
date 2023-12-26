use proconio::{input, source::line::LineSource};
use std::cmp::{Ordering, Reverse};
use std::io::{stdout, Write};

// enum だと入力との変換がめんどそうだから妥協
const NORMAL_WORK: usize = 0;
const SUPER_WORK: usize = 1;
const CANCEL_ONE: usize = 2;
const CANCEL_ALL: usize = 3;
const BOOST: usize = 4;

const BOOST_USE_MAX: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Project {
    h: usize,
    v: usize,
}

impl Project {
    fn is_good(&self, boost_use_num: usize) -> bool {
        let done_soon = {
            self.h < 2usize.pow(boost_use_num as u32)
        };
        let good_efficiency = {
            self.v > self.h
        };

        done_soon || good_efficiency
    }

    fn efficiency(&self) -> isize {
        // TODO: よい感じの効率指標を作るとよさそう
        self.v as isize - self.h as isize
    }
}

impl Ord for Project {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.efficiency();
        let b = other.efficiency();
        if a != b {
            // v-h 降順
            b.partial_cmp(&a).unwrap()
        } else {
            self.h.cmp(&other.h)
        }
    }
}

impl PartialOrd for Project {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

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
        hvm: [(usize, usize); m],
    }
    let mut projects = Vec::with_capacity(m);
    for (h, v) in hvm {
        projects.push(Project { h, v });
    }

    // 勘
    // そもそも一切のリスクを取らないほうが良いタイミングがありそうなもの
    // 手札を先読みできないので貪欲で回す方針は合理的なはず
    let use_cost_turn_last = t * 95 / 100;
    let use_boost_turn_last = t * 9 / 10;
    let use_cancel_turn_last = t * 80 / 100;

    // 平均値で考えると, 増資がなければ t=0 は平均で労働力コストともに 25
    // プロジェクトは残務量価値ともに 32 となる
    // 所持金 50 として, 労働 1 を 32 回続けると 32 ターン後に所持金 82
    // 初手で (work, cost) = (25, 25) の労働を取り残り労働 1 を 7 回続けると 8 ターン後に所持金 57
    // 前者は 32 ターンで価値 32, 後者は 8 ターンで価値 7 を取っている,
    // とすると後者が悪効率となる
    // つまりは終了間際を除き, 労働札は高効率札以外は取らない方がよい？
    // 上の例では (26, 25) 以上であれば札を取ることで効率が上がる
    // でも早期にはとりあえず所持金を得ることで選択肢が増えるわけで

    let mut boost_use_cnt = 0;
    let sort_prj_w_idx = |vp: &[Project]| {
        let mut ret = Vec::with_capacity(m);
        for (i, &vp) in vp.iter().enumerate() {
            ret.push((vp, i));
        }
        ret.sort_unstable();
        ret
    };
    let mut vp = sort_prj_w_idx(&projects);
    for ti in 0..t {
        println!("# turn: {ti}");

        // println!("# vp: {:?}", vp);
        let wi_do = vp[0].1;
        let wi_cancel = vp[m - 1].1;

        // TODO: 過労働してでも先に終えたほうがよい？
        // work_cost: 降順によい労働
        let work_cost = |w_target, w_cur| {
            if w_target >= w_cur {
                w_cur as isize
            } else {
                // オーバーキルは無駄が出るけど...
                w_cur as isize - (w_cur - w_target) as isize * 2
            }
        };
        let mut work_cards = vec![];
        let mut cancel_one_cards = vec![];
        let mut cancel_all_cards = vec![];
        let mut boost_cards = vec![];
        for (i, (t, w)) in twn.iter().enumerate() {
            match *t {
                NORMAL_WORK => {
                    let cur = work_cost(projects[wi_do].h, *w);
                    work_cards.push((Reverse(cur), 1, i))
                },
                SUPER_WORK => {
                    let mut cur = 0;
                    for p in &projects {
                        cur += work_cost(p.h, *w);
                    }
                    work_cards.push((Reverse(cur), 0, i))
                },
                CANCEL_ONE => {
                    cancel_one_cards.push(i);
                }
                CANCEL_ALL => {
                    // /2: 勘
                    cancel_all_cards.push(i);
                }
                BOOST => {
                    boost_cards.push(i);
                }
                _ => {},
            }
        }
        work_cards.sort_unstable();

        let mut card_i_used = 0;
        let mut have_work_card = !work_cards.is_empty();
        let mut have_cancel_card = !cancel_one_cards.is_empty() || !cancel_all_cards.is_empty();
        if !boost_cards.is_empty() && boost_use_cnt < BOOST_USE_MAX {
            println!("# boost");
            card_i_used = boost_cards[0];
            boost_use_cnt += 1;
            println!("{card_i_used} 0");
        } else if !projects[wi_do].is_good(boost_use_cnt) && !cancel_all_cards.is_empty() {
            println!("# cancel all");
            card_i_used = cancel_all_cards[0];
            println!("{card_i_used} 0");
            if cancel_all_cards.len() == 1 && cancel_one_cards.is_empty() {
                have_cancel_card = false;
            }
        } else if !projects[wi_do].is_good(boost_use_cnt) && !cancel_one_cards.is_empty() {
            println!("# cancel one");
            card_i_used = cancel_one_cards[0];
            println!("{card_i_used} {wi_cancel}");
            if cancel_one_cards.len() == 1 && cancel_all_cards.is_empty() {
                have_cancel_card = false;
            }
        } else if work_cards.is_empty() {
            // 労働カードと増資カードが手元にないので適当に流す
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
            if work_cards.len() == 1 {
                have_work_card = false;
            }
        }
        stdout().flush().unwrap();

        // カード選択部

        input! {
            from &mut source,
            hvm_nxt: [(usize, usize); m],
            money: usize,
            twpk_nxt: [(usize, usize, usize); k],
        }
        // プロジェクトが全部非効率か？
        let mut prj_all_bad = true;
        for (h, v) in &hvm_nxt {
            if v > h {
                prj_all_bad = false;
                break;
            }
        }

        // いちいちインスタンス作り直すけれど, 乱択してないので時間には余裕がある
        let mut projects_nxt = Vec::with_capacity(m);
        for (h, v) in hvm_nxt {
            projects_nxt.push(Project {h, v});
        }
        projects = projects_nxt;
        vp = sort_prj_w_idx(&projects);

        // 取得方針はこの順
        // - 増資があれば取る
        // - 安いもの
        let mut boosts = vec![];
        let mut cancels = vec![];
        let mut works = vec![];
        for (i, (t, w, p)) in twpk_nxt.iter().enumerate() {
            match *t {
                NORMAL_WORK => {
                    if *p <= money && w >= p {
                        works.push((p, Reverse(w - p), 1, i));
                    }
                },
                SUPER_WORK => {
                    if *p <= money && w * n >= *p {
                        works.push((p, Reverse(w * n - p), 0, i));
                    }
                }
                CANCEL_ONE => {
                    if *p <= money {
                        cancels.push((*p, 1, i));
                    }
                }
                CANCEL_ALL => {
                    if *p <= money {
                        // /2: 勘
                        cancels.push((*p / 2, 0, i));
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

        let card_i_get = if ti > use_cost_turn_last {
            // 最終ターン付近にコストを払わない
            0
        } else if ti <= use_boost_turn_last
            && !boosts.is_empty()
            && boost_use_cnt < BOOST_USE_MAX
        {
            boosts[0].1
        } else if ti <= use_cancel_turn_last && !cancels.is_empty()
            && (prj_all_bad || (have_work_card && !have_cancel_card && cancels[0].0 < 3 * 2usize.pow(boost_use_cnt as u32)))
        {
            cancels[0].2
        } else {
            // "0" は労働力 1 という最弱手であり避けられるなら避けたい
            if works.len() > 1 {
                let wi = works[1].3;
                if (works[1].2 == 1 && twpk_nxt[wi].1 > twpk_nxt[wi].2)
                    || (works[1].2 == 0 && twpk_nxt[wi].1 * m > twpk_nxt[wi].2)
                {
                    works[1].3
                } else {
                    works[0].3
                }
            } else {
                // コスト 0 札
                works[0].3
            }
        };

        // println!("{:?}", twpk_nxt);
        // println!("{:?}", works);
        println!("{card_i_get}");
        twn[card_i_used] = (twpk_nxt[card_i_get].0, twpk_nxt[card_i_get].1);
    }
}
