#include <bits/stdc++.h>
using namespace std;

// 高速な乱数生成
struct Xorshift
{
    uint32_t x = 123456789, y = 362436069, z = 521288629, w = 88675123;
    uint32_t next()
    {
        uint32_t t = x ^ (x << 11);
        x = y;
        y = z;
        z = w;
        return w = (w ^ (w >> 19)) ^ (t ^ (t >> 8));
    }
    uint32_t next(uint32_t limit) { return next() % limit; }
    double nextDouble() { return next() / 4294967296.0; }
} rng;

int N, M, K, T;
vector<vector<int>> adj;
vector<int> X, Y;
vector<vector<int>> dist;   // dist[i][j] = 頂点iから頂点jへの最短距離
vector<vector<int>> parent; // 最短経路復元用

// BFSで全頂点間最短距離を計算
void computeShortestPaths()
{
    dist.assign(N, vector<int>(N, INT_MAX));
    parent.assign(N, vector<int>(N, -1));

    for (int src = 0; src < N; src++)
    {
        queue<int> q;
        q.push(src);
        dist[src][src] = 0;

        while (!q.empty())
        {
            int u = q.front();
            q.pop();
            for (int v : adj[u])
            {
                if (dist[src][v] == INT_MAX)
                {
                    dist[src][v] = dist[src][u] + 1;
                    parent[src][v] = u;
                    q.push(v);
                }
            }
        }
    }
}

// srcからdstへの最短経路を取得（srcは含まない、dstは含む）
vector<int> getPath(int src, int dst)
{
    if (src == dst)
        return {};
    vector<int> path;
    for (int v = dst; v != src; v = parent[src][v])
    {
        path.push_back(v);
    }
    reverse(path.begin(), path.end());
    return path;
}

// 状態: 各木の色 (0=W, 1=R) と行動列
struct State
{
    vector<int> treeColor; // treeColor[i] for vertex K+i
    vector<int> actions;   // 行動列: >=0 なら移動先頂点, -1 なら行動2
};

// シミュレーション結果
struct SimResult
{
    int score;
    bool valid;
};

// 状態をシミュレートしてスコアを計算
SimResult simulate(const State &state)
{
    vector<unordered_set<string>> shopInventory(K);
    vector<int> treeColor(N - K, 0); // 0=W, 1=R

    int pos = 0;
    int prevPos = -1;
    string cone;
    cone.reserve(100);

    for (int action : state.actions)
    {
        if (action == -1)
        {
            // 行動2: 現在位置がW木ならRに変更
            if (pos < K)
                return {0, false}; // ショップでは行動2不可
            int treeIdx = pos - K;
            if (treeColor[treeIdx] == 1)
                return {0, false}; // 既にR
            treeColor[treeIdx] = 1;
        }
        else
        {
            // 行動1: 移動
            int nextPos = action;

            // 隣接チェック
            bool found = false;
            for (int neighbor : adj[pos])
            {
                if (neighbor == nextPos)
                {
                    found = true;
                    break;
                }
            }
            if (!found)
                return {0, false};

            // 前回の移動元には戻れない
            if (nextPos == prevPos)
                return {0, false};

            prevPos = pos;
            pos = nextPos;

            if (pos >= K)
            {
                // アイスクリームの木: 収穫
                int treeIdx = pos - K;
                cone += (treeColor[treeIdx] == 0) ? 'W' : 'R';
            }
            else
            {
                // アイスクリームショップ: 納品
                shopInventory[pos].insert(cone);
                cone.clear();
            }
        }
    }

    int score = 0;
    for (int i = 0; i < K; i++)
    {
        score += shopInventory[i].size();
    }
    return {score, true};
}

