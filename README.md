## ModsUpdater

[English](#english)

## 中文

跨平台 Minecraft mod 更新工具，通过 [Modrinth API](https://modrinth.com/) 检查并应用更新。

**特性：**

- **检查 & 更新** — 扫描本地 `.jar` 模组文件，在 Modrinth 上查找可用更新
- **安全设计** — 更新文件保存到 `./mods_update/`；**绝不**修改原始 mod 文件
- **跨平台** — 支持 Windows 和 Linux（macOS 未测试但理论上可用）
- **GUI + CLI** — 图形界面（egui）或命令行双模式
- **双语支持** — 英文 + 中文（通过 `$LANG` 环境变量自动检测）

### 安装

#### 预编译二进制

从 [Releases](https://github.com/Strad65/ModsUpdater/releases) 页面下载最新版本。

| 文件                       | 说明     |
| ------------------------ | ------ |
| `modsupdater-v0_1_0`     | 命令行工具  |
| `modsupdater-gui-v0_1_0` | 图形界面程序 |

#### 从源码编译

```bash
git clone https://github.com/Strad65/ModsUpdater.git
cd ModsUpdater
cargo build --release
```

编译后的二进制文件在 `target/release/` 目录下。

### 用法

#### GUI

```bash
./modsupdater-gui-v0_1_0
```

1. 浏览选择 Minecraft `mods/` 文件夹
2. 输入加载器（如 `fabric`）和游戏版本（如 `1.21.1`）
3. 点击 **Check Updates**（检查更新）
4. 勾选需要更新的 mod，点击 **Update Selected**（更新选中项）
5. 更新后的文件出现在 `./mods_update/` 目录

#### CLI

```bash
# 检查更新（不修改文件）
modsupdater-v0_1_0 check -d ~/.minecraft/mods

# 应用更新
modsupdater-v0_1_0 update -d ~/.minecraft/mods

# 按名称搜索
modsupdater-v0_1_0 check -n sodium -n iris

# 从文件读取 mod 名称
modsupdater-v0_1_0 check -f mods.txt

# 指定加载器 / 游戏版本
modsupdater-v0_1_0 check -d ./mods -l forge -g 1.20.1

# 递归扫描子目录
modsupdater-v0_1_0 check -d ./mods -r

# 试运行（仅预览，不下载）
modsupdater-v0_1_0 update -d ./mods --dry-run

# 跳过确认提示
modsupdater-v0_1_0 update -d ./mods -y

# 自定义报告路径
modsupdater-v0_1_0 check -d ./mods -o report.md

# 管理配置
modsupdater-v0_1_0 config --set-dir ~/.minecraft/mods --set-loader fabric --set-game-version 1.21.1

# 从上次运行生成报告
modsupdater-v0_1_0 report -o report.md
```

### 项目结构

```
ModsUpdater/
├── crates/
│   ├── core/          # 核心库：API 客户端、扫描器、更新器
│   ├── cli/           # CLI 前端（clap + indicatif）
│   └── gui/           # GUI 前端（egui/eframe）
├── Cargo.toml         # 工作区配置
└── README.md
```

### 系统要求

- **Rust** 1.80+（源码编译时需要）
- 网络连接（需要访问 Modrinth API）

---

## English

A cross-platform Minecraft mod update tool that checks and applies updates via the [Modrinth API](https://modrinth.com/).

**Features:**

- **Check & Update** — Scan local `.jar` mod files and find available updates on Modrinth
- **Safe by design** — Updated files are saved to `./mods_update/`; original mods are **never** touched
- **Cross-platform** — Works on Windows and Linux (macOS untested but should work)
- **GUI + CLI** — Graphical interface (egui) or command-line for scripting
- **Bilingual** — English + Chinese (auto-detected from system locale `$LANG`)

### Installation

#### Pre-built binaries

Download the latest release from the [Releases](https://github.com/Strad65/ModsUpdater/releases) page.

| Binary                   | Description |
| ------------------------ | ----------- |
| `modsupdater-v0_1_0`     | CLI tool    |
| `modsupdater-gui-v0_1_0` | GUI app     |

#### Build from source

```bash
git clone https://github.com/Strad65/ModsUpdater.git
cd ModsUpdater
cargo build --release
```

Binaries are at `target/release/`.

### Usage

#### GUI

```bash
./modsupdater-gui-v0_1_0
```

1. Browse to your Minecraft `mods/` folder
2. Enter your loader (e.g. `fabric`) and game version (e.g. `1.21.1`)
3. Click **Check Updates**
4. Select the mods you want to update, then click **Update Selected**
5. Updated files appear in `./mods_update/`

#### CLI

```bash
# Check for updates (no files changed)
modsupdater-v0_1_0 check -d ~/.minecraft/mods

# Apply updates
modsupdater-v0_1_0 update -d ~/.minecraft/mods

# Search by mod name
modsupdater-v0_1_0 check -n sodium -n iris

# Read mod names from a file
modsupdater-v0_1_0 check -f mods.txt

# Change loader / game version
modsupdater-v0_1_0 check -d ./mods -l forge -g 1.20.1

# Recursive scan
modsupdater-v0_1_0 check -d ./mods -r

# Dry run (preview without downloading)
modsupdater-v0_1_0 update -d ./mods --dry-run

# Skip confirmation prompt
modsupdater-v0_1_0 update -d ./mods -y

# Custom report path
modsupdater-v0_1_0 check -d ./mods -o report.md

# Manage config
modsupdater-v0_1_0 config --set-dir ~/.minecraft/mods --set-loader fabric --set-game-version 1.21.1

# Generate report from last run
modsupdater-v0_1_0 report -o report.md
```

### Project Structure

```
ModsUpdater/
├── crates/
│   ├── core/          # Core library: API client, scanner, updater
│   ├── cli/           # CLI frontend (clap + indicatif)
│   └── gui/           # GUI frontend (egui/eframe)
├── Cargo.toml         # Workspace root
└── README.md
```

### Requirements

- **Rust** 1.80+ (build from source)
- Internet connection (Modrinth API access)
