# Handy Mobile Phase 2a — Android Debug APK 端到端打通

> **状态**: 设计稿,待评审
> **日期**: 2026-05-27
> **作者**: Claude Code (Opus 4.7) + 唐海文
> **关联**:
> - [2026-05-26 总体设计](./2026-05-26-handy-mobile-design.md)
> - [Phase 0+1 实施计划](../plans/2026-05-26-handy-mobile-phase-0-1.md)
> - [docs/mobile/README.md](../../mobile/README.md)

---

## 1. 背景

Phase 0+1 已完成 (`mobile/phase-1-workspace` 分支 + PR #1):

- Cargo workspace 引入,根 manifest 就位
- `crates/handy-core` 抽出来:`text` / `audio` / `vad` (可选 feature) / `history` / `model 数据类型` / `settings 骨架`
- `crates/handy-platform` 定义了四个 trait:`AudioCapture` / `TextOutput` / `AppStorage` / `EventSink`
- `handy-core` 已验证可交叉编译到 `aarch64-linux-android` / `armv7-linux-androideabi`(关闭 `vad` feature 后)
- CI `mobile-ci.yml` 全绿(6 个 job)

**当前状态**: 有共享 Rust 核心,但没有任何移动端 Tauri 工程,没有任何 Android UI 代码,没有 APK 产出。

## 2. 目标 (Phase 2a 范围)

产出**第一个能装到 Android 真机/模拟器上的 debug APK**,完成单一端到端流程:

```
按住按钮 → 录音 → Whisper Tiny 转录 → 屏幕显示文字 → 点"复制"进剪贴板
```

**这是一个垂直切片**:每一层都最薄,优先证明整条链路在 Android 上跑得通;能力扩展(History/Models/Settings/前台服务/IME)放在后续 phase。

### 2.1 非目标 (Out of Scope)

明确**不做**(各自有后续 phase 承接):

- ❌ 系统输入法 (IME) — phase 3
- ❌ Android 前台录音服务 + 锁屏/后台录音 — phase 2c
- ❌ History / Models / Settings UI(三页) — phase 2b
- ❌ Voice Activity Detection (VAD) — phase 2c(或换实现)
- ❌ Sqlite 持久化(任何历史记录) — phase 2b
- ❌ iOS — 单独立项
- ❌ 模型推荐 / 网络检测 / 多模型切换 — phase 2b
- ❌ APK 签名 / 上架发布 — phase 4
- ❌ 桌面端任何 UI / 行为变更
- ❌ `bun workspaces` 升级 — 维持 phase 0+1 决策,`src-mobile-ui/` 独立 `bun install`

## 3. 范围内变更

### 3.1 新增工程

- **`src-mobile/`** — Tauri Mobile Rust 工程
- **`src-mobile-ui/`** — 移动端 React + Vite + Tailwind
- **`scripts/enter-android-env.ps1`** — Android 工具链环境注入 (Windows)
- **`docs/mobile/android-build.md`** — Windows + Android Studio 构建指引(简版,phase 2a 用)

### 3.2 改动现有工程

- **根 `Cargo.toml`** workspace `members` 加 `"src-mobile"`
- **根 `package.json`** scripts 加 `mobile:*` / `tauri:android:*` 入口(仅追加)
- **`.github/workflows/mobile-ci.yml`** 加 `android-debug-apk` job(`cargo tauri android build --apk --debug` + 上传 artifact)
- **`docs/mobile/README.md`** 更新 phase 2a 入口与状态

### 3.3 关键 YAGNI 决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Android 平台代码位置 | `src-mobile/src/platform/android.rs`,**不**开 `crates/handy-mobile` | YAGNI;phase 2a 平台代码量小(只 1 个 `AudioCapture` impl + clipboard)。等 phase 2c 加前台服务/JNI 时再决定是否抽 crate |
| 首版默认模型 | **Whisper Tiny q5_0 (~39MB)** | 测试反馈循环短;Parakeet V3 (478MB) 留给 phase 2b |
| VAD | **关闭** (`handy-core` 不开 `vad` feature) | `ort-sys` 无 Android 预编译;phase 2a 用按住说话按钮的按下/松开作显式分段 |
| Sqlite | 不引入 | 没有 History 页就不需要 |
| 网络栈 | `reqwest` + `rustls-tls`(显式) | Android 上 `native-tls` 不可用 |
| 前端代码组织 | `src-mobile-ui/` 独立 `package.json` + vite alias 复用桌面端 i18n & UI 子集 | phase 0+1 已选,继续 |

## 4. 架构

### 4.1 分层(本 phase 实际涉及)

```
┌─────────────────────────────────────────────┐
│ src-mobile-ui/  (React + Vite,装 APK 时打包) │
│   App.tsx (单页): 录音按钮 / 转录结果 / 复制   │
└──────────────────┬──────────────────────────┘
                   │ Tauri commands / events
┌──────────────────▼──────────────────────────┐
│ src-mobile/  (Tauri Mobile, mobile_entry)    │
│   commands/transcription.rs   ← 装配点        │
│   platform/android.rs                        │
│     AndroidAudioCapture  : AudioCapture      │
│     AndroidTextOutput    : TextOutput        │
│     AndroidStorage       : AppStorage        │
│     TauriEventSink       : EventSink         │
│   download.rs  ← 首次启动下载 Whisper Tiny    │
└──────────────────┬──────────────────────────┘
                   │ Arc<dyn Trait>
┌──────────────────▼──────────────────────────┐
│ handy-core (vad feature OFF)                 │
│   text::*  audio::resampler  model 数据类型   │
└──────────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│ transcribe-rs (whisper feature)              │
│   whisper.cpp 静态库 (NDK 编译)               │
└──────────────────────────────────────────────┘
```

### 4.2 录音 → 转录数据流

```
[屏幕按下按钮]
    │ invoke("start_recording")
    ▼
[AndroidAudioCapture::start()]
    │ 起一个 Oboe / cpal-android stream,16kHz mono f32
    │ frames_rx() → mpsc<AudioFrame>
    ▼
[累积 buffer 到内存 Vec<f32>]   ← phase 2a 简化:不流式,先攒后转
    │
[屏幕松开按钮]
    │ invoke("stop_recording")
    ▼
[AndroidAudioCapture::stop()]
    │
[handy_core::audio::resampler]    ← 如果 Oboe 不出 16kHz 就重采样
    │
[transcribe-rs WhisperModel::transcribe(samples)]   ← 同步,~1–3s
    │ -> Vec<Segment>
    ▼
[handy_core::text::filter_transcription_output / apply_custom_words]
    │ -> String
    ▼
[Tauri command 返回 Result<String>]
    │
[前端拿到文本,显示;点"复制"按钮 → tauri-plugin-clipboard-manager]
```

**关键简化**:phase 2a **不做流式**,按住说话期间完整录到 buffer,松开后一次性送 whisper.cpp 同步转录。这避免了:
- 流式接口的 partial result event 通道
- 转录线程与 UI 线程的 cancel/abort 同步
- VAD 分段(本来就关了)

代价:用户体验上松开后有 1–3s 等待。phase 2c 加 VAD + 流式时再优化。

### 4.3 平台 trait 注入

`src-mobile/src/commands/transcription.rs` 起点(伪代码,真实接口以代码为准):

```rust
#[tauri::command]
async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    let samples = state.audio.lock().await.stop().await?;     // AndroidAudioCapture
    let resampled = handy_core::audio::resampler::to_16k(&samples)?;
    let segments = state.engine.lock().await.transcribe(&resampled)?;
    let raw = segments.into_iter().map(|s| s.text).collect::<String>();
    let text = handy_core::text::filter_transcription_output(&raw);
    Ok(text)
}
```

`AppState` 在 `setup` 时构造:

```rust
.setup(|app| {
    let storage = Arc::new(AndroidStorage::from(app.path()));
    let audio   = Arc::new(Mutex::new(AndroidAudioCapture::new()));
    let engine  = Arc::new(Mutex::new(load_whisper_tiny(&storage)?));
    let sink    = Arc::new(TauriEventSink::new(app.handle()));
    app.manage(AppState { storage, audio, engine, sink });
    Ok(())
})
```

## 5. 目录结构(本 phase 新增 / 改动)

```
Handy/
├── Cargo.toml                         # 改: workspace.members 加 src-mobile
├── package.json                       # 改: 仅追加 scripts
├── .github/workflows/mobile-ci.yml    # 改: 加 android-debug-apk job
│
├── src-mobile/                        # 新增
│   ├── Cargo.toml
│   ├── tauri.conf.json                # mobile-aware,bundle.identifier = com.handy.mobile
│   ├── build.rs                       # tauri-build
│   ├── gen/android/                   # `cargo tauri android init` 生成,checked in
│   │   ├── app/build.gradle.kts
│   │   ├── app/src/main/AndroidManifest.xml
│   │   └── app/src/main/java/com/handy/mobile/MainActivity.kt
│   └── src/
│       ├── lib.rs                     # #[tauri::mobile_entry_point] + setup
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── transcription.rs       # start_recording / stop_recording
│       │   └── model.rs               # download_model / model_status
│       ├── platform/
│       │   ├── mod.rs
│       │   ├── android.rs             # cfg(target_os="android") 全部 Android 实现
│       │   └── desktop_stub.rs        # cfg(not(target_os="android")) 桩,让 cargo check 在 host 上能过
│       └── state.rs                   # AppState 定义
│
├── src-mobile-ui/                     # 新增
│   ├── package.json                   # 独立 deps,不进 root bun workspaces
│   ├── vite.config.ts                 # alias: @desktop-i18n → ../src/i18n/locales
│   │                                  #        @desktop-ui   → ../src/components/ui
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx                    # 单页
│       ├── components/
│       │   ├── RecordButton.tsx       # 按住说话大圆按钮
│       │   ├── TranscriptDisplay.tsx
│       │   └── ModelDownloadGate.tsx  # 首启动遮罩 + 进度条
│       ├── lib/
│       │   ├── bindings.ts            # tauri-specta 生成(后期),phase 2a 手写 invoke 包装
│       │   └── api.ts                 # invoke + listen 的瘦封装
│       └── i18n/                      # 复用桌面端翻译源 via alias
│
├── scripts/
│   └── enter-android-env.ps1          # 新增
│
└── docs/mobile/
    ├── README.md                      # 改: 加 phase 2a 入口
    └── android-build.md               # 新增
```

### 5.1 `src-mobile/Cargo.toml` 依赖(关键节选)

```toml
[package]
name = "handy-mobile"   # 注意:不是 "handy",和桌面区分
version = "0.1.0"
edition.workspace = true

[lib]
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
# 共享核心(VAD 关闭,specta 在 phase 2a 不强求,后期开)
handy-core = { path = "../crates/handy-core" }
handy-platform = { path = "../crates/handy-platform" }
transcribe-rs = { workspace = true }   # whisper feature
tauri = { workspace = true, features = [
  "protocol-asset",
  # 注意:不开 tray-icon / macos-private-api / image-png — 这些是 desktop-only feature,
  # 在 mobile target 下会编译失败。桌面端 src-tauri/Cargo.toml 不变,各开各的。
] }
tauri-plugin-clipboard-manager = "2.3.2"
tauri-plugin-fs = "2.4.4"
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
log = { workspace = true }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
futures-util = "0.3"

[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"
ndk-context = "0.1"
# Android 音频:默认 oboe-rs(Android 官方推荐 + 成熟 Rust binding)。
# cpal 的 Android backend 在 0.16 主线未启用,留作"如果 oboe 不顺再评估"。
oboe = { version = "0.6", features = ["java-interface"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

> 注意 `tauri.conf.json` 的 bundle.identifier 必须是反 DNS 形式(`com.handy.mobile`),否则 Android 拒装。

### 5.2 `src-mobile-ui/package.json` (草案)

```json
{
  "name": "handy-mobile-ui",
  "private": true,
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-clipboard-manager": "^2",
    "react": "^19",
    "react-dom": "^19",
    "i18next": "^25",
    "react-i18next": "^16",
    "zustand": "^5"
  },
  "devDependencies": {
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "@vitejs/plugin-react": "^5",
    "typescript": "^5.6",
    "vite": "^6",
    "tailwindcss": "^4"
  }
}
```

版本号以桌面端 `package.json` 现状对齐,避免一个仓库两套 React。

## 6. 模块设计

### 6.1 `src-mobile/src/platform/android.rs`

**`AndroidAudioCapture`**:
- 实现 `handy_platform::AudioCapture`
- 默认用 `oboe-rs` (16kHz mono f32,`InputPreset::VoiceRecognition`)
- 如果 oboe 在 spike 阶段出意外(JNI 初始化 / `AAudioStream` 不可用),fallback 评估 `cpal` 的 Android backend 或直接走 `AudioRecord` JNI(本 spec 把 fallback 明列出来,plan 中追踪)
- `start()` 内部起一个 tokio task 把 stream 帧 push 到内部 `Mutex<Vec<f32>>`(累积模式,不走 mpsc — phase 2a 不流式)
- `stop()` 返回完整 buffer

**`AndroidTextOutput`**:
- 仅实现 `OutputMode::Clipboard`,通过 `tauri-plugin-clipboard-manager`
- `OutputMode::Typed` / `OutputMode::ImeCommit` 返回 `Err(anyhow!("not supported in phase 2a"))`

**`AndroidStorage`**:
- `models_dir()` → `app.path().app_data_dir()? / "models"`
- `db_path()` / `settings_path()` 在 phase 2a 不用,但 trait 要求实现 — 返回路径即可,不创建文件

### 6.2 `src-mobile/src/commands/transcription.rs`

```rust
#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String>;

