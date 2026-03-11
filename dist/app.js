// web/app.js

// --- 1. THE MUSIC THEORY MATH ---
function frequencyToNoteData(frequency) {
    // 1. The universal array of Western note names
    const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

    // 2. Calculate the exact continuous MIDI note number
    const exactNote = 12 * Math.log2(frequency / 440.0) + 69;

    // 3. Round to the nearest whole integer to find the closest musical note
    const closestNoteIndex = Math.round(exactNote);

    // 4. Calculate the error (difference between exact and closest) in cents.
    // One semitone is 100 cents.
    const cents = (exactNote - closestNoteIndex) * 100;

    // 5. Map the integer index to a note name and octave
    // (MIDI note 0 is C-1)
    const noteString = noteNames[closestNoteIndex % 12];
    const octave = Math.floor(closestNoteIndex / 12) - 1;

    return {
        note: noteString,
        octave: octave,
        cents: cents
    };
}

// --- THE CANVAS RENDERING ENGINE ---
const canvas = document.getElementById('tuner-canvas');
const ctx = canvas.getContext('2d');

// State variables for smooth animation
let targetCents = 0;
let currentRenderedCents = 0;
let isAudioActive = false;

function drawTuner() {
    // 1. Clear the screen for the next frame
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const centerX = canvas.width / 2;
    const centerY = canvas.height * 0.9; 
    const radius = 280;

    // Angles in radians
    const topCenterAngle = -Math.PI / 2; 
    const sweepAngle = Math.PI / 6; 
    const leftEdgeAngle = topCenterAngle - sweepAngle;
    const rightEdgeAngle = topCenterAngle + sweepAngle;

    // 2. Draw the main 60-degree Arc (No Glow)
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, leftEdgeAngle, rightEdgeAngle);
    ctx.strokeStyle = '#333333'; 
    ctx.lineWidth = 4;
    ctx.stroke();

    // --- THE LOCK-IN LOGIC ---
    // We only trigger the glow if the audio is active AND the physical needle has 
    // animated to within 2 cents of the absolute center.
    const isLockedIn = isAudioActive && Math.abs(currentRenderedCents) < 2.0;

    // 3. Draw the Target Circle (The Pocket)
    const targetX = centerX + radius * Math.cos(topCenterAngle);
    const targetY = centerY + radius * Math.sin(topCenterAngle);
    
    if (isLockedIn) {
        // Ignite the neon bloom!
        ctx.shadowBlur = 15;
        ctx.shadowColor = '#4ade80'; // Neon green aura
        ctx.strokeStyle = '#4ade80'; // Turn the ring itself green
    } else {
        // Standard resting state
        ctx.shadowBlur = 0;
        ctx.strokeStyle = '#ffffff'; // Crisp white ring
    }

    ctx.beginPath();
    ctx.arc(targetX, targetY, 12, 0, Math.PI * 2);
    ctx.lineWidth = 3;
    ctx.stroke();
    
    // CRITICAL: Instantly turn the shadow off so it doesn't corrupt other drawings
    ctx.shadowBlur = 0; 

    // 4. Draw the Moving Needle
    if (isAudioActive) {
        const distanceToCenter = Math.abs(targetCents);
        const maxSpeed = 0.10;  
        const minSpeed = 0.01;  
        const dynamicLerp = minSpeed + (distanceToCenter / 50.0) * (maxSpeed - minSpeed);

        currentRenderedCents += (targetCents - currentRenderedCents) * dynamicLerp;
        const clampedCents = Math.max(-50, Math.min(50, currentRenderedCents));
        const currentAngle = topCenterAngle + (clampedCents / 50) * sweepAngle;

        const currentX = centerX + radius * Math.cos(currentAngle);
        const currentY = centerY + radius * Math.sin(currentAngle);

        // If locked in, make the solid needle glow too!
        if (isLockedIn) {
            ctx.shadowBlur = 20;
            ctx.shadowColor = '#4ade80';
        }

        ctx.beginPath();
        ctx.arc(currentX, currentY, 10, 0, Math.PI * 2);
        
        // Turn green if within 5 cents, otherwise yellow
        ctx.fillStyle = Math.abs(clampedCents) < 5.0 ? '#4ade80' : '#facc15';
        ctx.fill();
        
        ctx.shadowBlur = 0; // Reset again
    }

    requestAnimationFrame(drawTuner);
}

