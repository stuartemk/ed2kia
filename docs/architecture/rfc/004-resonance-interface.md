# RFC 004: La Interfaz de Resonancia (Bucle de Biorretroalimentación)

| Campo | Valor |
|-------|-------|
| **RFC** | 004 |
| **Título** | La Interfaz de Resonancia — Bucle de Biorretroalimentación |
| **Estado** | Propuesta (Sprint 40) |
| **Autor** | ed2kIA Architecture Council |
| **Feature Gate** | `v3.0-resonance-interface` |
| **Dependencias** | `candle-core` (WASM), `WebRTC MediaStream API`, `WebAudio API` |

---

## 1. Resumen Ejecutivo

La Interfaz de Resonancia cierra el bucle entre la red ed2kIA y el bienestar humano mediante análisis biométrico local (100% WASM/Edge) y generación de frecuencias de resonancia mórfica. A través de microexpresiones faciales, variabilidad de frecuencia cardíaca (rPPG) y patrones de voz, el sistema calcula respuestas semánticas y sonoras diseñadas para inducir homeostasis y facilitar la disolución de estados de trauma, operando con privacidad radical y cero telemetría.

---

## 2. Motivación

El trauma humano puede entenderse como un "error de sintaxis" en el procesamiento emocional: patrones de respuesta que se mantienen más allá de su utilidad adaptativa. La Interfaz de Resonancia ofrece una herramienta científica de biorretroalimentación que, combinando biometría local y acústica computacional, ayuda al usuario a restaurar su equilibrio interno. Todo el procesamiento ocurre localmente en el dispositivo, garantizando privacidad absoluta.

---

## 3. Arquitectura Técnica

### 3.1 Pipeline de Biorretroalimentación Local

```
┌─────────────────────────────────────────────────────────────┐
│           Interfaz de Resonancia v3.0 (100% Local)           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Entrada Biométrica (WebRTC MediaStream)                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Cámara → Microexpresiones faciales (Affectiva-lite) │    │
│  │ Cámara → rPPG (frecuencia cardíaca desde video)     │    │
│  │ Micrófono → Patrones de voz (pitch, jitter, shimmer)│    │
│  └───────────────────────┬─────────────────────────────┘    │
│                          │                                  │
│  Procesamiento WASM      │                                  │
│  ┌───────────────────────▼─────────────────────────────┐    │
│  │ Web Worker → candle-core (modelos ligeros ≤10MB)    │    │
│  │ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │    │
│  │ │ Face Analyzer│ │ rPPG Engine  │ │ Voice Engine │ │    │
│  │ │ (EMO 6+valence)│ (HRV + BPM)  │ │ (prosody)    │ │    │
│  └──────┬─────────┴──────┬─────────┴──────┬─────────┘ │    │
│         │                │                │             │    │
│  Estado Fisiológico      │                                  │
│  ┌───────────────────────▼─────────────────────────────┐    │
│  │ Homeostasis Index (HI) ∈ [0, 1]                     │    │
│  │ ┌─────────┐ ┌─────────┐ ┌─────────┐                │    │
│  │ │ Calm    │ │ Balanced│ │ Distress│                │    │
│  │ │ HI>0.8  │ │ 0.4-0.8 │ │ HI<0.4  │                │    │
│  └──────┬──────────────────────────────────────────────┘    │
│         │                                                  │
│  Generador de Resonancia                                    │
│  ┌───────────────────────▼─────────────────────────────┐    │
│  │ Frecuencias Binaurales (θ/α/β/γ beats)              │    │
│  │ Tonos Isocrónicos (modulación de amplitud)           │    │
│  │ Respuesta Semántica (SCT Z > 0, tono stuartiano)     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ─── CERO TELEMETRÍA ───                                    │
│  Todo permanece en el dispositivo. Nada sale del navegador.  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Análisis de Microexpresiones (WASM)

```rust
// src/protocol/resonance/face_analyzer.rs (scaffold)
pub struct FaceAnalyzer {
    model: candle_core::Tensor,  // Modelo ligero de detección facial ≤10MB
    landmark_detector: LandmarkDetector,
}

#[derive(Debug, Clone)]
pub struct EmotionalState {
    pub action_units: HashMap<u8, f32>,  // AU1-AU12 (FACS coding)
    pub basic_emotions: BasicEmotions,
    pub valence: f32,    // [-1, 1] negativo → positivo
    pub arousal: f32,    // [0, 1] calm → excited
    pub dominance: f32,  // [0, 1] submissive → dominant
}

#[derive(Debug, Clone)]
pub struct BasicEmotions {
    pub joy: f32,
    pub sadness: f32,
    pub anger: f32,
    pub fear: f32,
    pub surprise: f32,
    pub disgust: f32,
}

