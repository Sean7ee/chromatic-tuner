<div align="center">

# 🎛️ Bare-Metal Chromatic Tuner
**A zero-latency, hardware-accelerated DSP engine built in Rust and WebAssembly.**

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)]()
[![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=webassembly&logoColor=white)]()
[![JavaScript](https://img.shields.io/badge/JavaScript-F7DF1E?style=for-the-badge&logo=javascript&logoColor=black)]()
[![Vercel](https://img.shields.io/badge/Vercel-000000?style=for-the-badge&logo=vercel&logoColor=white)]()

[View Live Demo](https://chromatic-tuner-pied.vercel.app/) • [Report Bug](https://github.com/Sean7ee/chromatic-tuner/issues)

</div>

---

## ⚡ Architecture Overview

This project is a systems-level approach to real-time audio processing in the browser. Standard web-based audio tools often suffer from Garbage Collection (GC) pauses and high-latency JavaScript execution. 

To solve this, the core digital signal processing (DSP) math is written completely in **Rust**, compiled to **WebAssembly (Wasm)**, and executed on a dedicated background thread using the **Web Audio API AudioWorklet**. The frontend is a vanilla JavaScript 60 FPS Canvas UI, completely decoupled from the audio processing thread.

### The Lock-Free Pipeline
By isolating the Wasm binary inside an AudioWorklet, the engine achieves a lock-free architecture. The browser's main thread handles the UI rendering, while the Rust binary safely manages its own memory and executes continuous matrix math without ever being interrupted by JavaScript's event loop.

---

## 🧠 Algorithmic Design

Building an accurate chromatic tuner requires bypassing the limitations of standard frequency-domain analysis.

### 1. The Resolution Problem (YIN vs. FFT)
A standard Fast Fourier Transform (FFT) is strictly bound by the sample rate and buffer size (Resolution = Fs / N). At a 48 kHz sample rate with a 2048-sample buffer, FFT resolution is ~23.4 Hz per bin—making it mathematically impossible to distinguish between closely tuned notes. 

Instead of an FFT, this engine implements the **YIN Algorithm** (time-domain autocorrelation). By analyzing the audio wave's period in the time domain, the engine mathematically sidesteps the FFT resolution limit entirely.

### 2. Sub-Sample Interpolation (0.1 Cent Accuracy)
Digital audio provides discrete integer samples. This introduces a slight issue when the period of a wave doesn't match up nicely with the sampling rate. For example, if a wave's true period falls between sample index 109 and 110, rounding to the nearest integer introduces a tuning error. This engine applies the **Parabolic Interpolation** method proposed by the authors to improve the YIN algorithm, fitting a mathematical parabola to calculate the exact fractional sub-sample minimum. This allows the tuner to track pitch down to microscopic fractions of a cent.

### 3. Ring Buffers
A ring buffer was implemented to provide a stream of fresh audio data to the tuning mechanism. This minimizes the amount of memory allocated to store the raw audio data for the mathematical calculations required by the YIN algorithm. 

### 4. The FFT Trick: $O(N^2)$ to $O(N \log N)$
A clever mathematical manipulation of the YIN Difference Function ($DF$) yields a massive computational optimization. The core expression is:

$$DF(\tau) = \sum_{i=t}^{t + W} (f(x_i) - f(x_i + \tau))^2$$

Expanding the binomial yields:

$$= \sum_{i=t}^{t + W} \left( f(x_i)^2 + f(x_i + \tau)^2 - 2f(x_i)f(x_i + \tau) \right)$$

We can also write this expression as:

$$= ACF(0, t) + ACF(0, t+ \tau) - 2ACF(\tau, t)$$

In the implementation, the first two energy terms are easily calculated with a cumulative sum of squares array based on the audio data array that was passed in. This operates in $\mathcal{O}(N)$ time using the pre-calculated array.

Calculating $\sum_{i=t}^{t + W} -2f(x_i)f(x_i + \tau)$ without the FFT trick requires $\mathcal{O}(W \cdot N)$ operations, where $N$ is the number of different lags we are testing. With the current implementation of the tuner, this comes down to exactly $\mathcal{O}(2400 \times 4096)$ operations at a $48\text{ kHz}$ sampling rate and a minimum frequency of 20Hz.

Since ACF is a correlation of two shifted functions, we can calculate this efficiently using the FFT. We rewrite the ACF as a convolution between the signal $x$ and its time-reversed signal $x'$ and apply the Convolution Theorem to precalculate the ACF values of each lag we want to calculate. This takes $\mathcal{O}(N \log_2 N)$ time, completely bypassing the naive $\mathcal{O}(W \cdot N)$ bottleneck.

## 🛠️ Local Development & CI/CD

This repository utilizes a custom whitelist-extraction build pipeline to bypass default Wasm compilation traps and prepare a pristine production distribution.

### Prerequisites
* [Rust & Cargo](https://www.rust-lang.org/tools/install)
* [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Building for Production
A shell script (`build.sh`) handles the Rust compilation, strips hidden ignore files, rewrites environment paths using `sed`, and stages the exact WebAssembly payload for deployment.

```bash
# 1. Make the script executable
chmod +x build.sh

# 2. Run the build pipeline
./build.sh
$ python -m http.server 8000
to start hosting the server

wasm-pack build --target web 
while in chromatic-tuner/ to recompile rust into .js
```

## 📚 Resources & Citations

This project was built using insights and architectures from the following academic and engineering resources:

### Papers & Documentation
* **[YIN, a fundamental frequency estimator for speech and music](https://asa.scitation.org/doi/10.1121/1.1458024)** * *De Cheveigné, A., & Kawahara, H. (2002).* The foundational mathematical paper detailing the Difference Function and Cumulative Mean Normalized Difference Function used in this DSP engine.
* **[Rust and WebAssembly Official Book](https://rustwasm.github.io/docs/book/)**
  * *The Rust Wasm Working Group.* Core architectural patterns for bridging memory safely between JavaScript and compiled Wasm binaries.
* **[Web Audio API: AudioWorklet](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet)**
  * *MDN Web Docs.* Documentation on bypassing the main thread to run high-performance, low-latency audio processing in the browser.

### Video References
* **[Detecting pitch automatically - The intuition behind the YIN pitch detection algorithm](https://www.youtube.com/watch?v=W585xR3bjLM)**
  * *V For Science.* Provided critical insights into the YIN pitch detection algorithm for implementation.
* **[The Fast Fourier Transform (FFT): Most Ingenious Algorithm Ever?](https://www.youtube.com/watch?v=h7apO7q16V0)**
  * *Reducible.* Provided insight into the implementation of the FFT algorithm. 
