# Mobile & Browser Expansion Architecture — ed2kIA v1.8

This document defines the architecture for expanding ed2kIA to mobile devices and web browsers via WebAssembly (WASM), enabling background compute services, browser extensions, and mobile-native participation in the decentralized AI verification network.

## 1. Strategic Objectives

| Objective | Target | Impact |
|-----------|--------|--------|
| **10x Node Count** | Q2 2027 | Mobile users contribute idle compute |
| **Zero-Install Participation** | Q1 2027 | Browser extension + PWA |
| **Background Verification** | Q1 2027 | Passive ZKP verification on mobile |
| **Cross-Platform Identity** | Q2 2027 | Unified Ed25519 identity across devices |

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                    ed2kIA Network Core                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐      │
│  │ Desktop     │  │ Server      │  │ Edge Nodes          │      │
│  │ Nodes (Rust)│  │ Nodes       │  │ (existing infra)    │      │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘      │
│         │                │                     │                 │
│         ▼                ▼                     ▼                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              P2P Federation Layer                       │    │
│  │         (existing cross-model scaling)                  │    │
│  └──────────────────────┬──────────────────────────────────┘    │
└─────────────────────────┼───────────────────────────────────────┘
                          │ WASM-Compatible Protocol
                          ▼
┌──────────────────────────────────────────────────────────────────┐
│                   WASM Runtime Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐      │
│  │ Browser     │  │ Mobile      │  │ Tauri/Electron      │      │
│  │ Extension   │  │ (wasmtime)  │  │ Desktop Apps        │      │
│  └─────────────┘  └─────────────┘  └─────────────────────┘      │
└──────────────────────────────────────────────────────────────────┘
```

## 3. WebAssembly Strategy

### 3.1 WASM Target Selection

| Target | Runtime | Use Case | Priority |
|--------|---------|----------|----------|
| **Web Browser** | WASI/WebAssembly | Browser extension + PWA | P0 |
| **Mobile (iOS/Android)** | wasmtime | Background service | P0 |
| **Desktop (Tauri)** | wasmtime | Lightweight desktop client | P1 |
| **Edge (Cloudflare Workers)** | WASI | Edge verification nodes | P2 |

### 3.2 Rust → WASM Compilation

```toml
# Cargo.toml WASM targets
[lib]
crate-type = ["cdylib", "rlib"]

# Build commands
# wasm-pack build --target web          # Browser
# wasm-pack build --target nodejs       # Node.js
# cargo build --target wasm32-wasip1    # WASI (wasmtime)
```

### 3.3 WASM Module Boundaries

```
ed2kIA-wasm/
├── core/              # Shared WASM core (verification, crypto)
├── browser/           # Browser extension adapter
├── mobile/            # Mobile background service adapter
├── tauri/             # Tauri desktop adapter
└── cli/               # WASI CLI tool
```

### 3.4 Size Constraints

| Target | Max WASM Size | Budget |
|--------|--------------|--------|
| Browser Extension | < 2MB | ~40% of 5MB Chrome limit |
| Mobile Background | < 5MB | Conservative for battery |
| Tauri Desktop | < 10MB | Generous for desktop |
| Edge Workers | < 1MB | Cloudflare limit |

## 4. Browser Extension

### 4.1 Manifest V3 Architecture

```json
{
  "manifest_version": 3,
  "name": "ed2kIA Verifier",
  "version": "1.8.0",
  "description": "Decentralized AI verification - contribute idle browser compute",
  "permissions": [
    "background",
    "storage",
    "idle"
  ],
  "background": {
    "service_worker": "background.js",
    "type": "module"
  },
  "action": {
    "default_popup": "popup.html",
    "default_icon": "icon-48.png"
  },
  "icons": {
    "48": "icon-48.png",
    "128": "icon-128.png"
  },
  "web_accessible_resources": [{
    "resources": ["wasm/ed2kIA-core.wasm"],
    "matches": ["<all_urls>"]
  }]
}
```

### 4.2 Background Service Worker

```javascript
// background.js
import { instantiateEd2kIA } from './wasm/ed2kIA-core.js';

let ed2kIA = null;
let isContributing = false;