#[tauri::command]
pub async fn cancel_recording(state: State<'_, AppState>) -> Result<(), String>;
```

事件(从 Rust 发到前端):

- `recording/level` — `{ rms: f32 }`,~10Hz,前端画一个简单 amplitude 条
- `model/download/progress` — `{ pct: u32, downloaded: u64, total: u64 }`
- `model/ready` — `{ name: String }`
- `transcription/error` — `{ message: String }`

### 6.3 `src-mobile/src/commands/model.rs`

- `download_default_model()`:从 `https://blob.handy.computer/ggml-tiny-q5_0.bin` 下载到 `storage.models_dir()`(whisper.cpp 用 ggml `.bin`,不是 `.gguf`;plan 第一步确认 URL 在 CDN 实际存在,否则改成 `ggml-tiny.bin` ~75MB 非量化版)
- `model_status()`:返回当前模型是否已下载、是否已加载、大小
- 文件完整性:下载完成后比对 size(简版),phase 2b 加 sha256

### 6.4 前端 `App.tsx` 草图

```
┌──────────────────────────────────┐
│            Handy                 │  <- 顶栏 (h-12)
├──────────────────────────────────┤
│                                  │
│        ┌──────────────┐          │
│        │              │          │  按住说话大圆按钮
│        │     🎙       │          │  按下 invoke('start_recording')
│        │   按住说话   │          │  松开 invoke('stop_recording')
│        │              │          │  录音中:边框脉冲 + RMS 条
│        └──────────────┘          │
│                                  │
│   ┌──────────────────────────┐   │
│   │ 转录结果会显示在这里...   │   │  TextDisplay,只读
│   │                          │   │
│   └──────────────────────────┘   │
│                                  │
│   [   复制   ]                   │  invoke clipboard-manager
│                                  │
└──────────────────────────────────┘
```

首次启动 `ModelDownloadGate` 全屏遮罩:
- 显示"首次启动,下载语音模型 (~39MB)"
- 一个进度条 + 取消按钮(取消即退出 App)
- 下载完成后自动 dismiss,进主页

### 6.5 `scripts/enter-android-env.ps1`

参考 `scripts/enter-build-env.ps1` 的 candidates 模式,职责:

1. 探测 `ANDROID_HOME`:候选 `%LOCALAPPDATA%\Android\Sdk`、`C:\Android\Sdk`、`D:\Android\Sdk`
2. 探测 `ANDROID_NDK_HOME`:`$ANDROID_HOME\ndk\*` 下最高版本
3. 探测 `JAVA_HOME`:Android Studio 内置 (`C:\Program Files\Android\Android Studio\jbr`) 优先,否则系统 JDK 17
4. `CARGO_TARGET_DIR_ANDROID` 走 `C:\handy-android-target`(避开 TEMP,沿用 phase 0+1 经验)
5. 加 `aarch64-linux-android` 与 `armv7-linux-androideabi` 到 rustup target
6. 把 NDK 的 `toolchains/llvm/prebuilt/windows-x86_64/bin` 加到 PATH

脚本失败时给清晰错误指向 `docs/mobile/android-build.md`。

## 7. CI 扩展

`.github/workflows/mobile-ci.yml` 加新 job(在 `android-cross-compile` 之后):

```yaml
android-debug-apk:
  name: Build debug APK
  needs: android-cross-compile
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-java@v4
      with:
        distribution: temurin
        java-version: "17"
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: aarch64-linux-android,armv7-linux-androideabi,i686-linux-android,x86_64-linux-android
    - name: Install cargo-tauri & cargo-ndk
      run: |
        cargo install tauri-cli --version "^2" --locked
        cargo install cargo-ndk --locked
    - uses: Swatinem/rust-cache@v2
      with:
        shared-key: android-apk
    - name: Resolve runner-bundled NDK
      run: |
        NDK="${ANDROID_NDK_LATEST_HOME:-${ANDROID_NDK_ROOT}}"
        echo "ANDROID_NDK_HOME=$NDK"  >> "$GITHUB_ENV"
        echo "ANDROID_NDK_ROOT=$NDK"  >> "$GITHUB_ENV"
        echo "NDK_HOME=$NDK"          >> "$GITHUB_ENV"
    - name: Install src-mobile-ui deps
      working-directory: src-mobile-ui
      run: |
        npm install -g bun
        bun install
    - name: cargo tauri android init (if not generated)
      working-directory: src-mobile
      run: |
        if [ ! -d gen/android ]; then
          cargo tauri android init --ci
        fi
    - name: Build debug APK
      working-directory: src-mobile
      env:
        ANDROID_PLATFORM: "24"
      run: cargo tauri android build --apk --debug
    - uses: actions/upload-artifact@v4
      with:
        name: handy-mobile-debug-apk
        path: src-mobile/gen/android/app/build/outputs/apk/universal/debug/*.apk
        if-no-files-found: error
```

> 路径以 `cargo tauri android init` 实际生成为准,可能是 `gen/android/app/build/outputs/apk/...` — plan 中第一步先跑通本地 init,确认路径再写到 yml。

## 8. 风险登记

| # | 风险 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|
| 1 | `transcribe-rs` 的 whisper-rs-sys (whisper.cpp) 在 NDK 下编译失败 | 中-高 | 高 | Plan 第一个任务就是这个 spike;失败后 fallback:(a) `whisper.cpp` cmake 加 `-DGGML_OPENMP=OFF`(NDK 不带 OpenMP);(b) 改用 `whisper-rs` 的纯 CPU pure-Rust 实现(若存在);(c) 切换默认引擎为 Parakeet(走 ONNX,但 ort-sys Android 又是另一个坑)— 三条都不行就上 spec 重新规划 |
| 2 | `oboe-rs` 在 NDK 27 下编译或 JNI 初始化失败 | 中 | 中 | Fallback 路径:(a) cpal Android backend;(b) 直接走 `AudioRecord` JNI 桥;plan 显式追踪 |
| 3 | Tauri Mobile 2.x 在 Windows 上 `cargo tauri android init` 工具链探测有 bug | 中 | 中 | `enter-android-env.ps1` 显式设置所有 env;Tauri 官方 mobile 文档同步参考 |
| 4 | APK 首次启动下载模型时网络环境差(国内 CDN) | 中 | 中 | `blob.handy.computer` 与桌面同源;phase 2a 不做镜像,允许用户手动复制模型到 `filesDir/models/`(`docs/mobile/android-build.md` 注明) |
| 5 | `MainActivity` Lifecycle 与 Tauri WebView 的 onPause/onResume 处理录音 stream 异常崩溃 | 中 | 中 | phase 2a 不要求后台 — onPause 时强制 stop 录音 + 释放 audio resource |
| 6 | `cargo tauri android build` 中间产物路径太长 / 含中文撞 Windows | 中 | 低 | 用 `CARGO_TARGET_DIR_ANDROID=C:\handy-android-target` 避开 |
| 7 | 桌面端 CI 因 workspace 多了 `src-mobile` member 出现新的 desktop-side 编译错误 | 低 | 低 | `src-mobile/src/platform/desktop_stub.rs` 提供 host `cargo check` 通过路径;`desktop-no-regression` job 已有 |

## 9. 验收标准 ("Done")

**必须全部满足**:

1. `.github/workflows/mobile-ci.yml` 全绿,包含新的 `android-debug-apk` job
2. CI artifacts 中可下载到 `handy-mobile-debug-apk.apk`
3. 该 APK 装到 **Android 10+** 真机或模拟器(`arm64-v8a` 或 `x86_64`)后:
   - 首次启动出现下载遮罩 → 自动下载 Whisper Tiny → 进入主页
   - 授予麦克风权限后,按住按钮录一段中文 / 英文 → 松开 → 屏幕在 3 秒内显示转录文本
   - 点"复制"按钮 → 切到记事本可粘出
4. **桌面端零回归**:
   - `cargo test -p handy-core --all-features` 全过
   - `desktop-no-regression` CI job 全平台过
   - 本地 `bun run tauri dev` 启动正常,录音→转录→历史→设置基本功能完好
5. 文档:
   - `docs/mobile/android-build.md` 写好,Windows 上新开发者能照着 1 小时内跑出 debug APK
   - `docs/mobile/README.md` 更新 phase 2a 完成状态表

**不要求**:任何形式的发布、签名、应用商店相关产物。

## 10. 未决问题 (Open Questions)

不阻塞 phase 2a 启动,但实施过程会决议:

1. **音频后端**:cpal Android backend vs `oboe-rs` — 第一周 spike 决定
2. **whisper.cpp NDK 编译参数**:OpenMP / NEON / FP16 是否在 NDK 下都能开 — 实测
3. **WebView 版本**:Android System WebView 在低版本 Android(10/11)行为差异 — 装到真机后看
4. **Whisper Tiny 在低端 Android 实际延迟**:目标 <3s,实测后定方向
5. **`tauri.conf.json` mobile 段 schema 与桌面 conf 是否共用一份**:phase 2a 用独立 `src-mobile/tauri.conf.json`,phase 2b 再评估

## 11. 与 phase 0+1 的衔接

| 资产 | phase 2a 怎么用 |
|------|----------------|
| `handy-core::text::*` | 转录后做繁简/标点/自定义词处理 |
| `handy-core::audio::resampler` | 如果 cpal 出 44.1kHz / 48kHz,重采样到 16kHz |
| `handy-core::audio::constants` | 采样率/帧大小常量 |
| `handy-core::audio::visualizer` | 暂不用(没有波形显示),phase 2c 用 |
| `handy-core::vad` | **不启用**,phase 2c 评估替代 |
| `handy-core::history` | **不引入**,phase 2b 用 |
| `handy-core::model::{EngineType, ModelInfo, DownloadProgress}` | 模型下载进度事件 payload 直接用 `DownloadProgress` |
| `handy-platform::AudioCapture` | Android 实现 |
| `handy-platform::TextOutput` | Android 实现(仅 Clipboard) |
| `handy-platform::AppStorage` | Android 实现 |
| `handy-platform::EventSink` | Tauri 实现(`TauriEventSink`,复用桌面侧已有模式) |
| `transcribe-rs` workspace 依赖 | 移动端 NDK 编译 |
| `tauri-runtime` git patch | 沿用,不动 |

## 12. 后续 phase 概览(非本 spec 范围,仅说明衔接)

- **phase 2b** — History/Models/Settings 三页 + sqlite + 多模型支持
- **phase 2c** — Android 前台录音服务 + VAD 替代实现 + 流式转录 + 锁屏/后台录音
- **phase 3** — IME (`InputMethodService` + bound Service 模型共享)
- **phase 4** — APK 签名 + Release build + 发布渠道(自主分发 / F-Droid / Play)
- iOS 阶段单独立项

---

## 13. 变更记录

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-05-27 | 初稿 | Claude Code + 唐海文 |
