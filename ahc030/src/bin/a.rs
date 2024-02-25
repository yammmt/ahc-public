use proconio::{input, source::line::LineSource};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::io::{stdout, Write};

fn could_enumerate(n: usize, p: &Vec<Vec<(usize, usize)>>) -> bool {
    let nn = n * n;
    let mut ptrn = 1;

    // TODO: ポリオミノのサイズも考慮すべきに聞こえる, が総和 n*n 以下だし...
    for _pp in p {
        ptrn *= nn;
        // マス全通りに始点判定を入れるので, 1 ループあたり (n*n)^m だけかかる
        // ループを最大 n*n 回問うとして 10^9 超えないように, そんなにループ回らないとは思うが
        // でもジャッジ投げると TLE あるので計算間違ってるみたい
        if ptrn * nn > 200_000_000 {
            return false;
        }
    }

    true
}

// 取りうる状態の総数とそこに置かれる場合の数
fn ptrn_map(
    reserves_map: &Vec<Vec<Option<usize>>>,
    polys: &Vec<Vec<(usize, usize)>>,
) -> (usize, Vec<Vec<usize>>) {
    let n = reserves_map.len();

    let mut candidates = vec![];
    candidates.push(vec![vec![0; n]; n]);
    for p in polys {
        let mut candidates_nxt = vec![];
        for c in &candidates {
            for i in 0..n {
                for j in 0..n {
                    let mut candidate_cur = c.clone();
                    let mut cur_pass = true;
                    for pp in p {
                        let i_cur = i + pp.0;
                        let j_cur = j + pp.1;
                        if i_cur >= n || j_cur >= n {
                            cur_pass = false;
                            break;
                        }

                        if let Some(r) = reserves_map[i_cur][j_cur] {
                            if candidate_cur[i_cur][j_cur] + 1 > r {
                                cur_pass = false;
                                break;
                            }

                            candidate_cur[i_cur][j_cur] = 0;
                        } else {
                            candidate_cur[i_cur][j_cur] += 1;
                        }
                    }
                    if cur_pass {
                        candidates_nxt.push(candidate_cur);
                    }
                }
            }
        }
        candidates = candidates_nxt;
    }

    let mut ret = vec![vec![0; n]; n];
    for c in &candidates {
        for i in 0..n {
            for j in 0..n {
                if c[i][j] > 0 {
                    ret[i][j] += 1;
                }
            }
        }
    }

    (candidates.len(), ret)
}

