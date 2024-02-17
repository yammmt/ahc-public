use proconio::{input, source::line::LineSource};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashSet, VecDeque};
use std::io::{stdout, Write};

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
    let mut unfound_polys = HashSet::new();
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
        unfound_polys.insert(v);
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

    let mut reserves = vec![vec![None; n]; n];
    let mut reserves_found_sum = 0;
    let mut stack = VecDeque::new();
    stack.push_back((n / 2, n / 2));
    let mut poly_cur = vec![];
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

    for turn_cur in 0..turn_max {
        println!("#c turn: {turn_cur}");

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
                poly_cur.push((p_x, p_y));
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
            let mut poly_search_completed = false;
            if stack.is_empty() {
                poly_search_completed = true;
                insert_beginning_point(&mut stack, &reserves);
            } else {
                let mut point_dig = stack.pop_back().unwrap();
                while reserves[point_dig.0][point_dig.1].is_some() {
                    if stack.is_empty() {
                        poly_search_completed = true;
                        insert_beginning_point(&mut stack, &reserves);
                    }

                    point_dig = stack.pop_back().unwrap();
                }
                stack.push_back(point_dig);
            }

            if poly_search_completed {
                println!("#c   poly_search_completed");
                // HACK: 一々見つかったポリオミノから全体図を作るのでは判定遅い
                //       が, 実行時間にかなり余裕があるのでほっとく
                let mut poly_cur_map = vec![vec![false; n]; n];
                for &p in &poly_cur {
                    poly_cur_map[p.0][p.1] = true;
                }
                if !poly_cur.is_empty() {
                    println!("#c   poly_cur_map:");
                    for pm in &poly_cur_map {
                        println!("#c    {:?}", pm);
                    }
                }

                let mut found_polys = vec![];
                for vpoly in &unfound_polys {
                    'i_loop: for i in 0..n {
                        for j in 0..n {
                            // 左上 (i, j)
                            let mut contained = true;
                            for p in vpoly {
                                let i_cur = p.0 + i;
                                let j_cur = p.1 + j;
                                if i_cur >= n || j_cur >= n || !poly_cur_map[i_cur][j_cur] {
                                    contained = false;
                                    break;
                                }
                            }

                            if contained {
                                found_polys.push(vpoly.clone());
                                break 'i_loop;
                            }
                        }
                    }
                }

                println!("#c   found_polys.len(): {}", found_polys.len());
                for fp in found_polys {
                    unfound_polys.remove(&fp);
                }

                poly_cur.clear();
            }
        }
    }
}
