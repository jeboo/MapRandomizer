use std::path::Path;

use actix_web::{body::MessageBody, web::Bytes};
use anyhow::Result;
use log::info;
use object_store::{gcp::GoogleCloudStorageBuilder, ObjectStore, memory::InMemory, local::LocalFileSystem};

// Data needed to render the web page for a randomized seed and to use it to patch a ROM.
// The files `patch.ips`, `seed_footer.html`, and `seed_header.html` are mandatory,
// Extra files (e.g. spoiler logs/maps) can also be added which may be referenced
// in `seed_footer.html` and/or `seed_header`. We deliberately use a generic, minimally 
// structured format here, to ensure compatibility across Map Rando versions. Each of 
// these files is stored in durable object storage (currently Google Cloud Storage) 
// to allow players to share seeds.
pub struct Seed {
    pub name: String,
    pub files: Vec<SeedFile>,
}

pub struct SeedFile {
    filename: String,
    data: Vec<u8>,
}

impl SeedFile {
    pub fn new(filename: &str, data: Vec<u8>) -> Self {
        SeedFile {
            filename: filename.to_string(),
            data
        }
    }
}

pub struct SeedRepository {
    object_store: Box<dyn ObjectStore>,
    base_path: String,
}

impl SeedRepository {
    pub fn new(url: &str) -> Result<Self> {
        let object_store: Box<dyn ObjectStore> = if url.starts_with("gs:") {
            Box::new(
                    GoogleCloudStorageBuilder::from_env()
                        .with_url(url)
                        .build()?)
        } else if url == "mem" {
            Box::new(InMemory::new())
        } else if url.starts_with("file:") {
            let root = &url[5..];
            Box::new(LocalFileSystem::new_with_prefix(Path::new(root))?)
        } else {
            panic!("Unsupported seed repository type: {}", url);
        };
        Ok(Self {
            object_store,
            base_path: "seeds/".to_string(),
        })    
    }

    pub async fn get_file(&self, seed_name: &str, filename: &str) -> Result<Bytes> {
        let path = object_store::path::Path::parse(
            self.base_path.clone() + seed_name + "/" + filename + ".zstd",
        )?;
        let compressed_data = self.object_store.get(&path).await?.bytes().await?;
        let data = zstd::bulk::decompress(&compressed_data, 10_000_000)?;
        Ok(data.try_into_bytes().unwrap())
    }

    pub async fn put_file(&self, seed_name: &str, filename: String, data: Vec<u8>) -> Result<()> {
        let path = object_store::path::Path::parse(
            self.base_path.clone() + seed_name + "/" + filename.as_str() + ".zstd",
        )?;
        // info!("Compressing {}", filename);
        let compressed_data = zstd::bulk::compress(&data, 15)?;
        // info!("Writing {}", filename);
        self.object_store.put(&path, compressed_data.into()).await?;
        // info!("Done with {}", filename);
        Ok(())
    }

    pub async fn put_seed(&self, seed: Seed) -> Result<()> {
        info!("Storing seed");
        let mut futures = Vec::new();
        for file in seed.files {
            futures.push(self.put_file(&seed.name, file.filename, file.data));
        }
        let results = futures::future::join_all(futures).await;
        for result in results {
            result?;
        }
        info!("Done storing seed");
        Ok(())
    }
}
