<p align="center">
  <img src="assets/prompt-flow-icon.png" alt="prompt-flow icon" width="140" />
</p>

<h1 align="center">prompt-flow</h1>

<p align="center">
  English · <a href="README.zh-CN.md">Simplified Chinese</a>
</p>

<p align="center">
  <a href="https://github.com/baosen-h/prompt-flow/releases"><img src="https://img.shields.io/github/v/release/baosen-h/prompt-flow?style=flat" alt="GitHub release" /></a>
  <a href="https://github.com/baosen-h/prompt-flow/releases"><img src="https://img.shields.io/github/downloads/baosen-h/prompt-flow/total?style=flat&color=blue" alt="GitHub downloads" /></a>
</p>

## Product Introduction

`prompt-flow` is a tiny prompt picker and prompt workflow tool for Codex and Claude Code.

## Demos

<table>
  <tr>
    <th align="center">Codex Flow</th>
    <th align="center">Claude Code Flow</th>
  </tr>
  <tr>
    <td><img src="image/codex-flow.gif" alt="prompt-flow Codex workflow" width="100%" /></td>
    <td><img src="image/claude-flow.gif" alt="prompt-flow Claude Code workflow" width="100%" /></td>
  </tr>
</table>

## Usage

1. Download the Windows installer from [Releases](https://github.com/baosen-h/prompt-flow/releases/latest).
2. Open `prompt-flow` to configure prompts and flows.
3. Focus Codex or Claude Code.
4. Press `Ctrl + Alt + P`.
5. Press `Tab` to switch between Prompt and Flow.
6. Search, choose, and press `Enter`.

For flows, install the Codex and Claude hooks from the Flow settings page. Hooks let `prompt-flow` send the next step after the current answer finishes.

## Build

```bash
npm install
npm run tauri:build
```

## Notes

- Web text boxes can use normal prompt insertion, but web does not support flow mode.