// Initialize WASM module
chrome.runtime.onInstalled.addListener(async () => {
  ed2kIA = await instantiateEd2kIA();
  // Generate or restore Ed25519 identity
  const identity = await chrome.storage.local.get('identity');
  if (!identity) {
    const newIdentity = ed2kIA.generateIdentity();
    await chrome.storage.local.set({ identity: newIdentity });
  }
});

// Start contribution when browser is idle
chrome.idle.setDetectionInterval(60);
chrome.idle.onStateChanged.addListener(async (state) => {
  if (state === 'idle' && !isContributing) {
    isContributing = true;
    startContribution();
  } else if (state !== 'idle' && isContributing) {
    isContributing = false;
    stopContribution();
  }
});

async function startContribution() {
  // Fetch verification tasks from federation
  const tasks = await fetchTasks();
  for (const task of tasks) {
    const result = ed2kIA.verifyProof(task);
    await submitResult(result);
  }
}
```

### 4.3 User Interface (Popup)

```
┌─────────────────────────────────┐
│  ed2kIA Verifier v1.8.0        │
├─────────────────────────────────┤
│  Status: ● Active               │
│                                 │
│  Today's Contributions:         │
│  ┌───────────────────────────┐  │
│  │ Proofs Verified: 1,247   │  │
│  │ Reputation Earned: +42   │  │
│  │ Streak: 15 days 🔥       │  │
│  └───────────────────────────┘  │
│                                 │
│  Settings:                      │
│  ☑ Contribute when idle         │
│  ☐ Max CPU: 25%                 │
│  ☑ Show notifications           │
│                                 │
│  [View Leaderboard] [Settings]  │
└─────────────────────────────────┘
```

### 4.4 Resource Limits

| Resource | Limit | Rationale |
|----------|-------|-----------|
| CPU Usage | Max 25% single core | Don't impact user experience |
| Memory | Max 128MB | Browser tab limit |
| Network | Throttled to 1MB/s | Don't consume user bandwidth |
| Battery | Pause when < 20% | Preserve device battery |

## 5. Mobile Background Service

### 5.1 Platform Strategy

| Platform | Approach | Technology |
|----------|----------|------------|
| **Android** | Foreground service + WASM | wasmtime + Kotlin |
| **iOS** | Background tasks + WASM | wasmtime + Swift |
| **Cross-Platform** | Flutter + WASM plugin | Flutter + wasmtime |

### 5.2 Android Implementation

```kotlin
// Ed2kIAService.kt
class Ed2kIAService : Service() {
    private var wasmtime: WasmtimeEngine? = null
    private var isRunning = false

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("ed2kIA Verifier")
            .setContentText("Verifying AI proofs...")
            .setSmallIcon(R.drawable.ic_ed2kIA)
            .build()

        startForeground(1, notification)
        startVerification()
        return START_STICKY
    }

    private fun startVerification() {
        isRunning = true
        wasmtime = WasmtimeEngine.load("ed2kIA-core.wasm")

        lifecycleScope.launch {
            while (isRunning) {
                val tasks = fetchTasks()
                for (task in tasks) {
                    val result = wasmtime?.invoke("verify_proof", task)
                    submitResult(result)
                    updateNotification()
                }
                // Sleep to preserve battery
                delay(60_000L)
            }
        }
    }

    override fun onDestroy() {
        isRunning = false
        wasmtime?.destroy()
        super.onDestroy()
    }
}
```

### 5.3 iOS Implementation

```swift
// Ed2kIATaskHandler.swift
import BackgroundTasks
import Wasmtime

class Ed2kIATaskHandler: BGProcessingTask {
    private var engine: WasmtimeEngine?

    func perform(_ task: BGProcessingTask) {
        engine = WasmtimeEngine.load("ed2kIA-core.wasm")

        Task {
            while !task.isCancelled {
                let tasks = await fetchTasks()
                for task in tasks {
                    let result = engine?.invoke("verify_proof", parameters: task)
                    await submitResult(result)
                }
                // Respect system battery management
                try await Task.sleep(nanoseconds: 60_000_000_000)
            }
            task.setTaskCompleted(success: true)
        }
    }
}