// ランダムな初期解を生成（異なる長さのコーンを作る戦略）
State generateInitialState()
{
    State state;
    state.treeColor.assign(N - K, 0); // すべてW

    int pos = 0;
    int prevPos = -1;
    int stepCount = 0;
    int coneLength = 0;

    // 目標のコーン長さ（1〜5の間でランダムに変化）
    int targetConeLength = rng.next(5) + 1;

    while (stepCount < T)
    {
        // コーンが目標長さに達したらショップを目指す
        if (coneLength >= targetConeLength && pos >= K)
        {
            // 最寄りのショップを探す
            int nearestShop = -1;
            int minDist = INT_MAX;
            for (int shop = 0; shop < K; shop++)
            {
                if (dist[pos][shop] < minDist)
                {
                    minDist = dist[pos][shop];
                    nearestShop = shop;
                }
            }

            vector<int> path = getPath(pos, nearestShop);
            bool moved = false;
            for (int nextPos : path)
            {
                if (stepCount >= T)
                    break;
                if (nextPos == prevPos)
                    break;

                state.actions.push_back(nextPos);
                prevPos = pos;
                pos = nextPos;
                stepCount++;
                moved = true;

                if (pos >= K)
                    coneLength++;
                else
                {
                    coneLength = 0;
                    targetConeLength = rng.next(5) + 1;
                }
            }

            // パスを進めなかった場合はランダム移動にフォールバック
            if (!moved)
            {
                vector<int> candidates;
                for (int neighbor : adj[pos])
                {
                    if (neighbor != prevPos)
                        candidates.push_back(neighbor);
                }
                if (candidates.empty())
                    break;
                int nextPos = candidates[rng.next(candidates.size())];
                state.actions.push_back(nextPos);
                prevPos = pos;
                pos = nextPos;
                stepCount++;
                if (pos >= K)
                    coneLength++;
                else
                {
                    coneLength = 0;
                    targetConeLength = rng.next(5) + 1;
                }
            }
        }
        else
        {
            // ランダムに移動（木を優先）
            vector<int> treeCandidates, shopCandidates;
            for (int neighbor : adj[pos])
            {
                if (neighbor != prevPos)
                {
                    if (neighbor >= K)
                        treeCandidates.push_back(neighbor);
                    else
                        shopCandidates.push_back(neighbor);
                }
            }

            int nextPos;
            if (!treeCandidates.empty() && (shopCandidates.empty() || rng.next(100) < 70))
            {
                nextPos = treeCandidates[rng.next(treeCandidates.size())];
            }
            else if (!shopCandidates.empty())
            {
                nextPos = shopCandidates[rng.next(shopCandidates.size())];
            }
            else
            {
                break;
            }

            state.actions.push_back(nextPos);
            prevPos = pos;
            pos = nextPos;
            stepCount++;

            if (pos >= K)
                coneLength++;
            else
            {
                coneLength = 0;
                targetConeLength = rng.next(5) + 1;
            }
        }
    }

    return state;
}

