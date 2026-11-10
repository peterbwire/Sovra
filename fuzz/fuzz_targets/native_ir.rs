#![no_main]

use libfuzzer_sys::fuzz_target;
use sovra::compiler::native_image;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    if let Ok(program) = native_image::decode(data) {
        let encoded = native_image::encode(&program).expect("decoded image must re-encode");
        assert_eq!(native_image::decode(&encoded).unwrap(), program);
    }
    // Never execute decoded programs or their host effects.
});
