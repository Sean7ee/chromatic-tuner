// web/text-polyfill.js

// The AudioWorklet uses 'globalThis' as its top-level object.
if (typeof globalThis.TextDecoder === 'undefined') {
    globalThis.TextDecoder = class {
        decode(buffer) {
            // A bare-minimum ASCII decoder just to keep Wasm-bindgen happy
            let str = '';
            const arr = new Uint8Array(buffer);
            for (let i = 0; i < arr.length; i++) {
                str += String.fromCharCode(arr[i]);
            }
            return str;
        }
    };
}

if (typeof globalThis.TextEncoder === 'undefined') {
    globalThis.TextEncoder = class {
        encode(str) {
            const arr = new Uint8Array(str.length);
            for (let i = 0; i < str.length; i++) {
                arr[i] = str.charCodeAt(i);
            }
            return arr;
        }
    };
}