// 近傍解を生成（操作後は必ず repair で修正される前提）
State generateNeighbor(const State &current)
{
    State neighbor = current;

    int opType = rng.next(100);

    if (opType < 25 && !neighbor.actions.empty())
    {
        // 操作1: ランダムな位置から先を再生成（小規模）
        int idx = rng.next(neighbor.actions.size());

        // idx番目の行動時点での位置と前回位置を計算
        int pos = 0, prevPos = -1;
        for (int i = 0; i < idx; i++)
        {
            if (neighbor.actions[i] >= 0)
            {
                prevPos = pos;
                pos = neighbor.actions[i];
            }
        }

        // idx以降を削除して再生成
        neighbor.actions.resize(idx);

        int addCount = rng.next(30) + 1; // 小規模な再生成
        for (int i = 0; i < addCount && neighbor.actions.size() < (size_t)T; i++)
        {
            vector<int> candidates;
            for (int v : adj[pos])
            {
                if (v != prevPos)
                    candidates.push_back(v);
            }
            if (candidates.empty())
                break;
            int nextPos = candidates[rng.next(candidates.size())];
            neighbor.actions.push_back(nextPos);
            prevPos = pos;
            pos = nextPos;
        }
    }
    else if (opType < 40 && neighbor.actions.size() > 10)
    {
        // 操作2: ランダムな位置からショップへ向かう経路を挿入して再生成
        int idx = rng.next(neighbor.actions.size() - 5);

        // idx位置での状態を計算
        int pos = 0, prevPos = -1;
        for (int i = 0; i < idx; i++)
        {
            if (neighbor.actions[i] >= 0)
            {
                prevPos = pos;
                pos = neighbor.actions[i];
            }
        }

        // idx以降を削除
        neighbor.actions.resize(idx);

        // 最寄りのショップへ向かう
        int targetShop = -1;
        int minDist = INT_MAX;
        for (int shop = 0; shop < K; shop++)
        {
            if (dist[pos][shop] < minDist)
            {
                minDist = dist[pos][shop];
                targetShop = shop;
            }
        }
        vector<int> path = getPath(pos, targetShop);

        for (int nextPos : path)
        {
            if (neighbor.actions.size() >= (size_t)T)
                break;
            if (nextPos == prevPos)
                break;
            neighbor.actions.push_back(nextPos);
            prevPos = pos;
            pos = nextPos;
        }

        // その後ランダムに継続
        int insertCount = rng.next(50) + 1;
        for (int i = 0; i < insertCount && neighbor.actions.size() < (size_t)T; i++)
        {
            vector<int> candidates;
            for (int v : adj[pos])
            {
                if (v != prevPos)
                    candidates.push_back(v);
            }
            if (candidates.empty())
                break;
            int nextPos = candidates[rng.next(candidates.size())];
            neighbor.actions.push_back(nextPos);
            prevPos = pos;
            pos = nextPos;
        }
    }
    else if (opType < 55)
    {
        // 操作3: 末尾に行動を追加（最寄りショップへ）
        if (neighbor.actions.size() < (size_t)T)
        {
            int pos = 0, prevPos = -1;
            for (int action : neighbor.actions)
            {
                if (action >= 0)
                {
                    prevPos = pos;
                    pos = action;
                }
            }

            // 木にいる場合は最寄りのショップへ向かう
            if (pos >= K)
            {
                int targetShop = -1;
                int minDist = INT_MAX;
                for (int shop = 0; shop < K; shop++)
                {
                    if (dist[pos][shop] < minDist)
                    {
                        minDist = dist[pos][shop];
                        targetShop = shop;
                    }
                }
                vector<int> path = getPath(pos, targetShop);
                for (int nextPos : path)
                {
                    if (neighbor.actions.size() >= (size_t)T)
                        break;
                    if (nextPos == prevPos)
                        break;
                    neighbor.actions.push_back(nextPos);
                    prevPos = pos;
                    pos = nextPos;
                }
            }

            // ランダムに継続
            vector<int> candidates;
            for (int v : adj[pos])
            {
                if (v != prevPos)
                    candidates.push_back(v);
            }
            if (!candidates.empty())
            {
                int addCount = rng.next(50) + 1;
                for (int i = 0; i < addCount && neighbor.actions.size() < (size_t)T; i++)
                {
                    if (candidates.empty())
                        break;
                    int nextPos = candidates[rng.next(candidates.size())];
                    neighbor.actions.push_back(nextPos);
                    prevPos = pos;
                    pos = nextPos;
                    candidates.clear();
                    for (int v : adj[pos])
                    {
                        if (v != prevPos)
                            candidates.push_back(v);
                    }
                }
            }
        }
    }
    else if (opType < 65)
    {
        // 操作4: 行動2（色変更）を挿入（まだWの木のみ）
        if (neighbor.actions.size() < (size_t)T)
        {
            vector<int> treePositions;
            vector<int> treeColor(N - K, 0);
            int pos = 0;
            for (size_t i = 0; i < neighbor.actions.size(); i++)
            {
                if (neighbor.actions[i] == -1)
                {
                    if (pos >= K)
                        treeColor[pos - K] = 1;
                }
                else if (neighbor.actions[i] >= 0)
                {
                    pos = neighbor.actions[i];
                    if (pos >= K && treeColor[pos - K] == 0)
                    {
                        treePositions.push_back(i);
                    }
                }
            }
            if (!treePositions.empty())
            {
                int idx = treePositions[rng.next(treePositions.size())];
                neighbor.actions.insert(neighbor.actions.begin() + idx + 1, -1);
            }
        }
    }
    else if (opType < 75)
    {
        // 操作5: 行動2（色変更）を削除
        vector<size_t> action2Positions;
        for (size_t i = 0; i < neighbor.actions.size(); i++)
        {
            if (neighbor.actions[i] == -1)
            {
                action2Positions.push_back(i);
            }
        }
        if (!action2Positions.empty())
        {
            size_t idx = action2Positions[rng.next(action2Positions.size())];
            neighbor.actions.erase(neighbor.actions.begin() + idx);
        }
    }
    else if (opType < 90)
    {
        // 操作6: 2点間を最短経路で繋ぎ直す
        if (neighbor.actions.size() > 20)
        {
            int idx1 = rng.next(neighbor.actions.size() - 10);
            int idx2 = idx1 + rng.next(min(30, (int)neighbor.actions.size() - idx1 - 5)) + 5;

            // idx1時点での位置を計算
            int pos1 = 0, prevPos1 = -1;
            for (int i = 0; i <= idx1; i++)
            {
                if (neighbor.actions[i] >= 0)
                {
                    prevPos1 = pos1;
                    pos1 = neighbor.actions[i];
                }
            }

            // idx2時点での位置を計算
            int pos2 = 0;
            for (int i = 0; i <= idx2; i++)
            {
                if (neighbor.actions[i] >= 0)
                {
                    pos2 = neighbor.actions[i];
                }
            }

            // pos1からpos2への最短経路で置き換え
            vector<int> path = getPath(pos1, pos2);
            if (!path.empty() && path[0] != prevPos1)
            {
                vector<int> newActions;
                newActions.reserve(neighbor.actions.size());
                for (int i = 0; i <= idx1; i++)
                {
                    newActions.push_back(neighbor.actions[i]);
                }
                int prevPos = prevPos1;
                int pos = pos1;
                for (int nextPos : path)
                {
                    if (nextPos == prevPos)
                        break;
                    newActions.push_back(nextPos);
                    prevPos = pos;
                    pos = nextPos;
                }
                for (size_t i = idx2 + 1; i < neighbor.actions.size(); i++)
                {
                    newActions.push_back(neighbor.actions[i]);
                }
                neighbor.actions = move(newActions);
            }
        }
    }
    else
    {
        // 操作7: 末尾をランダムに再生成（大規模）
        if (!neighbor.actions.empty())
        {
            int idx = rng.next(neighbor.actions.size());
            int pos = 0, prevPos = -1;
            for (int i = 0; i < idx; i++)
            {
                if (neighbor.actions[i] >= 0)
                {
                    prevPos = pos;
                    pos = neighbor.actions[i];
                }
            }
            neighbor.actions.resize(idx);

            int addCount = rng.next(200) + 50;
            for (int i = 0; i < addCount && neighbor.actions.size() < (size_t)T; i++)
            {
                vector<int> candidates;
                for (int v : adj[pos])
                {
                    if (v != prevPos)
                        candidates.push_back(v);
                }
                if (candidates.empty())
                    break;
                int nextPos = candidates[rng.next(candidates.size())];
                neighbor.actions.push_back(nextPos);
                prevPos = pos;
                pos = nextPos;
            }
        }
    }

    return neighbor;
}