// Registration in AppDelegate
func registerBackgroundTasks() {
    BGProcessingTaskRequest.register(for: "io.ed2kIA.verify") { request in
        request.requiresExternalPower = false  // Allow on battery
        request.requiresNetworkConnectivity = true
        request.maximumExecutionCount = 4  // Max 4 runs per day
    }
}
```

### 5.4 Battery & Data Optimization

| Optimization | Implementation |
|--------------|---------------|
| **Adaptive Scheduling** | Reduce frequency when battery < 30% |
| **WiFi-Only Mode** | Optional: only contribute on WiFi |
| **Thermal Throttling** | Pause when device temperature > 40°C |
| **Data Compression** | LZ4 compression for proof payloads |
| **Batch Submission** | Accumulate results, submit every 5 minutes |

## 6. Tauri Desktop App

### 6.1 Why Tauri

| Feature | Tauri | Electron | Benefit |
|---------|-------|----------|---------|
| Binary Size | ~3MB | ~80MB | 96% smaller |
| Memory Usage | ~100MB | ~300MB | 67% less RAM |
| WASM Support | Native | Limited | Better WASM integration |
| Security | System-level | Chromium sandbox | Smaller attack surface |

### 6.2 Tauri Configuration

```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png"]
  },
  "app": {
    "windows": [
      {
        "title": "ed2kIA Desktop",
        "width": 800,
        "height": 600,
        "resizable": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'"
    }
  },
  "wasm": {
    "modulePath": "ed2kIA-core.wasm",
    "wasi": true
  }
}
```

### 6.3 System Tray Integration

```rust
// src-tauri/src/main.rs
use tauri::SystemTray;

