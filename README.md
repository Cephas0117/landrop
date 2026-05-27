<div align="center">

<img src="src-tauri/icons/icon.svg" width="96" height="96" alt="LANDrop icon" />

# LANDrop

**局域网点对点文件传输工具**

跨平台 · 无需服务器 · 端对端加密 · 极速传输

[![Release](https://img.shields.io/github/v/release/Cephas0117/landrop?style=flat-square)](https://github.com/Cephas0117/landrop/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey?style=flat-square)](#下载)

[下载](#下载) · [功能特性](#功能特性) · [快速开始](#快速开始) · [开发指南](#开发指南)

</div>

---

## 简介

LANDrop 是一款运行在局域网内的点对点文件传输应用，无需互联网连接，无需账号注册，无需中转服务器。只要两台设备在同一个 Wi-Fi 下，即可安全、快速地互传任意文件。

- **macOS ↔ macOS**：直接传输
- **macOS ↔ Windows**：跨平台无缝互通
- **Windows ↔ Windows**：直接传输

## 功能特性

### 设备发现
- **mDNS 自动发现**：同局域网设备自动出现，无需手动输入 IP
- **UDP 广播兜底**：mDNS 不可用时自动切换广播模式（端口 7777）
- **手动 IP 添加**：支持直接输入 IP 地址连接特定设备
- **雷达 UI**：实时显示发现的设备，动态扫描动画

### 安全配对
- **6 位 PIN 码验证**：首次连接需在两台设备上确认相同 PIN 码
- **TOFU 信任模型**：配对后记住对方指纹，后续连接免验证
- **TLS 1.3 加密**：所有传输数据均通过 TLS 1.3 加密，防止中间人攻击
- **证书指纹持久化**：可信设备指纹本地存储，重启后仍有效

### 文件传输
- **任意文件类型**：图片、视频、文档、压缩包……无限制
- **文件夹支持**：整个文件夹一键发送
- **断点续传**：网络中断后可从中断处继续
- **实时进度**：EWMA 算法计算瞬时速度与剩余时间（ETA）
- **拖放发送**：直接把文件拖进窗口即可发送

### 界面体验
- **玻璃态 UI**：深色玻璃拟态设计，视觉清晰
- **中文界面**：完整中文本地化
- **Toast 通知**：发现设备、传输完成、错误等均有即时提示
- **启动加载**：应用初始化期间显示加载动画
- **传输记录**：历史传输记录随时查看

---

## 下载

前往 [Releases 页面](https://github.com/Cephas0117/landrop/releases/latest) 下载对应平台安装包：

| 平台 | 文件 | 说明 |
|------|------|------|
| macOS（Apple Silicon）| `LANDrop_x.x.x_aarch64.dmg` | M1 / M2 / M3 / M4 |
| macOS（Intel）| `LANDrop_x.x.x_x64.dmg` | x86_64 |
| Windows | `LANDrop_x.x.x_x64-setup.exe` | NSIS 安装程序（推荐）|
| Windows | `LANDrop_x.x.x_x64_zh-CN.msi` | MSI 安装包 |

> **macOS 提示**：首次打开时系统可能提示"无法验证开发者"，请前往「系统设置 → 隐私与安全性」点击「仍要打开」。

---

## 快速开始

### 发送文件

1. 打开 LANDrop，等待左侧雷达扫描到对方设备
2. 点击设备卡片选中目标设备（右上角会显示"发送至 xxx"）
3. 拖放文件到窗口，**或**点击右上角「发送文件」按钮选择文件
4. 等待对方在弹出的确认框中接受

### 接收文件

1. 保持 LANDrop 在后台运行
2. 对方发起传输后，屏幕上会弹出接收确认
3. 文件默认保存到系统下载目录，可在「设置」中修改

### 首次配对

首次向一台新设备发送文件时需要配对：

1. 点击设备卡片上的「配对」按钮
2. 两台设备屏幕上分别显示 6 位 PIN 码
3. 确认两端 PIN 相同后点击「接受」
4. 配对完成，后续传输无需重新配对

---

## 开发指南

### 环境要求

| 工具 | 版本 |
|------|------|
| Rust | ≥ 1.75 |
| Node.js | ≥ 20 |
| npm | ≥ 10 |
| Tauri CLI | 2.x |

macOS 还需要 Xcode Command Line Tools：
```bash
xcode-select --install
```

### 克隆与运行

```bash
git clone https://github.com/Cephas0117/landrop.git
cd landrop

# 安装前端依赖
cd frontend && npm install && cd ..

# 启动开发模式（热重载）
cargo tauri dev
```

### 项目结构

```
landrop/
├── frontend/                  # Svelte 5 前端
│   └── src/
│       ├── App.svelte         # 应用入口
│       ├── components/        # UI 组件
│       │   ├── common/        # Toast、ProgressBar 等公共组件
│       │   ├── discovery/     # 雷达、设备卡片
│       │   ├── pairing/       # PIN 配对弹窗
│       │   └── transfer/      # 传输列表、拖放区
│       └── lib/
│           ├── ipc.ts         # Tauri IPC 封装
│           └── stores/        # Svelte 5 响应式状态
├── src-tauri/                 # Tauri 主进程
│   └── src/
│       ├── commands.rs        # IPC 命令处理
│       ├── events.rs          # 事件常量
│       ├── lib.rs             # 应用入口 + 事件桥接
│       └── state.rs           # 应用状态初始化
└── crates/                    # Rust 核心功能库
    ├── landrop-core/          # 服务容器、连接抽象
    ├── landrop-discovery/     # mDNS + UDP 广播发现
    ├── landrop-protocol/      # 自定义消息帧协议（MessagePack）
    ├── landrop-security/      # TLS 证书、TOFU 配对、信任存储
    ├── landrop-transfer/      # 传输引擎、EWMA 速度追踪
    ├── landrop-fs/            # 文件读写、目录 manifest
    ├── landrop-state/         # 应用状态定义
    └── landrop-platform/      # 平台相关（防火墙、权限）
```

### 技术栈

| 层次 | 技术 |
|------|------|
| 桌面框架 | [Tauri 2](https://tauri.app) |
| 前端 | [Svelte 5](https://svelte.dev)（Runes 响应式）+ TypeScript |
| 后端 | Rust 1.75+ |
| 传输层 | TCP + TLS 1.3（tokio-rustls + rcgen 自签名证书）|
| 消息帧 | MessagePack（rmp-serde）|
| 设备发现 | mDNS（mdns-sd）+ UDP 广播 |

### 打包发布

```bash
# 构建当前平台安装包
cargo tauri build
```

产物位于 `target/release/bundle/`。

跨平台构建（macOS + Windows）通过 GitHub Actions 自动完成，推送 `v*` tag 即可触发：

```bash
git tag v1.0.0
git push origin v1.0.0
```

---

## 协议说明

LANDrop 使用自定义二进制协议，基于 MessagePack 消息帧，运行在 TLS 1.3 之上：

```
HELLO → CAPS_ACK → PAIR_REQUEST → PAIR_ACCEPT
                 → MANIFEST → MANIFEST_ACK
                 → CHUNK × N → CHUNK_ACK × N
                 → DONE → DONE_ACK
```

- 传输端口：TCP 7878
- 发现端口：UDP 7777 / mDNS
- 块大小：256 KB
- 速度计算：EWMA（α = 0.25）

---

## License

MIT © 2026 LANDrop
