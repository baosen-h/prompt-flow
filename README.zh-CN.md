<p align="center">
  <img src="assets/prompt-flow-icon.png" alt="prompt-flow 图标" width="140" />
</p>

<h1 align="center">prompt-flow</h1>

<p align="center">
  <a href="README.md">English</a> · 简体中文
</p>

<p align="center">
  <a href="https://github.com/baosen-h/prompt-flow/releases"><img src="https://img.shields.io/github/v/release/baosen-h/prompt-flow?style=flat" alt="GitHub release" /></a>
  <a href="https://github.com/baosen-h/prompt-flow/releases"><img src="https://img.shields.io/github/downloads/baosen-h/prompt-flow/total?style=flat&color=blue" alt="GitHub downloads" /></a>
</p>

## 产品介绍

`prompt-flow` 是一个轻量的提示词选择器，也可以把多个提示词按顺序组成工作流，用于 Codex 和 Claude Code。

## 演示

<table>
  <tr>
    <th align="center">Codex 工作流</th>
    <th align="center">Claude Code 工作流</th>
  </tr>
  <tr>
    <td><img src="image/codex-flow.gif" alt="prompt-flow Codex 工作流" width="100%" /></td>
    <td><img src="image/claude-flow.gif" alt="prompt-flow Claude Code 工作流" width="100%" /></td>
  </tr>
</table>

## 使用方法

1. 从 [Releases](https://github.com/baosen-h/prompt-flow/releases/latest) 下载 Windows 安装包。
2. 打开 `prompt-flow`，配置提示词和工作流。
3. 聚焦到 Codex 或 Claude Code。
4. 按 `Ctrl + Alt + P`。
5. 按 `Tab` 在 Prompt 和 Flow 之间切换。
6. 搜索、选择，然后按 `Enter`。

如果要使用工作流，请在 Flow 设置页安装 Codex 和 Claude hook。hook 的作用是：等当前回答结束后，再自动发送下一步提示词。

## 构建

```bash
npm install
npm run tauri:build
```

## 说明

- 网页文本框可以使用普通提示词插入，但网页不支持 Flow 模式。