fn main() {
    tauri::Builder::default()
        .system_tray(SystemTray::new())
        .on_system_tray_event(|app, event| match event {
            tauri::SystemTrayEvent::LeftClick { .. } => {
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
            }
            tauri::SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "start" => start_contribution(app),
                    "stop" => stop_contribution(app),
                    "quit" => std::process::exit(0),
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ed2kIA");
}
```

## 7. Cross-Platform Identity

### 7.1 Unified Ed25519 Identity

Same Ed25519 key pair across all platforms:

```
┌─────────────────────────────────────────────┐
│         Contributor Identity                 │
│  ┌───────────────────────────────────────┐  │
│  │ Ed25519 Private Key (encrypted)       │  │
│  │ - Stored in OS Keychain/Keystore      │  │
│  │ - Never transmitted to network        │  │
│  │ - Used only for signing proofs        │  │
│  └───────────────────────────────────────┘  │
│  ┌───────────────────────────────────────┐  │
│  │ Ed25519 Public Key (shared)           │  │
│  │ - Identifies contributor across       │  │
│  │   browser, mobile, desktop            │  │
│  │ - Reputation aggregated across        │  │
│  │   all devices                         │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### 7.2 Key Storage by Platform

| Platform | Storage | Encryption |
|----------|---------|------------|
| Browser Extension | chrome.storage.local (encrypted) | Web Crypto API |
| Android | Android Keystore | Hardware-backed |
| iOS | iOS Keychain | Secure Enclave |
| Tauri Desktop | OS Keychain (Keytar) | OS-native |

### 7.3 Key Recovery

Social recovery mechanism:
1. **Mnemonic Backup**: 12-word BIP39 mnemonic for key derivation
2. **Social Recovery**: 3-of-5 trusted contacts can initiate recovery
3. **Cloud Backup**: Encrypted key backup to user's cloud storage (optional)

## 8. Protocol Adaptations for WASM

### 8.1 WASM-Compatible Crypto

| Operation | Native Rust | WASM Alternative |
|-----------|------------|------------------|
| Ed25519 Sign | `ed25519-dalek` | `ed25519-dalek` (WASM-compatible) |
| SHA-256 | `sha2` | `sha2` (WASM-compatible) |
| LZ4 Compression | `lz4` | `lz4_flex` (no-std) |
| Merkle Tree | Custom | Custom (WASM-compatible) |

### 8.2 Network Communication

WASM modules use HTTP/WebSocket for federation communication:

```rust
// WASM-compatible HTTP client
#[wasm_bindgen]
pub async fn fetch_verification_tasks() -> Result<Vec<Task>, JsValue> {
    let response = web_sys::fetch("https://federation.ed2kIA.org/api/v1/tasks")
        .await?
        .json()
        .await?;
    Ok(response)
}

#[wasm_bindgen]
pub async fn submit_proof_result(proof: &ProofResult) -> Result<(), JsValue> {
    let _ = web_sys::fetch("https://federation.ed2kIA.org/api/v1/submit")
        .with_method("POST")
        .with_body(proof.to_json())
        .await?;
    Ok(())
}
```

### 8.3 Graceful Degradation

| Capability | Full Node | WASM Node | Fallback |
|------------|-----------|-----------|----------|
| Full SAE Loading | ✅ | ❌ | Skip heavy models |
| ZKP Verification | ✅ | ✅ | Core proofs only |
| Tensor Computation | ✅ | ⚠️ | Limited precision |
| P2P Networking | ✅ | ❌ | HTTP API only |
| Reputation Tracking | ✅ | ✅ | Full support |

## 9. Development Roadmap

### Phase 1: WASM Core (v1.8 Sprint 1)
- [ ] Extract WASM-compatible core module
- [ ] `wasm-pack build` CI pipeline
- [ ] Browser extension MVP (Chrome/Firefox)
- [ ] Basic verification tasks

### Phase 2: Mobile Services (v1.8 Sprint 2)
- [ ] Android foreground service
- [ ] iOS background tasks
- [ ] Battery optimization
- [ ] Cross-platform identity sync

### Phase 3: Desktop App (v1.8 Sprint 3)
- [ ] Tauri desktop shell
- [ ] System tray integration
- [ ] Auto-update mechanism
- [ ] Multi-device reputation aggregation

### Phase 4: Advanced Features (v1.9)
- [ ] Progressive Web App (PWA)
- [ ] Edge worker deployment (Cloudflare)
- [ ] Mobile app store listings
- [ ] Contributor onboarding wizard

## 10. Testing Strategy

### 10.1 WASM Testing

```bash
# Unit tests in WASM
wasm-pack test --headless --chrome

# Integration tests
cargo test --target wasm32-wasip1

# Performance benchmarks
wasm-bench --target web
```

### 10.2 Mobile Testing

| Test Type | Tool | Platform |
|-----------|------|----------|
| Unit Tests | XCTest / JUnit | iOS / Android |
| Integration | XCUITest / Espresso | iOS / Android |
| Battery Impact | Battery Historian | Android |
| Thermal Testing | Xcode Instruments | iOS |

### 10.3 Browser Extension Testing

| Test Type | Tool | Browser |
|-----------|------|---------|
| Unit Tests | Jest + jsdom | All |
| Integration | WebExt Test Runner | Firefox |
| E2E | Playwright | Chrome/Firefox/Safari |
| Manifest Validation | Web Store Validator | Chrome |

## 11. Security Considerations

### 11.1 WASM Security

| Threat | Mitigation |
|--------|-----------|
| WASM Memory Corruption | Bounds checking + safe Rust |
| Private Key Exposure | OS keychain storage + never transmit |
| Man-in-the-Middle | TLS 1.3 + certificate pinning |
| Malicious Tasks | Input validation + sandboxing |

### 11.2 Browser Extension Security

- **Content Security Policy**: Restrict script execution
- **Minimal Permissions**: Only `background`, `storage`, `idle`
- **No Remote Code Execution**: All WASM bundled (no dynamic fetch)
- **Audit Trail**: All actions logged to extension console

### 11.3 Mobile Security

- **Certificate Pinning**: Prevent MITM on mobile networks
- **Key Isolation**: Hardware-backed key storage
- **Jailbreak/Root Detection**: Warn users of compromised devices
- **Data Encryption**: AES-256 for local cache

## 12. Metrics & Monitoring

### 12.1 Key Metrics

| Metric | Target | Alert |
|--------|--------|-------|
| Active browser extension users | > 10,000 | < 1,000 |
| Active mobile contributors | > 5,000 | < 500 |
| WASM verification success rate | > 99% | < 95% |
| Average battery impact (mobile) | < 5%/hour | > 10%/hour |
| Extension crash rate | < 0.1% | > 1% |

### 12.2 Platform Distribution Target

| Platform | Q1 2027 | Q2 2027 | Q3 2027 |
|----------|---------|---------|---------|
| Desktop (Rust) | 60% | 45% | 35% |
| Browser Extension | 20% | 30% | 35% |
| Mobile | 15% | 20% | 25% |
| Tauri Desktop | 5% | 5% | 5% |

---

*This document is a living architecture spec. Update with each sprint iteration.*
