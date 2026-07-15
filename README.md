# tiny-dock

一个基于 [Tauri v2](https://tauri.app/) + [Svelte 5](https://svelte.dev/) 的轻量级 Windows 应用停靠栏（Dock），仿 macOS Dock 风格。

## 功能特性

- 🖥️ 停靠栏自动吸附到屏幕底部中央，跟随任务栏上方
- 🔍 鼠标悬停时图标放大效果（Magnification）
- 📌 从开始菜单快捷方式（.lnk）自动解析应用并提取图标
- 🖼️ 使用 Windows GDI 提取应用图标并缓存为 Base64 PNG
- 📋 应用进程追踪（启动、退出、重复实例检测）
- 🪟 无焦点窗口（WS_EX_NOACTIVATE），不干扰当前工作窗口
- 📺 全屏应用自动隐藏停靠栏
- ⚙️ 独立的设置窗口，可调整图标大小、间距、边距等

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Svelte 5 (Runes), TypeScript, Vite 6 |
| 后端 | Rust, Tauri v2 |
| 平台 | Windows 专属（依赖 Win32 API） |

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/) >= 9
- [Rust](https://www.rust-lang.org/) >= 1.75
- Windows 10/11

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
# 仅前端
pnpm dev

# 前端 + 后端（Tauri）
pnpm tauri dev
```

### 构建生产版本

```bash
pnpm build
```

### 类型检查

```bash
pnpm check
```

## 项目结构

```
tiny-dock/
├── src/                        # SvelteKit 前端
│   ├── lib/
│   │   ├── components/         # 停靠栏组件
│   │   │   ├── DockBar.svelte
│   │   │   ├── DockItem.svelte
│   │   │   └── DockSeparator.svelte
│   │   └── stores/
│   │       └── dockStore.svelte.ts  # 状态管理 + 事件监听
│   └── routes/
│       ├── +page.svelte        # 主窗口（停靠栏）
│       └── settings/+page.svelte # 设置窗口
├── src-tauri/
│   └── src/
│       ├── lib.rs              # Tauri 命令 + 窗口初始化
│       ├── config.rs           # 配置读写（JSON）
│       ├── launcher.rs         # 进程追踪
│       ├── tray.rs             # 系统托盘
│       └── fullscreen_detector.rs  # 全屏检测
├── static/
├── package.json
└── src-tauri/Cargo.toml
```

## 许可证

本项目采用 [MIT](LICENSE) 协议开源。
