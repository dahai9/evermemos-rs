# 45fe47b — feat(evermemos-rs): auto-start server in test scripts

## 背景

每次运行 `test_completeness.py` 或 `parity_test.py` 都需要先手动判断服务是否在运行，在另一个终端启动，
等待就绪，再切回去跑测试。这是纯粹的重复机械劳动，没有任何信息量。

## 解法

新增 `demo/server_utils.py`，提供 `ensure_server(url)` 函数：

1. 先做 HTTP health check，如果服务已在运行 → 直接 return None（不重复启动）
2. 如果不在运行 → 自动定位二进制（按优先级查找：`EVERMEMOS_BIN` env → `~/.cargo/target/debug/evermemos` → 本地 target → PATH）
3. 用 `subprocess.Popen` 从 `evermemos-rs/` 目录启动（保证 `rocksdb://./data/surreal` 路径正确）
4. 最长等待 20s 轮询 health，超时报错
5. 通过 `atexit` 注册在脚本退出时自动 terminate（除非 `KEEP_SERVER=1`）

返回 `Popen | None`，调用者知道自己是否启动了服务。

### 关键设计决策

- **为什么用 atexit 而不是 try/finally** — 不需要重构 `async with` 的缩进，改动最小；`sys.exit()` 也会触发 atexit。
- **为什么不自动 build** — build 需要 nix 环境且耗时，用户应该先 build 好。找不到 binary 时告知用法并 exit。
- **`KEEP_SERVER=1` env var** — 开发者调试时不想每次等服务重启，可以保留已启动的服务。

### before / after

```bash
# Before（每次需要手动两步）
cd evermemos-rs && cargo run --bin evermemos &   # Terminal A
sleep 8 && curl http://localhost:8080/health      # Terminal A（等待）
python demo/test_completeness.py                  # Terminal B

# After（一条命令）
python evermemos-rs/demo/test_completeness.py
```

输出示例（无服务器状态下启动）：
```
▶ Step 0 — Health check
  ⚡ Starting server: /home/.../.cargo/target/debug/evermemos
     cwd=.../evermemos-rs  log=/tmp/evermemos.log
  ✓ server ready in 1.0s (PID=773341)
  ✓ GET /health → server reachable at http://localhost:8080
```

outputv：
```
▶ Step 0 — Health check
  ✓ server already running at http://localhost:8080
  ✓ GET /health → server reachable at http://localhost:8080
```

## 影响文件

| 文件 | 改动 |
|------|------|
| `demo/server_utils.py` | 新增，提供 `ensure_server()` |
| `demo/test_completeness.py` | 在 Step 0 调用 `ensure_server()`，更新 docstring |
| `demo/parity_test.py` | 在 `main()` 调用 `ensure_server()`，更新 docstring |

## 验证

在无服务器状态下运行：
```bash
pkill -f "debug/evermemos"
python evermemos-rs/demo/test_completeness.py
```
→ `⚡ Starting server` → `✓ server ready in 1.0s` → 测试正常继续执行
