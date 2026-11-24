//! Framing validation precedes filesystem access and package compilation.
use sovra::compiler::project::packages::{
    read_source_overlay_request, MAX_OVERLAY_REQUEST_BYTES, SOURCE_OVERLAY_MAGIC,
};
fn frame(sources: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = SOURCE_OVERLAY_MAGIC.to_vec();
    bytes.extend(42u32.to_be_bytes());
    bytes.extend((sources.len() as u32).to_be_bytes());
    for (path, source) in sources {
        bytes.extend((path.len() as u32).to_be_bytes());
        bytes.extend((source.len() as u32).to_be_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend(*source);
    }
    bytes
}
fn rejected(bytes: &[u8]) {
    let error = read_source_overlay_request(bytes).unwrap_err();
    assert_eq!(error.items[0].code, "E4119");
    assert!(error.items[0].source_file.is_none());
}
#[test]
fn unicode_payload_roundtrips_and_all_truncations_trailing_bytes_and_bad_utf8_fail() {
    let path = std::env::temp_dir().join("entry λ😀.svr");
    let path = path.to_str().unwrap();
    let source = "// λ😀\r\nfn main() {}";
    let bytes = frame(&[(path, source.as_bytes())]);
    let request = read_source_overlay_request(bytes.as_slice()).unwrap();
    assert_eq!(request.request_id, 42);
    assert_eq!(request.sources[0].source, source);
    assert_eq!(request.sources[0].entry, std::path::Path::new(path));
    for at in 0..bytes.len() {
        rejected(&bytes[..at]);
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    rejected(&trailing);
    rejected(&frame(&[(path, &[0xff])]));
    let mut version = bytes.clone();
    version[6] = b'2';
    rejected(&version);
    let empty = read_source_overlay_request(frame(&[]).as_slice()).unwrap();
    assert!(empty.sources.is_empty());
}
#[test]
fn hostile_counts_lengths_and_path_forms_are_rejected_before_graph_resolution() {
    let path = std::env::temp_dir().join("entry.svr");
    let path = path.to_str().unwrap();
    for range in [12..16, 16..20, 20..24] {
        let mut bytes = frame(&[(path, b"")]);
        bytes[range].copy_from_slice(&u32::MAX.to_be_bytes());
        rejected(&bytes);
    }
    rejected(&frame(&vec![(path, &b""[..]); 33]));
    rejected(&frame(&[("relative.svr", b"")]));
    rejected(&frame(&[("", b"")]));
    let nul = format!("{path}\0");
    rejected(&frame(&[(&nul, b"")]));
    let huge = "p".repeat(4097);
    rejected(&frame(&[(&huge, b"")]));
}
#[test]
fn exact_total_payload_succeeds_and_per_buffer_total_and_transport_overflow_fail() {
    let path = std::env::temp_dir().join("entry.svr");
    let path = path.to_str().unwrap();
    let source = vec![b' '; 512 * 1024];
    let buffers = vec![(path, source.as_slice()); 8];
    assert_eq!(
        read_source_overlay_request(frame(&buffers).as_slice())
            .unwrap()
            .sources
            .len(),
        8
    );
    let mut excessive = buffers;
    excessive.push((path, b"x"));
    rejected(&frame(&excessive));
    let oversized = vec![b' '; 512 * 1024 + 1];
    rejected(&frame(&[(path, &oversized)]));
    struct Endless {
        read: usize,
    }
    impl std::io::Read for Endless {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            output.fill(b'x');
            self.read += output.len();
            Ok(output.len())
        }
    }
    let mut endless = Endless { read: 0 };
    assert_eq!(
        read_source_overlay_request(&mut endless).unwrap_err().items[0].code,
        "E4119"
    );
    assert_eq!(endless.read, MAX_OVERLAY_REQUEST_BYTES + 1);
}
