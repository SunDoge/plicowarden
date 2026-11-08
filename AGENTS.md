# Plicowarden

- 独立 GPL-3.0-only Rust 密码客户端；Plico 框架保持 Apache-2.0。
- 应用与 Bitwarden SDK、SSH agent 均在本仓库维护，不将应用服务代码移入 Plico。
- Plico 依赖统一使用 Cargo.toml 中的完整 Git revision；更新时同步所有 Plico 直接依赖并更新 Cargo.lock，不引入相邻 checkout 的 path dependency，不依赖旧 Warden/Wabou。
- CLI 使用 usage-rs，错误使用 Snafu。组件遵循 UiContext 首参普通函数与 #[component]，布局使用 + |cx|。
- 私钥与解密服务留在 sdk worker，UI 只接收签名审批的公开元数据。签名逐次明确批准，锁定、停用、超时必须拒绝。
- reports/、target/、node_modules/ 是本地产物，不提交。截图与 recipe 只使用合成数据。
- 测试 cargo test --locked --workspace；静态检查 cargo clippy --locked --workspace --all-targets -- -D warnings；recipe python3 scripts/run-recipes.py。
- TypeScript 依赖与运行统一使用 Bun；根目录 bun.lock 是唯一 JS 锁文件。bun install --frozen-lockfile、bun run check、bun run test；不引入 npm/pnpm/yarn 锁文件。
- 提交使用 Conventional Commits；未经要求不提交、不推送。
