//! Populate ignored working corpora from the reviewed, checked-in source seeds.

use sovra::compiler::{ir, native_image, parser::Parser};
use std::{fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_corpus = root.join("corpus/source");
    let image_corpus = root.join("corpus/native_ir");
    fs::create_dir_all(&source_corpus)?;
    fs::create_dir_all(&image_corpus)?;
    for entry in fs::read_dir(root.join("seeds/source"))? {
        let entry = entry?;
        let source = fs::read_to_string(entry.path())?;
        fs::write(source_corpus.join(entry.file_name()), &source)?;
        if let Ok(program) = Parser::new().parse_source(&source) {
            if let Ok(lowered) = ir::lower_program(&program) {
                fs::write(
                    image_corpus.join(entry.file_name()),
                    native_image::encode(&lowered)?,
                )?;
            }
        }
    }
    Ok(())
}
