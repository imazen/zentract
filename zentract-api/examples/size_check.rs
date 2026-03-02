use zentract_api::{InferenceEngine, TensorMeta};

fn main() {
    println!("zentract-api size check binary");
    println!("This binary does NOT link tract — it loads the plugin at runtime.");

    let meta = TensorMeta::f32_shape(&[1, 3, 320, 320]);
    println!("Example input shape: {:?}", meta);
    println!("Elements: {}", meta.num_elements());

    // Try to load the plugin (will fail if not present, that's fine)
    match InferenceEngine::load("libzentract_abi.so") {
        Ok(engine) => {
            println!("Plugin loaded successfully!");
            // Try loading a dummy model — will fail, just proving the API works
            let dummy = vec![0u8; 10];
            match engine.load_onnx(&dummy, meta) {
                Ok(_) => println!("Model loaded (unexpected)"),
                Err(e) => println!("Model load failed as expected: {e}"),
            }
        }
        Err(e) => println!("Plugin not found (expected): {e}"),
    }
}