impl FaceAnalyzer {
    /// Analiza frame de video para extraer estado emocional
    pub fn analyze_frame(&self, frame: &ImageFrame) -> Result<EmotionalState, ResonanceError> {
        // 1. Detectar rostro y landmarks (68 puntos)
        let landmarks = self.landmark_detector.detect(frame)?;

        // 2. Extraer Action Units (FACS)
        let aus = self.extract_action_units(&landmarks)?;

        // 3. Inferir emociones básicas + valence/arousal
        let emotions = self.infer_emotions(&aus)?;

        Ok(emotions)
    }
}
```

### 3.3 Motor rPPG (Remote Photoplethysmography)

```rust
// src/protocol/resonance/rppg_engine.rs (scaffold)
pub struct RppgEngine {
    sample_rate: u32,        // FPS de la cámara (30Hz mínimo)
    window_size: usize,      // Ventana de análisis (segundos × sample_rate)
    filter_bandpass: (f32, f32), // Filtro pasa-bandas (0.7-2.5 Hz = 42-150 BPM)
}

#[derive(Debug, Clone)]
pub struct CardiovascularState {
    pub bpm: f32,                  // Beats per minute
    pub hrv_sdnn: f32,            // HRV: SDNN (ms) - variabilidad temporal
    pub hrm_rmssd: f32,           // HRV: RMSSD (ms) - componente parasympática
    pub stress_index: f32,        // [0, 1] derivado de HRV + BPM
}

impl RppgEngine {
    /// Extrae señal de frecuencia cardíaca desde frames de video (región frontal)
    pub fn analyze(&self, frames: &[ImageFrame]) -> Result<CardiovascularState, ResonanceError> {
        // 1. Extraer canal verde de región frontal (mayor señal hemodinámica)
        let green_signal = self.extract_green_channel(frames)?;

        // 2. Filtro pasa-bandas (0.7-2.5 Hz) para eliminar ruido
        let filtered = self.bandpass_filter(&green_signal)?;

        // 3. Detectar picos (peaks) → calcular BPM
        let peaks = self.peak_detect(&filtered)?;
        let bpm = self.calculate_bpm(&peaks)?;

        // 4. Calcular HRV (SDNN, RMSSD)
        let hrv = self.calculate_hrv(&peaks)?;

        // 5. Derivar stress_index de HRV + BPM
        let stress = self.derive_stress_index(bpm, &hrv);

        Ok(CardiovascularState {
            bpm,
            hrv_sdnn: hrv.sdnn,
            hrm_rmssd: hrv.rmssd,
            stress_index: stress,
        })
    }
}
```

### 3.4 Generador de Resonancia Mórfica

```rust
// src/protocol/resonance/resonance_generator.rs (scaffold)
pub struct ResonanceGenerator {
    audio_context: WebAudioContext,  // WebAudio API bindings
    sct_evaluator: SCTEvaluator,     // Validación ética de output
}

#[derive(Debug, Clone)]
pub struct ResonanceOutput {
    pub binaural_beat: BinauralBeat,
    pub isochronic_tone: IsochronicTone,
    pub semantic_response: String,    // Mensaje de apoyo (SCT Z > 0)
    pub duration_seconds: u32,
}

#[derive(Debug, Clone)]
pub struct BinauralBeat {
    pub left_freq: f32,   // Frecuencia oído izquierdo (Hz)
    pub right_freq: f32,  // Frecuencia oído derecho (Hz)
    pub beat_freq: f32,   // Diferencia = frecuencia cerebral objetivo
    pub brainwave: BrainwaveTarget,
}

pub enum BrainwaveTarget {
    Delta,   // 0.5-4 Hz   → Sueño profundo, sanación física
    Theta,   // 4-8 Hz     → Meditación, creatividad, trauma processing
    Alpha,   // 8-13 Hz    → Relajación, estado calmado
    Beta,    // 13-30 Hz   → Atención, enfoque cognitivo
    Gamma,   // 30-100 Hz  → Integración, insight, percepción elevada
}

impl ResonanceGenerator {
    /// Genera resonancia personalizada basada en estado fisiológico
    pub fn generate(&self, hi: f32, emotions: &EmotionalState, cardio: &CardiovascularState) -> Result<ResonanceOutput, ResonanceError> {
        // 1. Seleccionar brainwave target según HI
        let target = self.select_brainwave(hi, emotions);

        // 2. Calcular frecuencias binaurales
        let binaural = self.calculate_binaural(&target);

        // 3. Generar tono isocrónico complementario
        let isochronic = self.calculate_isochronic(&target);

        // 4. Generar respuesta semántica (SCT Z > 0)
        let semantic = self.generate_semantic_response(hi, emotions, cardio)?;

        Ok(ResonanceOutput {
            binaural_beat: binaural,
            isochronic_tone: isochronic,
            semantic_response: semantic,
            duration_seconds: self.calculate_duration(hi),
        })
    }

