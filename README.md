# dsh-desktop

用 **Tauri 2** 把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web GUI 装进一个原生桌面窗口。双击即用：启动时自动拉起本地服务，退出自动清理，无需安装 Node.js 或 dsh。

## 特性

- **开箱即用** — 捆绑运行时，拿到 `.app` 双击即跑
- **自定义标题栏** — 无系统原生标题栏，右侧三灯（关闭 / 最小化 / 最大化），Liquid Glass 玻璃质感
- **自动清理** — 关闭窗口即结束后台服务，不留孤儿进程

## 开发

```sh
npm install
node scripts/prepare-runtime.mjs   # 准备捆绑运行时（首次）
npm run dev
```

## 打包

```sh
node scripts/prepare-runtime.mjs
npm run dist        # 产出 .app / .dmg 到 dist/
```

产物也可在 `src-tauri/target/release/bundle/` 找到。

## 发布

push 一个 `v*` tag 即触发 GitHub Actions 自动构建并发布 Release：

```sh
git tag v0.2.0
git push origin v0.2.0
```

Release 附件包含 `.dmg`（安装包）与 `.app`（免安装）。

## 致谢

- 应用图标：DeepSeek 鲸鱼娘（社区二创形象），源自 [deepseek-whale-girl-icon](https://github.com/fornarwhal/deepseek-whale-girl-icon)，授权协议 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/)（须署名、非商用、相同方式共享）。