// 有効な解に修正
void repair(State &state)
{
    vector<int> newActions;
    vector<int> treeColor(N - K, 0); // 0=W, 1=R

    int pos = 0, prevPos = -1;

    for (int action : state.actions)
    {
        if (newActions.size() >= (size_t)T)
            break;

        if (action == -1)
        {
            // 行動2: 現在位置がW木の場合のみ有効
            if (pos >= K)
            {
                int treeIdx = pos - K;
                if (treeColor[treeIdx] == 0)
                {
                    treeColor[treeIdx] = 1;
                    newActions.push_back(-1);
                }
                // 既にRの場合はスキップ（エラーではない）
            }
            // ショップにいる場合はスキップ
        }
        else
        {
            // 行動1: 隣接かつ前回位置でないかチェック
            bool valid = false;
            for (int v : adj[pos])
            {
                if (v == action && v != prevPos)
                {
                    valid = true;
                    break;
                }
            }
            if (valid)
            {
                newActions.push_back(action);
                prevPos = pos;
                pos = action;
            }
            else
            {
                // 不正な移動が見つかったら、ここで打ち切り
                break;
            }
        }
    }

    state.actions = newActions;
}

int main()
{
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    cin >> N >> M >> K >> T;

    adj.resize(N);
    for (int i = 0; i < M; i++)
    {
        int a, b;
        cin >> a >> b;
        adj[a].push_back(b);
        adj[b].push_back(a);
    }

    X.resize(N);
    Y.resize(N);
    for (int i = 0; i < N; i++)
    {
        cin >> X[i] >> Y[i];
    }

    // 最短経路を計算
    computeShortestPaths();

    // 時間計測
    auto startTime = chrono::high_resolution_clock::now();
    auto getElapsed = [&]()
    {
        auto now = chrono::high_resolution_clock::now();
        return chrono::duration<double>(now - startTime).count();
    };

    const double TIME_LIMIT = 1.9;

    // 複数の初期解を試して最良のものを選ぶ
    State bestState;
    int bestScore = -1;

    for (int trial = 0; trial < 5; trial++)
    {
        State state = generateInitialState();
        repair(state);
        SimResult result = simulate(state);
        if (result.valid && result.score > bestScore)
        {
            bestScore = result.score;
            bestState = state;
        }
    }

    State currentState = bestState;
    int currentScore = bestScore;

    // 焼きなまし法
    double startTemp = 30.0;
    double endTemp = 0.001;
    int iteration = 0;
    int noImproveCount = 0;
    const int RESTART_THRESHOLD = 20000;

    while (true)
    {
        double elapsed = getElapsed();
        if (elapsed > TIME_LIMIT)
            break;

        double progress = elapsed / TIME_LIMIT;
        double temp = startTemp * pow(endTemp / startTemp, progress);

        // 停滞時はrestart
        if (noImproveCount > RESTART_THRESHOLD)
        {
            State newState = generateInitialState();
            repair(newState);
            SimResult newResult = simulate(newState);
            if (newResult.valid)
            {
                currentState = newState;
                currentScore = newResult.score;
                if (currentScore > bestScore)
                {
                    bestScore = currentScore;
                    bestState = currentState;
                }
            }
            noImproveCount = 0;
        }

        // 近傍解を生成
        State neighborState = generateNeighbor(currentState);
        repair(neighborState);

        SimResult neighborResult = simulate(neighborState);

        if (neighborResult.valid)
        {
            int neighborScore = neighborResult.score;
            int delta = neighborScore - currentScore;

            // 受理判定
            if (delta > 0 || rng.nextDouble() < exp(delta / temp))
            {
                currentState = neighborState;
                currentScore = neighborScore;

                if (currentScore > bestScore)
                {
                    bestScore = currentScore;
                    bestState = currentState;
                    noImproveCount = 0;
                }
                else
                {
                    noImproveCount++;
                }
            }
            else
            {
                noImproveCount++;
            }
        }
        else
        {
            noImproveCount++;
        }

        iteration++;
    }

    // 出力前に最終検証（不正な行動が見つかったら打ち切り）
    {
        int pos = 0, prevPos = -1;
        vector<int> treeColor(N - K, 0);
        vector<int> validActions;
        for (int action : bestState.actions)
        {
            if (action == -1)
            {
                // 行動2: 現在位置がW木の場合のみ有効
                if (pos >= K)
                {
                    int treeIdx = pos - K;
                    if (treeColor[treeIdx] == 0)
                    {
                        treeColor[treeIdx] = 1;
                        validActions.push_back(-1);
                    }
                }
                continue;
            }

            // 隣接チェック
            bool isAdj = false;
            for (int v : adj[pos])
            {
                if (v == action)
                {
                    isAdj = true;
                    break;
                }
            }
            if (!isAdj)
                break; // 不正な移動が見つかったら打ち切り
            if (action == prevPos)
                break; // 前回位置への移動も打ち切り

            validActions.push_back(action);
            prevPos = pos;
            pos = action;
        }
        bestState.actions = validActions;
    }

    // 出力
    for (int action : bestState.actions)
    {
        cout << action << "\n";
    }

    return 0;
}
