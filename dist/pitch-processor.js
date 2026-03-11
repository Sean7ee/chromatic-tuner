// pitch-processor.js
import './text-polyfill.js';
// Import the WASM loader and your Rust wrapper struct
import init, { WasmTuner } from './pkg/tuner_dsp.js';

class PitchProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.tuner = null;

        // Listen for the initialization package from the Main Thread
        this.port.onmessage = async (event) => {
            if (event.data.type === 'INIT_WASM') {
                
                // Boot up the WebAssembly memory space using the compiled module!
                await init(event.data.wasmModule);
                
                // Instantiate your Rust DSP Core
                this.tuner = new WasmTuner(event.data.sampleRate, 0.1, 20.0);
                
                // Tell the Main Thread we are locked and loaded
                this.port.postMessage({ type: 'READY' });
            }
        };
    }

    process(inputs, outputs, parameters) {
        // If Wasm hasn't finished booting up yet, just pass the audio through
        if (!this.tuner) return true;

        const input = inputs[0];

        if (input.length > 0) {
            const channelData = input[0]; 

            // EXTREME PERFORMANCE: Pass the JS Float32Array directly into the 
            // Rust Ring Buffer without crossing any thread boundaries!
            const pitch = this.tuner.process_audio(channelData);

            // Only wake up the Main Thread UI if we have an actual pitch
            if (pitch > 0) {
                this.port.postMessage({ 
                    type: 'PITCH', 
                    value: pitch 
                });
            }
        }

        return true; 
    }
}

registerProcessor('pitch-processor', PitchProcessor);
