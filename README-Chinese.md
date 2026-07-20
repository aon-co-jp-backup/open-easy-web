# open-easy-web

**"第二个 KUSANAGI" — 上传应用后通过 IP 地址启动，轻松完成域名注册与
自动 HTTPS(Rust → WebAssembly，不依赖任何框架)**

如同 WordPress 加速套件 "KUSANAGI" 一样，`open-easy-web` 致力于实现
"上传应用 → 通过 IP 地址启动 → 简化域名注册 → 自动 HTTPS" 的一体化
流程。提供可注册/切换/测试多个站点连接信息的"站点管理"界面，并可为
WordPress、PHP+Laravel、Python+FastAPI 或任意后端生成基础的反向代理
vhost 配置(Nginx/Apache)。**不具备数据库连接功能**(有意排除在范围
之外)。

**2026-07-13 从 `aruaru-web` 拆分**: `aruaru-web` 原本开发的"简单
域名/子域名注册与删除"、"HTTPS 自动监控/签发/续期"、"上传后的简单
站点运维"——**除 KUSANAGI 加速功能之外的全部**——已迁移至此仓库。
加速功能(gzip 压缩、静态资源长期缓存、FastCGI 缓冲区调优、
upstream keepalive 连接池)不再以 Nginx/Apache 配置生成的形式提供，
而是整合为 **`open-runo`/`poem-cosmo-tauri` 中的原生 Rust(hyper
中间件)实现**(gzip 响应压缩中间件、静态资源 Cache-Control 中间件等，
详见两仓库的 CLAUDE.md)。

📖 其他语言: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## 当前功能

- **站点管理界面**: 注册/编辑/删除多个部署目标(名称/用途/协议/主机/
  端口/路径/后端类型),保存于 `localStorage`。每张卡片提供"连接测试"
  按钮(`fetch(url, {mode:'no-cors'})` 的简单可达性检测)、端口号校验
  (1-65535)、删除前确认对话框、站点列表 JSON 导出/导入。
- **通过 IP 地址启动**: `scripts/serve.sh <BIND_IP> <PORT>`。
- **vhost 生成 + 自动 HTTPS**: `scripts/gen-vhost.sh [--stack=STACK]
  <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` 为 `static`、`proxy`
  (通用反向代理)、`wordpress`、`laravel`、`fastapi` 五种 stack 生成
  Nginx/Apache vhost(含 HTTP→HTTPS 跳转与 ACME challenge 路径)。
  **本仓库不包含加速调优**——详见 `open-runo`/`poem-cosmo-tauri`。
- **HTTPS 自动监控/续期**: `scripts/setup-tls.sh`(certbot 获取 Let's
  Encrypt 证书)、`deploy/systemd/install-systemd-units.sh` 启用
  `easyweb-tls-renew.timer`(每日两次自动续期)与
  `easyweb-tls-monitor.timer`(每日一次到期监控)。
- **VPS 部署**: `scripts/deploy-vps.ps1`(Windows PowerShell)自动完成
  构建→上传→启动。

## 当前不具备的功能

- **不含 Web 加速**(gzip/静态缓存/FastCGI 缓冲调优/upstream
  keepalive 连接池)——参见 `open-runo`/`poem-cosmo-tauri` 的原生
  Rust 实现。
- **不含任何数据库连接功能**。
- 未实现认证、分页、错误自动重试。
- 不提供类似 Tauri 的原生应用体验(仅浏览器运行的 WASM)。
- **不在本仓库内执行实际的域名购买/DNS 记录注册**(注册商操作)或 VPS
  租用——这些由用户自行完成,本仓库仅自动化已注册域名的 vhost 生成与
  TLS 证书生命周期管理。

## 构建

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **构建注意事项(网络驱动器环境)**: 如果本仓库位于网络共享驱动器
> (如 SMB 挂载)上,直接在该驱动器上读写 `cargo build` 的 `target/`
> 输出或 `wasm-bindgen` 的输入/输出,**写入后立即读取可能返回旧内容**
> (读取缓存不一致,2026-07-20 实际发生并确认)。若重新构建后改动未生效,
> 可用 `cargo build --target-dir <本地临时目录>` 将构建输出指向网络
> 驱动器之外(如本地 C 盘),再对该本地副本运行 `wasm-bindgen` 即可解决。

## 本次验证情况

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` 均
成功且零警告。`gen-vhost.sh` 已对全部 5 种 stack 验证(占位符替换
正确)。由于本开发环境为 Windows 且无可用的 nginx/apache 二进制,
本次**未**执行 `nginx -t`/`apache2ctl configtest` 的真实语法检查——
模板是从 aruaru-web 已验证模板中"仅删减"指令得到的差分,未新增任何
语法。详见 `CLAUDE.md`。

## 相关项目

- **aruaru-web**(拆分来源): https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**(开发规范正本): https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
