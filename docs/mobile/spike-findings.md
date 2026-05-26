# Android Cross-Compile Spike 记录 (Phase 0)

> 状态: GHA 已触发,等待运行结果。Run URL: <填写实际运行 URL>

## 执行环境
- GHA ubuntu-latest, NDK r26d, target aarch64-linux-android24
- 命令: `cargo check --target aarch64-linux-android --lib`
- 时间: <待填>

## 结果
- [ ] cargo check 通过
- [ ] cargo check 失败,错误源 (勾选所有适用):
  - [ ] cpal (不支持 Android,需在 handy-core 中隔离)
  - [ ] gtk-layer-shell / gtk (Linux 桌面专属)
  - [ ] whisper-rs cmake (whisper.cpp 编译失败)
  - [ ] onnxruntime-sys (Parakeet 引擎依赖)
  - [ ] rdev / enigo (输入模拟)
  - [ ] tauri-plugin-* (部分 plugin 已 cfg-guard,但可能还有遗漏)
  - [ ] 其他: ___

## 关键错误片段
```
<待粘贴 spike.log 中前 50 行关键错误>
```

## 后续阶段对策
- 阶段 1 抽 handy-core 时确保隔离: <待填>
- 阶段 2/3 在 src-mobile/ 中需要的额外 cfg-guard: <待填>
