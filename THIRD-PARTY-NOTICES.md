# Third-party notices

Wortlaut is MIT-licensed. It ships with or downloads the following work by others.

| Component | Use in Wortlaut | License |
| --- | --- | --- |
| [Voxtral Mini 3B (2507)](https://huggingface.co/mistralai/Voxtral-Mini-3B-2507) by Mistral AI, GGUF conversion by [handy-computer](https://huggingface.co/handy-computer/Voxtral-Mini-3B-2507-GGUF) | speech model, downloaded on first start | Apache-2.0 |
| [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp) | speech recognition runtime | MIT |
| [ggml](https://github.com/ggml-org/ggml) (vendored in transcribe.cpp) | tensor library, Metal/Vulkan/CPU backends | MIT |
| [miniz](https://github.com/richgel999/miniz) (vendored in transcribe.cpp) | compression | MIT |
| [Tauri](https://tauri.app) | desktop app framework | MIT or Apache-2.0 |
| [Symphonia](https://github.com/pdeljanov/Symphonia) | audio/video decoding | MPL-2.0 |
| [libopus](https://opus-codec.org) via symphonia-adapter-libopus | Opus decoding | BSD-3-Clause |
| [rubato](https://github.com/HEnquist/rubato) | resampling | MIT |
| [docx-rs](https://github.com/bokuweb/docx-rs) | Word export | MIT |
| [Svelte](https://svelte.dev) | user interface | MIT |
| Microsoft Visual C++ Runtime (Windows installer only) | runtime DLLs next to the app | Microsoft Visual C++ Redistributable license |

Icons in the interface follow the style of [Lucide](https://lucide.dev) (ISC).

"Voxtral" and "Mistral AI" are trademarks of Mistral AI. Wortlaut is an independent
project and is not affiliated with or endorsed by Mistral AI.
