// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use std::{fs::File, io::Read, path::Path};
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

/// Calculates entropy
pub fn calculate_entropy(data: &[u8]) -> f64 {
    let mut collector = [0u32; 256];
    data.iter().for_each(|byte| {
        collector[*byte as usize] += 1;
    });

    let len = data.len() as f64;
    let entropy = collector
        .iter()
        .filter(|x| **x != 0u32)
        .fold(0f64, |mut entropy, byte_count| {
            let symbol_probability = *byte_count as f64 / len;
            entropy += symbol_probability * symbol_probability.log2();

            entropy
        });

    -entropy
}

#[derive(Debug)]
pub struct Chunk {
    file: File,
}

impl Chunk {
    #[must_use]
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<Chunk> {
        let file = std::fs::File::open(path)?;
        Ok(Chunk { file })
    }

    pub fn calculate_entropy(&mut self) -> Result<f64> {
        let mut buffer = Vec::new();
        let _ = self.file.read_to_end(&mut buffer)?;
        Ok(calculate_entropy(&buffer))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("file io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("walkdir error: {0}")]
    WalkDirError(#[from] walkdir::Error),
}

#[cfg(test)]
mod tests {
    use crate::{calculate_entropy, Chunk};
    use std::{fs::File, io::Write};

    #[test]
    pub fn test_entropy() {
        let test_entropy = &[0x00, 0x00, 0x01, 0x01, 0x02];
        let shannon_entropy = calculate_entropy(test_entropy);

        assert!((shannon_entropy - 1.5219280948873621).abs() <= f64::EPSILON);
    }

    #[test]
    pub fn test_entropy_of_a_file() -> Result<(), crate::Error> {
        let tmp_dir = std::env::temp_dir();
        let path = tmp_dir.join("entropy-test");
        let mut file = File::create(path.clone())?;
        let _ = file.write(b"some random bytes")?;
        let mut chunk = Chunk::try_new(path)?;
        let entropy = chunk.calculate_entropy()?;
        assert!((entropy - 3.49922754713).abs() <= f64::EPSILON);

        Ok(())
    }
}
