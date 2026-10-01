// SPDX-License-Identifier: Apache-2.0

use buildah_ffi::{BuildRequest, Builder, Config, StorageDriver, startup};

fn main() -> Result<(), buildah_ffi::Error> {
    startup()?;
    let builder = Builder::open(Config {
        storage_driver: Some(StorageDriver::Vfs),
        ..Config::default()
    })?;
    let info = builder.build(BuildRequest::new("Dockerfile", ".").with_log(|record| {
        eprint!("{}", record.message);
    }))?;
    println!("image_id={}", info.image_id);
    if let Some(digest) = info.digest {
        println!("digest={digest}");
    }
    builder.shutdown()?;
    Ok(())
}
