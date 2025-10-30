# korman_screeps_bot 项目概述

korman_screeps_bot 是一个基于 Rust 开发的 Screeps AI bot 项目，专为 Screeps:
World MMO 游戏设计。通过 WebAssembly (WASM) 技术，将 Rust 代码编译为高效
bot，实现游戏内的资源采集、建筑管理、防御策略等功能。项目采用 Legion ECS
架构，确保代码模块化、高性能，适合新手学习 Rust 编程和游戏 AI 开发。

## 项目功能介绍

- **基础 AI 逻辑**：支持自动采集能量、升级控制器、生成 creep 等核心操作。
- **ECS 架构**：使用 Legion ECS 模拟房间布局和 creep 行为，提高扩展性。
- **性能优化**：Rust WASM bot 在计算密集任务中快 JS 2-5x，CPU 效率高。
- **部署支持**：兼容私人服务器和 MMO，易于手动或自动上传代码。

## 运行与编译指南

项目使用 wasm-pack 编译 Rust 到 WASM，Rollup 和 Babel 处理 JS
兼容性，screeps-api 部署。以下是完整步骤（整合英文 README 内容）：

1. **环境安装**：

   - 安装 rustup：访问 https://rustup.rs/。
   - 安装 wasm-pack：cargo install wasm-pack。
   - 安装 wasm-opt：cargo install wasm-opt。
   - 安装 Node.js（推荐 v20）：用 nvm 管理（Mac/Linux:
     https://github.com/nvm-sh/nvm；Windows:
     https://github.com/coreybutler/nvm-windows）。运行 nvm install 20 && nvm
     use 20。

2. **克隆与配置**：

   - 克隆项目：git clone https://github.com/korman/korman_screeps.git && cd
     korman_screeps。
   - 自定义 crate 名：如果修改 Cargo.toml 中的 name，更新 js_src/main.js 中的
     MODULE_NAME 和 import，以及 package.json 的 "name"。
   - 安装 JS 依赖：npm install。

3. **配置部署**：

   - 拷贝示例配置：cp .example-screeps.yaml .screeps.yaml，编辑服务器设置（如
     MMO 用 token，私人服务器用 host/port）。

   - 私人服务器示例：

     ```yaml
     servers:
       private-server:
         host: 127.0.0.1
         port: 21025
         secure: false
         branch: default
     ```

4. **编译与部署**：

   - 模拟部署：npm run deploy -- --server ptr --dryrun（检查但不上传）。
   - 真实部署：npm run deploy -- --server private-server。

5. **迁移到新版**（如果从旧版升级）：

   - 创建 .screeps.yaml，从 screeps.toml 迁移设置。
   - 添加 .gitignore：.screeps.yaml、node_modules、dist。
   - 拷贝 package.json 并自定义 name。
   - 安装 npm 依赖，拷贝 deploy.js 到 js_tools/，main.js 到 js_src/（更新 import
     和 MODULE_NAME）。
   - Cargo.toml 更新 screeps-game-api 到 v0.23.1。
   - 测试：npm run deploy -- --server ptr --dryrun。

6. **故障排除**：

   - "Not Authorized"：YAML 密码加双引号 "12345"。
   - "Unknown module"：更新 package.json "name"。
   - "Invalid opcode"：Cargo.toml 添加 --signext-lowering。

## 代码格式化规范

- **Rust 代码**：使用 cargo fmt 进行格式化，确保所有 .rs 文件符合 Rust
  风格指南。运行 cargo fmt 自动应用。
- **JavaScript/TypeScript/JSON**：使用 deno fmt --unstable-component
  命令格式化，确保一致性。运行 deno fmt js_src/ 和 deno fmt *.json。

## 项目结构说明

- src/：Rust 源代码，包括 lib.rs (主逻辑)。
- js_src/：JS 绑定文件，如 main.js (WASM 加载)。
- js_tools/：部署脚本，如 deploy.js。
- pkg/：WASM 编译输出（临时）。
- dist/：Rollup 打包输出（部署用）。
- .screeps.yaml：部署配置。
- Cargo.toml：Rust 依赖和设置。
- package.json：JS 依赖和脚本。

## 开发与贡献指南

- **开发**：fork 仓库，feature 分支开发，cargo test 测试 Rust，npm run watch
  热重载 JS。提交 PR 前 cargo fmt 和 deno fmt。
- **贡献**：欢迎 bug fix、new feature 或 doc 改进。提交 PR 时描述变化，引用
  issue。
- **社区**：Discord #rust 频道，GitHub issues 讨论。

**Key Citations**：

- [rustyscreeps/screeps-starter-rust - GitHub](https://github.com/rustyscreeps/screeps-starter-rust)
- [Screeps #27: Optimizing Pathfinding with Rust | Field Journal](https://jonwinsley.com/notes/screeps-clockwork)
- [Redesign Screeps in Rust : r/rust - Reddit](https://www.reddit.com/r/rust/comments/vf7758/redesign_screeps_in_rust/)
- [rustyscreeps/screeps-game-api - GitHub](https://github.com/rustyscreeps/screeps-game-api)
- [Automating Base Planning in Screeps – A Step-by-Step Guide](https://sy-harabi.github.io/Automating-base-planning-in-screeps/)
