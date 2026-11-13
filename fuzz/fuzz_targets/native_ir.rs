#![no_main]

use libfuzzer_sys::fuzz_target;
use sovra::compiler::native_image;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    if let Ok(program) = native_image::decode(data) {
        let encoded = native_image::encode(&program).expect("decoded image must re-encode");
        let decoded = native_image::decode(&encoded).unwrap();
        assert_eq!(decoded, program);
        assert_eq!(native_image::encode(&decoded).unwrap(), encoded);
        assert!(native_image::decode(&encoded[..encoded.len() - 1]).is_err());
        let mut trailing = encoded;
        trailing.push(0);
        assert!(native_image::decode(&trailing).is_err());
    }
    // Never execute decoded programs or their host effects.
});
