use proconio::input;
use proconio::marker::Chars;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

// 盤面サイズ 50x50
const N: usize = 50;
// グループ数
const M: usize = 1000;

fn calculate_available_component_sizes(
    grass: &[Vec<bool>],
    occupied: &[Vec<bool>],
) -> (Vec<Vec<usize>>, Vec<(usize, usize)>) {
    let n = grass.len();
    let mut component_sizes = vec![vec![0; n]; n];
    let mut component_starts = Vec::new();

    for start_x in 0..n {
        for start_y in 0..n {
            if !grass[start_x][start_y]
                || occupied[start_x][start_y]
                || component_sizes[start_x][start_y] != 0
            {
                continue;
            }

            let mut cells = vec![(start_x, start_y)];
            let mut cursor = 0;
            component_sizes[start_x][start_y] = usize::MAX;
            component_starts.push((start_x, start_y));

            while cursor < cells.len() {
                let (x, y) = cells[cursor];
                cursor += 1;

                for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || nx >= n as i32 || ny < 0 || ny >= n as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if grass[nx][ny] && !occupied[nx][ny] && component_sizes[nx][ny] == 0 {
                        component_sizes[nx][ny] = usize::MAX;
                        cells.push((nx, ny));
                    }
                }
            }

            for (x, y) in cells {
                component_sizes[x][y] = cursor;
            }
        }
    }

    (component_sizes, component_starts)
}

fn find_region_from_start(
    grass: &[Vec<bool>],
    occupied: &[Vec<bool>],
    start: (usize, usize),
    required_size: usize,
    deadline: Option<Instant>,
) -> Option<Vec<(usize, usize)>> {
    let n = grass.len();
    let (x, y) = start;
    let mut visited = vec![vec![false; n]; n];
    // 探索始点からのチェビシェフ距離が小さい候補を優先する。
    // Reverse により、距離・座標の昇順で取り出す min-heap として使う。
    let mut queue = BinaryHeap::from([Reverse((0_usize, x, y))]);
    let mut cells = Vec::with_capacity(required_size);
    visited[x][y] = true;

    while let Some(Reverse((_, cx, cy))) = queue.pop() {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return None;
        }

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

    None
}

fn boundary_len(cells: &[(usize, usize)], n: usize) -> usize {
    let mut in_region = vec![vec![false; n]; n];
    for &(x, y) in cells {
        in_region[x][y] = true;
    }

    let mut boundary = 0;
    for &(x, y) in cells {
        for (dx, dy) in [(0_i32, -1_i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0
                || nx >= n as i32
                || ny < 0
                || ny >= n as i32
                || !in_region[nx as usize][ny as usize]
            {
                boundary += 1;
            }
        }
    }

    boundary
}

fn find_best_region(
    grass: &[Vec<bool>],
    component_sizes: &[Vec<usize>],
    component_starts: &[(usize, usize)],
    occupied: &[Vec<bool>],
    required_size: usize,
) -> Option<Vec<(usize, usize)>> {
    let mut starts = component_starts
        .iter()
        .copied()
        .filter(|&(x, y)| component_sizes[x][y] >= required_size);
    let first_start = starts.next()?;
    let mut best_cells = find_region_from_start(grass, occupied, first_start, required_size, None)?;
    let mut best_boundary = boundary_len(&best_cells, grass.len());

    let deadline = Instant::now() + Duration::from_micros(1_500);
    for start in starts {
        if Instant::now() >= deadline {
            break;
        }

        if let Some(cells) =
            find_region_from_start(grass, occupied, start, required_size, Some(deadline))
            && Instant::now() < deadline
        {
            let boundary = boundary_len(&cells, grass.len());
            if boundary < best_boundary {
                best_cells = cells;
                best_boundary = boundary;
            }
        }
    }

    Some(best_cells)
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
    let mut out = io::BufWriter::new(stdout.lock());

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

        let (component_sizes, component_starts) =
            calculate_available_component_sizes(&grass, &occupied);

        // 移動は行わない。
        writeln!(out, "0").unwrap();
        if let Some(cells) =
            find_best_region(&grass, &component_sizes, &component_starts, &occupied, p)
        {
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
