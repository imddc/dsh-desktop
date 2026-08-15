# prod-front-dsh-client — DeepSeek Harness 桌面客户端

用 **Tauri 2** 把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh web`）包进一个原生桌面窗口。启动时自动拉起本地 `dsh web` 服务（动态端口，与已有实例互不冲突），就绪后由原生 WebView 加载 GUI；退出窗口时自动结束服务进程。

（项目命名遵循 [NAMING.md](../NAMING.md)：`prod` 个人产品 + `front` 桌面端 + `dsh-client` 主题）

## 功能

- 一条命令启动整个 Harness GUI，无需手动跑 `dsh web`
- **开箱即用**：捆绑 Node.js 与 `@deepseek-ai/dsh`，别人拿到 .app 双击即跑，无需任何依赖
- 动态端口（`--port 0`），不会与已在运行的 3080 实例冲突
- 优先使用捆绑运行时；缺失时回退 PATH / npx 缓存 / nvm / vite-plus / 环境变量
- 窗口关闭即清理子进程，不留孤儿服务
- 加载页显示启动状态，失败时给出可读错误

## 开发

```sh
npm install
node scripts/prepare-runtime.mjs   # 下载捆绑 node + 安装 dsh（构建前必须）
npm run dev                        # 开发模式
```

## 打包

```sh
node scripts/prepare-runtime.mjs   # 准备捆绑运行时（首次或更新 dsh 后）
npm run build                      # 产出 .app / .dmg
npm run dist                       # 构建 + 产物复制到项目根 dist/
```

产物在 `src-tauri/target/release/bundle/` 或项目根 `dist/`。

## 发布（GitHub Actions 自动构建）

push 一个 `v*` tag 即触发 CI，自动构建并发布 GitHub Release：

```sh
git tag v0.2.0
git push origin v0.2.0
```

Release 附件包含 `.dmg`（安装包）与 `.app`（免安装），别人直接下载使用。
CI 会现场执行 `prepare-runtime.mjs` 捆绑最新 node + dsh，无需手动维护运行时。

## 环境变量（可选）

| 变量 | 作用 |
|---|---|
| `DSH_CLIENT_DSH_BIN` | 指定 `dsh` 可执行文件路径 |
| `DSH_CLIENT_NODE_BIN` | 指定 `node` 二进制路径 |
| `DSH_CLIENT_WORKSPACE` | 指定 dsh 的工作目录（默认 `$HOME`） |

## 工作原理

```
Tauri 启动
  └─ 后台线程: 定位 node + dsh → spawn `dsh web --port 0 --host 127.0.0.1`
  └─ 主窗口: 加载本地 loading 页
        └─ 轮询 boot_status command
              ├─ starting → 继续等
              ├─ ready(url) → window.location = url
              └─ failed(msg) → 显示错误
窗口销毁 → app.exit(0) → RunEvent::Exit → kill 子进程
```

## 目录结构

- `src/index.html` — 启动 loading 页（轮询 `boot_status` 后跳转）
- `src-tauri/src/lib.rs` — 全部逻辑：进程定位、spawn、URL 解析、退出清理
- `src-tauri/tauri.conf.json` — 窗口在代码中创建，配置里不预声明
- `scripts/prepare-runtime.mjs` — 下载捆绑 node + 安装 dsh 到 `src-tauri/resources/`
- `scripts/copy-dist.mjs` — 构建后把产物复制到项目根 `dist/`
- `.github/workflows/release.yml` — push tag 自动构建并发布 Release