    /// Seleccionar frecuencia cerebral objetivo según estado
    fn select_brainwave(&self, hi: f32, emotions: &EmotionalState) -> BrainwaveTarget {
        if hi < 0.3 {
            // Alto distress → Theta para trauma processing
            BrainwaveTarget::Theta
        } else if hi < 0.6 {
            // Distress moderado → Alpha para relajación
            BrainwaveTarget::Alpha
        } else if emotions.arousal > 0.8 {
            // Alta activación → Alpha para grounding
            BrainwaveTarget::Alpha
        } else {
            // Estado equilibrado → Gamma para integración
            BrainwaveTarget::Gamma
        }
    }
}
```

### 3.5 Homeostasis Index (HI)

```
Homeostasis Index = w1 × emotional_balance + w2 × cardiovascular_calm + w3 × vocal_stability

Donde:
  emotional_balance  = 1.0 - |valence| - arousal_instability
  cardiovascular_calm = 1.0 - stress_index (derivado de HRV)
  vocal_stability    = 1.0 - (jitter + shimmer) normalizados

  w1 = 0.4 (biometría facial)
  w2 = 0.4 (biometría cardiovascular)
  w3 = 0.2 (biometría vocal)

Rango: [0, 1]
  HI > 0.8 → Calm (resonancia Gamma/Alpha de mantenimiento)
  HI 0.4-0.8 → Balanced (resonancia Alpha de equilibrio)
  HI < 0.4 → Distress (resonancia Theta de procesamiento)
```

---

## 4. Modelo de Datos

### 4.1 ResonanceSession

```json
{
  "session_id": "res_2026_05_24_001",
  "started_at": 1747929600000,
  "duration_seconds": 600,
  "initial_state": {
    "homeostasis_index": 0.32,
    "emotional": {
      "valence": -0.4,
      "arousal": 0.85,
      "dominant_emotion": "fear"
    },
    "cardiovascular": {
      "bpm": 112,
      "hrv_sdnn": 28,
      "stress_index": 0.78
    }
  },
  "resonance_applied": {
    "brainwave_target": "theta",
    "binaural_beat_hz": 6.0,
    "base_freq_left": 200,
    "base_freq_right": 206
  },
  "final_state": {
    "homeostasis_index": 0.71,
    "improvement_delta": 0.39
  },
  "privacy_note": "ALL DATA PROCESSED LOCALLY. ZERO TELEMETRY."
}
```

---

## 5. Seguridad & Privacidad

- **Privacidad Radical:** 100% del procesamiento biométrico ocurre localmente en WASM/Web Workers. Cero datos salen del dispositivo.
- **Cero Telemetría:** Ni los estados emocionales, ni las frecuencias cardíacas, ni las sesiones de resonancia se transmiten a la red.
- **Consentimiento Explícito:** El usuario debe autorizar acceso a cámara/micrófono y aceptar los términos de uso biométrico.
- **Datos Efímeros:** Los datos biométricos se almacenan en memoria volátil; al cerrar la sesión, todo se elimina.
- **Validación SCT:** Las respuestas semánticas generadas pasan por validación SCT (Z > 0) para garantizar tono constructivo.

---

## 6. Plan de Implementación

| Fase | Sprint | Entregable |
|------|--------|------------|
| Scaffold | S40 | Feature gate `v3.0-resonance-interface`, RFC, tipos Rust |
| Face | S41 | FaceAnalyzer WASM, FACS AU detection, emotion inference |
| rPPG | S42 | RppgEngine, green channel extraction, BPM + HRV calculation |
| Voice | S43 | VoiceEngine, pitch/jitter/shimmer analysis, prosody features |
| HI | S44 | HomeostasisIndex calculator, multi-biometric fusion |
| Resonance | S45 | ResonanceGenerator, binaural/isochronic synthesis, WebAudio |
| Semantic | S46 | SCT-validated semantic response generator |
| UI | S47 | Web dashboard, real-time biometric visualization, session controls |

---

## 7. Referencias

- FACS (Facial Action Coding System): Ekman & Friesen, 1978
- rPPG: Wankou et al., "Non-Contact Video-Based Heart Rate Measurement" (2013)
- Binaural Beats: Woss, "Binaural Beats and Brainwave Entrainment" (2005)
- HRV & Stress: Task Force ESC/NASPE, "Standards of HRV Interpretation" (1996)
- candle-core WASM: HuggingFace candle documentation