fn could_answer_w_possible_map(candidates_num: usize, ptrn_map: &Vec<Vec<usize>>) -> bool {
    let n = ptrn_map.len();
    for i in 0..n {
        for j in 0..n {
            if ptrn_map[i][j] != 0 && ptrn_map[i][j] != candidates_num {
                return false;
            }
        }
    }
    true
}

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        m: usize,
        _eps: f64,
    }
    let mut reserves_sum = 0;
    let mut polys = Vec::with_capacity(m);
    // 向きがわかっているので回転は不要
    for _ in 0..m {
        input! {
            from &mut source,
            d: usize,
        }
        reserves_sum += d;
        let mut v = Vec::with_capacity(d);
        for _ in 0..d {
            input! {
                from &mut source,
                i: usize,
                j: usize,
            }
            v.push((i, j));
        }
        polys.push(v);
    }

    let dir = [(-1, 0), (0, -1), (1, 0), (0, 1)];
    let turn_max = 2 * n * n;

    // マス数が n*n で最大操作回数が 2*n*n だから全マスなめると終わり
    // 正の貯蔵量を有すマスを特定すればよく, 厳密な配置は不要
    // ポリオミノの面積は入力時点でわかるが

    // 島の大きさ n: 10 <= n <= 20
    // マスの数は最大で 400

    // 油田の個数 m: 2 <= m <= 20
    // 油田のあるマス数の最小値は, 雑には最大の油田数
    // もう少し真面目に見ると各マスをよい感じに重なるように仕向けるとよいが
    // キレイなアルゴリズムが浮かばない
    // 単一の油田の面積最大は n*n/m, つまり最大面積の油田を m 個乗せると
    // マス数とポリオミノの面積数が一致する

    // 最大の島に対し油田最小最大を考えると
    // 最小: 4x2=8 マスしか埋まらない, 正マスをひく確率は最大で 8/400
    // 最大: 400/20*20 = 400 マス埋まる, おおよそ全部正マス

    // q1: 1 マス選ぶ
    // 全マス見て答えると制限回数の半分の施行で必ず正答になる
    // 飛ばして見ていって得する場面がある？ポリオミノは連結だからなさそう

    // q2: 任意マス選ぶ
    // 簡単のため, エラーパラメータを考えず常に正解が返ってくるとして考えると
    // 正方形囲んで中いくらか開けて...で
    // 一定サイズの正方形で走査していき, スコアの大きいところから中心的に掘っていくとか？
    // 2x2 領域で 3 マス既知の場合に q1 飛ばすのと q2 飛ばすのはどちらがよい？
    // 後者はコスト 0.5 になる, が, 聞いて答え問うなら単発であるなら答え投げたほうがよいのでは

    // 解出力
    // 失敗時にメリットがない, 直感的には五割で正答できるなら聞いて良い気がする
    // q1 連打で最後 1 マスの値聞くくらいなら解答してよい, ノーリスクになる
    // が, 不明マス数が多くなると, 正答引くために 2 べき乗ガチャを回すことになる

    // 方針: DFS で連結成分を抜き出す
    // ポリオミノは連結であるので, 一マス暴ければ上下左右を繋げていくと連鎖が取れそう
    // すべてのポリオミノを検出した可能性があるならば回答を試みる
    // すべてが検出できていない場合は回答しない, 2 べきガチャとなり不利
    // q2 がうまく使えるとよいのだろうが未だ名案は思い浮かばない
    // 最大計算量は 400 個のマス一つ一つを掘りつつ合計 400 マスをもつポリオミノと比較した場合
    // ランダムケースならなんか間に合いそうだしええやろの精神で取り敢えず出したろ
    // HashSet 管理だと定数倍が重いが TLE したら考える

    let mut rng = SmallRng::from_entropy();

    let use_short_method = could_enumerate(n, &polys);
    let mut reserves = vec![vec![None; n]; n];
    let mut reserves_found_sum = 0;
    let mut stack = VecDeque::new();
    stack.push_back((n / 2, n / 2));
    let mut could_answer = false;

    let mut insert_beginning_point =
        |q: &mut VecDeque<(usize, usize)>, map: &Vec<Vec<Option<usize>>>| {
            // とりあえず乱択するが, 見なくても 0 とわかる点がありそう
            loop {
                let i = rng.gen_range(0..n);
                let j = rng.gen_range(0..n);
                if map[i][j].is_none() {
                    q.push_back((i, j));
                    return;
                }
            }
        };

    println!("#c use_short_method: {use_short_method}");
    for turn_cur in 0..turn_max {
        println!("#c turn: {turn_cur}, found: {reserves_found_sum}/{reserves_sum}");
        if use_short_method {
            let ptrn = ptrn_map(&reserves, &polys);
            println!("#c ptrn.0: {:?}", ptrn.0);
            // if turn_cur > 20 {
            //     println!("#c ptrn.1: {:?}", ptrn.1);
            // }

            // 置き方が一意に絞れずとも埋まっているか否かに限ると絞れる場合がある (seed10)
            if could_answer_w_possible_map(ptrn.0, &ptrn.1) {
                // 答えが唯一
                let mut ans = vec![];
                for i in 0..n {
                    for j in 0..n {
                        if let Some(c) = reserves[i][j] {
                            if c > 0 {
                                ans.push((i, j));
                            }
                        } else if ptrn.1[i][j] > 0 {
                            ans.push((i, j));
                        }
                    }
                }

                print!("a {} ", ans.len());
                for (i, a) in ans.iter().enumerate() {
                    print!("{} {}", a.0, a.1);
                    if i == ans.len() - 1 {
                        println!();
                    } else {
                        print!(" ");
                    }
                }
                stdout().flush().unwrap();

                input! {
                    from &mut source,
                    is_true: usize,
                }
                assert_eq!(is_true, 1);
                return;
            }

            let mut p_x = 0;
            let mut p_y = 0;
            let mut p_prob = 10.0;
            for i in 0..n {
                for j in 0..n {
                    if reserves[i][j].is_some() || ptrn.1[i][j] == 0 || ptrn.1[i][j] == ptrn.0 {
                        continue;
                    }

                    let prob_cur = (ptrn.1[i][j] as f64 / ptrn.0 as f64 - 0.5).abs();
                    if prob_cur < p_prob {
                        p_x = i;
                        p_y = j;
                        p_prob = prob_cur;
                    }
                }
            }
            println!("#c  {} / {}", ptrn.1[p_x][p_y], ptrn.0);

            println!("q 1 {p_x} {p_y}");
            stdout().flush().unwrap();

            // 聞く
            input! {
                from &mut source,
                v: usize,
            }
            reserves[p_x][p_y] = Some(v);
            reserves_found_sum += v;
            continue;
        }

        if could_answer {
            let mut ans = vec![];
            for i in 0..n {
                for j in 0..n {
                    if let Some(v) = reserves[i][j] {
                        if v > 0 {
                            ans.push((i, j));
                        }
                    }
                }
            }
            print!("a {} ", ans.len());
            for (i, a) in ans.iter().enumerate() {
                print!("{} {}", a.0, a.1);
                if i == ans.len() - 1 {
                    println!();
                } else {
                    print!(" ");
                }
            }
            stdout().flush().unwrap();

            input! {
                from &mut source,
                is_true: usize,
            }
            if is_true == 1 {
                return;
            } else {
                assert!(false);
                insert_beginning_point(&mut stack, &reserves);
            }
            could_answer = false;
        } else {
            assert!(!stack.is_empty());
            // 問う
            let point_dig = stack.pop_back().unwrap();
            let p_x = point_dig.0;
            let p_y = point_dig.1;
            println!("q 1 {p_x} {p_y}");
            stdout().flush().unwrap();

            // 聞く
            input! {
                from &mut source,
                v: usize,
            }
            reserves[p_x][p_y] = Some(v);
            reserves_found_sum += v;
            if reserves_found_sum == reserves_sum {
                // この手法では全点見つければ確実に正答になる
                // ここから下の処理は stack 処理部以外はいらない
                could_answer = true;
                continue;
            }

            // 探索候補を足す
            if v > 0 {
                for d in &dir {
                    let x_nxt = p_x.wrapping_add_signed(d.0);
                    let y_nxt = p_y.wrapping_add_signed(d.1);
                    if x_nxt < n && y_nxt < n && reserves[x_nxt][y_nxt].is_none() {
                        stack.push_back((x_nxt, y_nxt));
                    }
                }
            }

            // 探索続行判定
            // 既に探索手詰まりになっている可能性がある, ここで探索可能点が出るまで更新する
            // TODO: ポリオミノの向きがわかっているので, 埋まっている可能性がある探索の方向が絞れる
            // TODO: 埋まっている率が高い点がわかると 0-1BFS で多少の高速化ができる
            if stack.is_empty() {
                insert_beginning_point(&mut stack, &reserves);
            } else {
                let mut point_dig = stack.pop_back().unwrap();
                while reserves[point_dig.0][point_dig.1].is_some() {
                    if stack.is_empty() {
                        insert_beginning_point(&mut stack, &reserves);
                    }

                    point_dig = stack.pop_back().unwrap();
                }
                stack.push_back(point_dig);
            }
        }
    }
}
