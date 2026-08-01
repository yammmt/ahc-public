use proconio::input;
use proconio::marker::Chars;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{self, Write};

// 盤面サイズ 50x50
const N: usize = 50;
// グループ数
const M: usize = 1000;

fn find_region(
    grass: &[Vec<bool>],
    occupied: &[Vec<bool>],
    required_size: usize,
) -> Option<Vec<(usize, usize)>> {
    let n = grass.len();

    // 始点は左上から行優先で選ぶ。
    for x in 0..n {
        for y in 0..n {
            if !grass[x][y] || occupied[x][y] {
                continue;
            }

            let mut visited = vec![vec![false; n]; n];
            // 探索始点からのチェビシェフ距離が小さい候補を優先する。
            // Reverse により、距離・座標の昇順で取り出す min-heap として使う。
            let mut queue = BinaryHeap::from([Reverse((0_usize, x, y))]);
            let mut cells = Vec::with_capacity(required_size);
            visited[x][y] = true;

            while let Some(Reverse((_, cx, cy))) = queue.pop() {
                cells.push((cx, cy));
                if cells.len() == required_size {
                    return Some(cells);
                }

                // 追加済みマスに隣接する空きマスを追加候補とする。
                for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if grass[nx][ny] && !occupied[nx][ny] && !visited[nx][ny] {
                        visited[nx][ny] = true;
                        let distance = nx.abs_diff(x).max(ny.abs_diff(y));
                        queue.push(Reverse((distance, nx, ny)));
                    }
                }
            }
        }
    }
    None
}

fn main() {
    input! {
        _n: usize,
        _m: usize,
        _r: f64,
        rows: [Chars; N],
    }
    let grass: Vec<Vec<bool>> = rows
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell == '.').collect())
        .collect();
    let mut occupied = vec![vec![false; N]; N];
    let mut regions: Vec<Vec<(usize, usize)>> = vec![Vec::new(); M];
    let mut departure_times = vec![0_i64; M];
    let mut active = vec![false; M];
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for i in 0..M {
        input! {
            _group_id: usize,
            s: i64,
            t: i64,
            p: usize,
            _v: i64,
        }

        for j in 0..i {
            if active[j] && departure_times[j] < s {
                for &(x, y) in &regions[j] {
                    occupied[x][y] = false;
                }
                active[j] = false;
            }
        }

        // 移動は行わない。
        writeln!(out, "0").unwrap();
        if let Some(cells) = find_region(&grass, &occupied, p) {
            writeln!(out, "Yes").unwrap();
            for &(x, y) in &cells {
                writeln!(out, "{x} {y}").unwrap();
                occupied[x][y] = true;
            }
            regions[i] = cells;
            departure_times[i] = t;
            active[i] = true;
        } else {
            writeln!(out, "No").unwrap();
        }
        out.flush().unwrap();
    }
}
