use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use unreal_pak::{pakversion::PakVersion, PakFile, PakRecord};
use walkdir::WalkDir;

fn add_extension(path: &mut PathBuf, extension: &str) {
    match path.extension() {
        Some(existing_extension) => {
            let mut os_str = existing_extension.to_os_string();
            os_str.push(".");
            os_str.push(extension);
            path.set_extension(os_str);
        }
        None => {
            path.set_extension(extension);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=baked");

    let baked_dir = fs::read_dir("baked")?;
    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir).join("baked");

    fs::create_dir_all(&out_dir)?;

    for path in baked_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().unwrap().is_dir())
    {
        let path = path.path();
        let mut pak_path = out_dir.join(path.file_name().unwrap());
        add_extension(&mut pak_path, "pak");

        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&pak_path)?;

        let file = OpenOptions::new().append(true).open(pak_path)?;

        let mut pak = PakFile::writer(PakVersion::PakFileVersionFnameBasedCompressionMethod, &file);

        for entry in WalkDir::new(&path).into_iter().map(|e| e.unwrap()) {
            if entry.file_type().is_file() {
                let rel_path = entry.path().strip_prefix(&path).unwrap();
                let record_name = rel_path.to_str().unwrap().replace('\\', "/");

                let file_data = fs::read(entry.path()).unwrap();
                let record =
                    PakRecord::new(record_name, file_data, unreal_pak::CompressionMethod::Zlib)?;

                pak.add_record(record)?;
            }
        }

        pak.write()?;
    }

    Ok(())
}
