# Performance baseline — 2026-10-11

Host: Apple M4, 16 GiB unified RAM, macOS 26.6 arm64. Rust 1.99 release, whisper-rs 0.16 / vendored whisper.cpp, multilingual GGML Tiny (77,691,713 bytes, pinned checksum in NATIVE.md). Engine logs explicitly reported **Metal backend on Apple M4**. Five sequential inference trials per sample; the same model context stayed loaded, with a new decoder state each trial. Four warm trials are too few to establish a reliable production p95.

| Sample | Audio | SHA check | Model load | First inference | Warm p50 | Warm max / empirical p95 | Warm RTF | Max RSS |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| JFK English | 11.000 s | 151.76 ms | 128.50 ms | 125.02 ms | 84.77 ms | 109.12 ms | 0.00771 | 205.48 MiB |
| Italian synthetic Alice | 8.221 s | 170.31 ms | 94.79 ms | 239.73 ms | 144.72 ms | 155.85 ms | 0.01760 | 211.02 MiB |

RSS comes from `/usr/bin/time -l` for the entire benchmark process, not idle app memory. Peak memory footprint was 227.74 MiB (English), 233.28 MiB (Italian). These include input PCM/model/context and repeated inference; they exclude WebViews, capture, AX insertion and tray. Generated word counts were 22 English and 25 Italian; this is a functional transcription check, not measured word-error-rate or quality validation for real microphones. OS model/shader caches were warm; the first failed spike loaded the embedded Metal library in 5.8 seconds before an abort-callback defect stopped encoding, so these load numbers do not represent pristine GPU startup.

Raw successful measurements: `benchmarks/m4-tiny-jfk.json` and `benchmarks/m4-tiny-italian-synthetic.json`, with five individual timings each. No user speech/text is included. The earlier failed callback spike is recorded as a defect discovered/fixed, not hidden from the baseline.

## Reproduce

```sh
# The developer explicitly downloads the public sample and optional model.
mkdir -p /tmp/dicta-benchmark
curl --fail --location https://raw.githubusercontent.com/ggml-org/whisper.cpp/v1.8.4/samples/jfk.wav -o /tmp/dicta-benchmark/jfk.wav
sh scripts/benchmark.sh /tmp/dicta-benchmark /tmp/dicta-benchmark/jfk.wav en --download
# Later runs: omit --download; no network is used.
say -v Alice -o /tmp/dicta-benchmark/italiano.wav --file-format=WAVE --data-format=LEI16@16000 'Questa è una prova di dettatura in italiano. Le parole vengono trascritte sul computer e rimangono private. Oggi preparo una lista di cose da fare.'
sh scripts/benchmark.sh /tmp/dicta-benchmark /tmp/dicta-benchmark/italiano.wav it
node scripts/benchmark-csv.mjs --self-test
```

The benchmark expects PCM16 mono 16 kHz WAV, maximum five minutes. It hashes the model once, times loading separately, runs five trials and writes JSON, CSV and runtime counters. The JFK file is the sample supplied by whisper.cpp (US presidential public speech); see upstream MIT repository/sample provenance. The Italian sample uses the installed Apple system TTS voice for an internally generated phrase; neither voice assets nor generated audio are redistributed. Synthetic resampler tests generate tones in memory and need no third-party audio licensing.

## Remaining measurements and budgets

TODO: tray-ready/overlay-visible startup, shortcut→capture/overlay p50/p95/max, idle RSS/CPU with and without warm context, real microphone end→text and text→insertion, waveform CPU, battery use, cold model/shader start and repeated real-world audio/model sizes. These require the bundled app plus TCC permission/hardware observations. Target overlay p95 <150 ms remains a proposed target, not a passing measurement. No performance guarantee is made from a compile flag or this small sample set.

Structural idle budget: the worker blocks on a channel outside recording; microphone is closed, overlay hidden, level events stop, dashboard is lazy. Five-minute source-rate capture is preallocated/bounded; no PCM serialization into JS. A single loaded model remains resident until model change/delete/full reset/exit; model state is not reloaded per dictation. SQL synthetic smoke measurements are separately labeled in PERSISTENCE.md (10,000 rows, one run).