// Start the animation loop immediately
requestAnimationFrame(drawTuner);

// --- 2. THE AUDIO ENGINE BOOT SEQUENCE ---
async function startTuner() {
    const startBtn = document.getElementById('start-btn');
    startBtn.innerText = "Connecting..."; // Visual feedback!
    startBtn.disabled = true; // Prevent double-clicks

    try {
        const audioCtx = new (window.AudioContext || window.webkitAudioContext)();

        if (audioCtx.state === 'suspended') {
            await audioCtx.resume();
        }

        // 1. Fetch and compile the WebAssembly binary on the Main Thread.
        const response = await fetch('../tuner-dsp/pkg/tuner_dsp_bg.wasm');
        const wasmBytes = await response.arrayBuffer();
        const compiledWasmModule = await WebAssembly.compile(wasmBytes);

        // 2. Load the Worklet script (Must be type: 'module' for Wasm imports)
        await audioCtx.audioWorklet.addModule('pitch-processor.js', { type: 'module' });

        // 3. Create the Node
        const processorNode = new AudioWorkletNode(audioCtx, 'pitch-processor');

        // 4. Mail the compiled WebAssembly Module to the Worklet
        processorNode.port.postMessage({
            type: 'INIT_WASM',
            wasmModule: compiledWasmModule,
            sampleRate: audioCtx.sampleRate
        });

        // 5. Listen for the highly optimized pitch updates coming back
        processorNode.port.onmessage = (event) => {
            if (event.data.type === 'READY') {
                console.log("AudioWorklet Wasm Engine Armed and Ready!");
            } else if (event.data.type === 'PITCH') {
                const pitchDisplay = document.getElementById('pitch-display');
                
                if (event.data.value > 0) {
                    isAudioActive = true; // Wake up the canvas needle
                    
                    const tuningData = frequencyToNoteData(event.data.value);
                    targetCents = tuningData.cents; // Pass the target to the Canvas!

                    const centString = tuningData.cents > 0 
                        ? `+${tuningData.cents.toFixed(1)}` 
                        : tuningData.cents.toFixed(1);

                    pitchDisplay.innerText = `${tuningData.note}${tuningData.octave} (${centString}¢)`;
                    pitchDisplay.style.color = Math.abs(tuningData.cents) < 5.0 ? '#4ade80' : '#facc15';

                } else {
                    // Audio went silent
                    isAudioActive = false; // Hide the canvas needle
                    pitchDisplay.innerText = `--`;
                    pitchDisplay.style.color = '#888888'; 
                }
            }
        };

        // 6. Connect the physical microphone to the Worklet
        const stream = await navigator.mediaDevices.getUserMedia({ 
            audio: {
                echoCancellation: false,
                autoGainControl: false,
                noiseSuppression: false,
                channelCount: 1
            }    
        });

        startBtn.innerText = "Microphone Active";
        startBtn.style.backgroundColor = "#10b981"; // Turn the button green
        const microphoneSource = audioCtx.createMediaStreamSource(stream);
        microphoneSource.connect(processorNode);
        
    } catch(err) {
        // If the user clicks "Deny" or there is no mic plugged in
        console.error("Microphone access failed:", err);
        startBtn.innerText = "Microphone Denied";
        startBtn.style.backgroundColor = "#ef4444"; // Turn the button red
        startBtn.disabled = false; // Let them try again if they fixed their mic
    }
}

// --- 3. THE IGNITION WIRE ---
// Boot up the engine when the user clicks the physical button
document.getElementById('start-btn').addEventListener('click', () => {
    startTuner().catch(console.error);
});
