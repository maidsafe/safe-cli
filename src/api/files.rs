// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::xorurl::SafeContentType;
use super::{Safe, XorUrl, XorUrlEncoder};
use chrono::{SecondsFormat, Utc};
use common_path::common_path_all;
use log::{debug, info};
use relative_path::RelativePath;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

// Each FileItem contains file metadata and the link to the file's ImmutableData XOR-URL
pub type FileItem = BTreeMap<String, String>;

// To use for mapping files names (with path in a flattened hierarchy) to FileItems
pub type FilesMap = BTreeMap<String, FileItem>;

// List of files uploaded with details if they were added, updated or deleted from FilesContainer
type ProcessedFiles = BTreeMap<String, (String, String)>;

// Type tag to use for the FilesContainer stored on AppendOnlyData
const FILES_CONTAINER_TYPE_TAG: u64 = 10_100;
// Informative string of the SAFE native data type behind a FilesContainer
const FILES_CONTAINER_NATIVE_TYPE: &str = "AppendOnlyData";

const FILE_ADDED_SIGN: &str = "+";
const FILE_UPDATED_SIGN: &str = "*";
const FILE_DELETED_SIGN: &str = "-";
const FILE_ERROR_SIGN: &str = "E";

const ERROR_MSG_NO_FILES_CONTAINER_FOUND: &str = "No FilesContainer found at this address";

const FILES_MAP_PREDICATE_LINK: &str = "link";
const FILES_MAP_PREDICATE_TYPE: &str = "type";
const FILES_MAP_PREDICATE_SIZE: &str = "size";
const FILES_MAP_PREDICATE_MODIFIED: &str = "modified";
const FILES_MAP_PREDICATE_CREATED: &str = "created";

#[allow(dead_code)]
impl Safe {
    /// # Create a FilesContaier.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", true, None).unwrap();
    /// assert!(xorurl.contains("safe://"))
    /// ```
    pub fn files_container_create(
        &mut self,
        location: &str,
        recursive: bool,
        set_root: Option<String>,
    ) -> Result<(XorUrl, ProcessedFiles, FilesMap), String> {
        // TODO: Enable source for funds / ownership
        // Warn about ownership?
        let processed_files = file_system_dir_walk(self, location, recursive, true)?;

        // The FilesContainer is created as a AppendOnlyData with a single entry containing the
        // timestamp as the entry's key, and the serialised FilesMap as the entry's value
        // TODO: use RDF format
        let root_path = get_root_path(location, set_root)?;
        let files_map = files_map_create(&processed_files, root_path)?;
        let serialised_files_map = serde_json::to_string(&files_map)
            .map_err(|err| format!("Couldn't serialise the FilesMap generated: {:?}", err))?;
        let now = gen_timestamp();
        let files_container_data = vec![(
            now.into_bytes().to_vec(),
            serialised_files_map.as_bytes().to_vec(),
        )];

        // Store the FilesContainer in a Published AppendOnlyData
        let xorname = self.safe_app.put_seq_appendable_data(
            files_container_data,
            None,
            FILES_CONTAINER_TYPE_TAG,
            None,
        )?;

        let xorurl = XorUrlEncoder::encode(
            xorname,
            FILES_CONTAINER_TYPE_TAG,
            SafeContentType::FilesContainer,
            &self.xorurl_base,
        )?;

        Ok((xorurl, processed_files, files_map))
    }

    /// # Fetch an existing FilesContaier.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", true, None).unwrap();
    /// let (version, files_map, native_type) = safe.files_container_get_latest(&xorurl).unwrap();
    /// println!("FilesContainer fetched is at version: {}", version);
    /// println!("FilesContainer is stored on a {} data type", native_type);
    /// println!("FilesMap of latest fetched version is: {:?}", files_map);
    /// ```
    pub fn files_container_get_latest(
        &self,
        xorurl: &str,
    ) -> Result<(u64, FilesMap, String), String> {
        let xorurl_encoder = XorUrlEncoder::from_url(xorurl)?;

        match self
            .safe_app
            .get_latest_seq_appendable_data(xorurl_encoder.xorname(), FILES_CONTAINER_TYPE_TAG)
        {
            Ok((version, (_key, value))) => {
                // TODO: use RDF format and deserialise it
                let files_map = serde_json::from_str(&String::from_utf8_lossy(&value.as_slice()))
                    .map_err(|err| {
                    format!(
                        "Couldn't deserialise the FilesMap stored in the FilesContainer: {:?}",
                        err
                    )
                })?;
                Ok((version, files_map, FILES_CONTAINER_NATIVE_TYPE.to_string()))
            }
            Err("SeqAppendOnlyDataEmpty") => Ok((
                0,
                FilesMap::default(),
                FILES_CONTAINER_NATIVE_TYPE.to_string(),
            )),
            Err("SeqAppendOnlyDataNotFound") => Err(ERROR_MSG_NO_FILES_CONTAINER_FOUND.to_string()),
            Err(err) => Err(format!("Failed to get current version: {}", err)),
        }
    }

    /// # Sync up local folder with the content on a FilesContaier.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", true, None).unwrap();
    /// let (version, new_processed_files, new_files_map) = safe.files_container_sync("tests/testfolder", &xorurl, true, None, false).unwrap();
    /// println!("FilesContainer fetched is at version: {}", version);
    /// println!("The local files that were synced up are: {:?}", new_processed_files);
    /// println!("The FilesMap of the updated FilesContainer now is: {:?}", new_files_map);
    /// ```
    pub fn files_container_sync(
        &mut self,
        location: &str,
        xorurl: &str,
        recursive: bool,
        set_root: Option<String>,
        delete: bool,
    ) -> Result<(u64, ProcessedFiles, FilesMap), String> {
        let (mut version, current_files_map, _): (u64, FilesMap, String) =
            self.files_container_get_latest(xorurl)?;

        let root_path = get_root_path(location, set_root)?;
        let processed_files = file_system_dir_walk(self, location, recursive, false)?;
        let (processed_files, new_files_map, success_count): (ProcessedFiles, FilesMap, u64) =
            files_map_sync(self, current_files_map, processed_files, root_path, delete)?;

        if success_count > 0 {
            // The FilesContainer is updated adding an entry containing the timestamp as the
            // entry's key, and the serialised new version of the FilesMap as the entry's value
            let serialised_files_map = serde_json::to_string(&new_files_map)
                .map_err(|err| format!("Couldn't serialise the FilesMap generated: {:?}", err))?;
            let now = gen_timestamp();
            let files_container_data = vec![(
                now.into_bytes().to_vec(),
                serialised_files_map.as_bytes().to_vec(),
            )];

            let xorurl_encoder = XorUrlEncoder::from_url(xorurl)?;
            let xorname = xorurl_encoder.xorname();
            let type_tag = xorurl_encoder.type_tag();

            let current_version = match self
                .safe_app
                .get_current_seq_appendable_data_version(xorname, type_tag)
            {
                Ok(version) => version,
                Err("SeqAppendOnlyDataNotFound") => {
                    return Err(ERROR_MSG_NO_FILES_CONTAINER_FOUND.to_string())
                }
                Err(err) => return Err(format!("Failed to get current version: {}", err)),
            };

            version = self.safe_app.append_seq_appendable_data(
                files_container_data,
                current_version + 1,
                xorname,
                type_tag,
            )?;
        }

        Ok((version, processed_files, new_files_map))
    }

    /// # Put Published ImmutableData
    /// Put data blobs onto the network.
    ///
    /// ## Example
    /// ```
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// let data = b"Something super good";
    /// let xorurl = safe.files_put_published_immutable(data).unwrap();
    /// # let received_data = safe.files_get_published_immutable(&xorurl).unwrap();
    /// # assert_eq!(received_data, data);
    /// ```
    pub fn files_put_published_immutable(&mut self, data: &[u8]) -> Result<XorUrl, String> {
        // TODO: do we want ownership from other PKs yet?
        let xorname = self.safe_app.files_put_published_immutable(&data)?;

        XorUrlEncoder::encode(
            xorname,
            0,
            SafeContentType::ImmutableData,
            &self.xorurl_base,
        )
    }

    /// # Get Published ImmutableData
    /// Put data blobs onto the network.
    ///
    /// ## Example
    /// ```
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// # let data = b"Something super good";
    /// let xorurl = safe.files_put_published_immutable(data).unwrap();
    /// let received_data = safe.files_get_published_immutable(&xorurl).unwrap();
    /// # assert_eq!(received_data, data);
    /// ```
    pub fn files_get_published_immutable(&self, xorurl: &str) -> Result<Vec<u8>, String> {
        // TODO: do we want ownership from other PKs yet?
        let xorurl_encoder = XorUrlEncoder::from_url(&xorurl)?;
        self.safe_app
            .files_get_published_immutable(xorurl_encoder.xorname())
    }
}

// Helper functions

fn get_root_path(location: &str, set_root: Option<String>) -> Result<String, String> {
    match set_root {
        Some(location) => Ok(location),
        None => {
            let normalised_location = normalise_path_separator(location);
            // lets check for a trailing '/' which results in no root
            if !normalised_location.ends_with('/') {
                let path = Path::new(&normalised_location);
                let metadata = fs::metadata(&path).map_err(|err| {
                    format!(
                        "Couldn't read metadata from source path ('{}'): {}",
                        location, err
                    )
                })?;

                if !metadata.is_dir() {
                    return Ok("".to_string());
                }
                let parts_vec: Vec<&str> = normalised_location.split('/').collect();
                let our_root = parts_vec[parts_vec.len() - 1];
                Ok(our_root.to_string())
            } else {
                Ok("".to_string())
            }
        }
    }
}

fn gen_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn normalise_path_separator(from: &str) -> String {
    str::replace(&from, "\\", "/").to_string()
}

fn gen_normalised_paths(new_content: &ProcessedFiles, root_path: String) -> (String, String) {
    // Let's normalise the path to use '/' (instead of '\' as on Windows)
    let mut base_path = normalise_path_separator(&root_path);

    if !base_path.starts_with('/') {
        base_path = format!("/{}", base_path)
    }

    let normalised_prefix = if new_content.len() > 1 {
        let mut paths: Vec<&Path> = vec![];
        new_content.keys().for_each(|key| {
            paths.push(Path::new(key));
        });
        let prefix = common_path_all(paths).unwrap_or_else(PathBuf::new);
        let normalised = &normalise_path_separator(&prefix.to_str().unwrap());
        normalised.clone()
    } else {
        "/".to_string()
    };

    (base_path, normalised_prefix)
}

fn gen_new_file_item(
    safe: &mut Safe,
    file_path: &Path,
    file_type: &str,
    file_size: &str,
    file_created: Option<&str>,
) -> Result<FileItem, String> {
    let now = gen_timestamp();
    let mut file_item = FileItem::new();
    let xorurl = upload_file(safe, file_path)?;
    file_item.insert(FILES_MAP_PREDICATE_LINK.to_string(), xorurl.to_string());
    file_item.insert(FILES_MAP_PREDICATE_TYPE.to_string(), file_type.to_string());
    file_item.insert(FILES_MAP_PREDICATE_SIZE.to_string(), file_size.to_string());
    file_item.insert(FILES_MAP_PREDICATE_MODIFIED.to_string(), now.clone());
    let created = file_created.unwrap_or_else(|| &now);
    file_item.insert(FILES_MAP_PREDICATE_CREATED.to_string(), created.to_string());

    Ok(file_item)
}

fn files_map_sync(
    safe: &mut Safe,
    mut current_files_map: FilesMap,
    new_content: ProcessedFiles,
    root_path: String,
    delete: bool,
) -> Result<(ProcessedFiles, FilesMap, u64), String> {
    let (base_path, normalised_prefix) = gen_normalised_paths(&new_content, root_path);
    let mut updated_files_map = FilesMap::new();
    let mut processed_files = ProcessedFiles::new();
    let mut success_count = 0;

    for (key, _value) in new_content
        .iter()
        .filter(|(_, (change, _))| change != FILE_ERROR_SIGN)
    {
        let metadata = fs::metadata(&key).map_err(|err| {
            format!(
                "Couldn't obtain metadata information for local file ('{}'): {:?}",
                key, err,
            )
        })?;

        let file_path = Path::new(&key);
        let file_type = match &file_path.extension() {
            Some(ext) => ext.to_str().ok_or("unknown")?,
            None => "unknown",
        };

        let file_size = metadata.len().to_string();

        let file_name =
            RelativePath::new(&key.to_string().replace(&normalised_prefix, &base_path)).normalize();
        // Above normalize removes initial slash, and uses '\' if it's on Windows
        let normalised_file_name = format!("/{}", normalise_path_separator(file_name.as_str()));

        // Let's update FileItem if there is a change or it doesn't exist in current_files_map
        match current_files_map.get(&normalised_file_name) {
            None => {
                // We need to add a new FileItem, let's upload it first
                match gen_new_file_item(safe, &file_path, &file_type, &file_size, None) {
                    Ok(new_file_item) => {
                        debug!("New FileItem item: {:?}", new_file_item);
                        debug!("New FileItem item inserted as {:?}", &file_name);
                        updated_files_map.insert(normalised_file_name, new_file_item.clone());
                        processed_files.insert(
                            key.to_string(),
                            (
                                FILE_ADDED_SIGN.to_string(),
                                new_file_item[FILES_MAP_PREDICATE_LINK].clone(),
                            ),
                        );
                        success_count += 1;
                    }
                    Err(err) => {
                        processed_files.insert(
                            key.to_string(),
                            (FILE_ERROR_SIGN.to_string(), format!("<{}>", err)),
                        );
                        info!(
                        "Skipping file \"{}\" since it couldn't be uploaded to the network: {:?}",
                        normalised_file_name, err);
                    }
                };
            }
            Some(file_item) => {
                // TODO: we don't record the original creation/modified timestamp from the,
                // filesystem thus we cannot compare to see if they changed
                if file_item[FILES_MAP_PREDICATE_SIZE] != file_size
                    || file_item[FILES_MAP_PREDICATE_TYPE] != file_type
                {
                    // We need to update the current FileItem, let's upload it first
                    match gen_new_file_item(
                        safe,
                        &file_path,
                        &file_type,
                        &file_size,
                        Some(&file_item[FILES_MAP_PREDICATE_CREATED]),
                    ) {
                        Ok(new_file_item) => {
                            debug!("Updated FileItem item: {:?}", new_file_item);
                            debug!("Updated FileItem item inserted as {:?}", &file_name);
                            updated_files_map
                                .insert(normalised_file_name.to_string(), new_file_item.clone());
                            processed_files.insert(
                                key.to_string(),
                                (
                                    FILE_UPDATED_SIGN.to_string(),
                                    new_file_item[FILES_MAP_PREDICATE_LINK].clone(),
                                ),
                            );
                            success_count += 1;
                        }
                        Err(err) => {
                            processed_files.insert(
                                key.to_string(),
                                (FILE_ERROR_SIGN.to_string(), format!("<{}>", err)),
                            );
                            info!("Skipping file \"{}\": {}", &normalised_file_name, err);
                        }
                    };
                } else {
                    // No need to update FileItem just copy the existing one
                    updated_files_map.insert(normalised_file_name.to_string(), file_item.clone());
                }

                // let's now remove it form the current list so we now it has been processed
                current_files_map.remove(&normalised_file_name);
            }
        }
    }

    // Finally, unless 'delete' was set keep the files that are currently
    // in FilesContainer but not in source location
    current_files_map.iter().for_each(|(file_name, file_item)| {
        if !delete {
            updated_files_map.insert(file_name.to_string(), file_item.clone());
        } else {
            processed_files.insert(
                file_name.to_string(),
                (
                    FILE_DELETED_SIGN.to_string(),
                    file_item[FILES_MAP_PREDICATE_LINK].clone(),
                ),
            );
        }
    });

    Ok((processed_files, updated_files_map, success_count))
}

fn upload_file(safe: &mut Safe, path: &Path) -> Result<XorUrl, String> {
    let data = fs::read(path)
        .map_err(|err| format!("Failed to read file from local location: {}", err))?;
    safe.files_put_published_immutable(&data)
}

fn file_system_dir_walk(
    safe: &mut Safe,
    location: &str,
    recursive: bool,
    upload_files: bool,
) -> Result<ProcessedFiles, String> {
    let path = Path::new(location);
    info!("Reading files from {}", &path.display());
    let metadata = fs::metadata(&path).map_err(|err| {
        format!(
            "Couldn't read metadata from source path ('{}'): {}",
            location, err
        )
    })?;
    debug!("Metadata for location: {:?}", metadata);

    let mut processed_files = BTreeMap::new();
    if recursive {
        // TODO: option to enable following symlinks and hidden files?
        // We now compare both FilesMaps to upload the missing files
        WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| is_not_hidden(e))
            .filter_map(|v| v.ok())
            .for_each(|child| {
                info!("{}", child.path().display());
                let current_file_path = child.path();
                let current_path_str = current_file_path.to_str().unwrap_or_else(|| "").to_string();
                let normalised_path = normalise_path_separator(&current_path_str);
                info!("Normalised path: {}", normalised_path);
                match fs::metadata(&current_file_path) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            // Everything is in the iter. We dont need to recurse.
                            // so what do we do with dirs? decide if we want to support empty dirs also
                        } else if upload_files {
                            match upload_file(safe, &current_file_path) {
                                Ok(xorurl) => {
                                    processed_files.insert(normalised_path, (FILE_ADDED_SIGN.to_string(), xorurl));
                                }
                                Err(err) => {
                                    processed_files.insert(normalised_path.clone(), (FILE_ERROR_SIGN.to_string(), format!("<{}>", err)));
                                    info!(
                                    "Skipping file \"{}\". {}",
                                    normalised_path, err);
                                },
                            };
                        } else {
                            processed_files.insert(current_path_str, ("".to_string(), "".to_string()));
                        }
                    },
                    Err(err) => {
                        processed_files.insert(normalised_path.clone(), (FILE_ERROR_SIGN.to_string(), format!("<{}>", err)));
                        info!(
                        "Skipping file \"{}\" since no metadata could be read from local location: {:?}",
                        normalised_path, err);
                    }
                }
            });
    } else {
        if metadata.is_dir() {
            return Err(format!(
                "{:?} is a directory. Use \"-r\" to recursively upload folders.",
                location
            ));
        }

        if upload_files {
            let xorurl = upload_file(safe, &path)?;
            processed_files.insert(location.to_string(), (FILE_ADDED_SIGN.to_string(), xorurl));
        } else {
            processed_files.insert(location.to_string(), ("".to_string(), "".to_string()));
        }
    }

    Ok(processed_files)
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with('.'))
        .unwrap_or(false)
}

fn files_map_create(content: &ProcessedFiles, root_path: String) -> Result<FilesMap, String> {
    let mut files_map = FilesMap::default();
    let now = gen_timestamp();

    let (base_path, normalised_prefix) = gen_normalised_paths(content, root_path);
    for (file_name, (_change, link)) in content
        .iter()
        .filter(|(_, (change, _))| change != FILE_ERROR_SIGN)
    {
        debug!("FileItem item name:{:?}", &file_name);
        let mut file_item = FileItem::new();
        let metadata = fs::metadata(&file_name).map_err(|err| {
            format!(
                "Couldn't obtain metadata information for local file: {:?}",
                err,
            )
        })?;

        file_item.insert(FILES_MAP_PREDICATE_LINK.to_string(), link.to_string());

        let file_path = Path::new(&file_name);
        let file_type: &str = match &file_path.extension() {
            Some(ext) => ext.to_str().ok_or("unknown")?,
            None => "unknown",
        };

        file_item.insert(FILES_MAP_PREDICATE_TYPE.to_string(), file_type.to_string());

        let file_size = &metadata.len().to_string();
        file_item.insert(FILES_MAP_PREDICATE_SIZE.to_string(), file_size.to_string());

        // file_item.insert("permissions", metadata.permissions().to_string());
        file_item.insert(FILES_MAP_PREDICATE_MODIFIED.to_string(), now.clone());
        file_item.insert(FILES_MAP_PREDICATE_CREATED.to_string(), now.clone());

        debug!("FileItem item: {:?}", file_item);
        let new_file_name = RelativePath::new(
            &file_name
                .to_string()
                .replace(&normalised_prefix, &base_path),
        )
        .normalize();

        // Above normalize removes initial slash, and uses '\' if it's on Windows
        let final_name = format!("/{}", normalise_path_separator(new_file_name.as_str()));

        debug!("FileItem item inserted as {:?}", &final_name);
        files_map.insert(final_name.to_string(), file_item);
    }

    Ok(files_map)
}

// Unit Tests

#[test]
fn test_files_map_create() {
    use unwrap::unwrap;
    let mut processed_files = ProcessedFiles::new();
    processed_files.insert(
        "./tests/testfolder/test.md".to_string(),
        (FILE_ADDED_SIGN.to_string(), "safe://top_xorurl".to_string()),
    );
    processed_files.insert(
        "./tests/testfolder/subfolder/subexists.md".to_string(),
        (
            FILE_ADDED_SIGN.to_string(),
            "safe://second_xorurl".to_string(),
        ),
    );
    let files_map = unwrap!(files_map_create(&processed_files, "".to_string()));
    assert_eq!(files_map.len(), 2);
    let file_item1 = &files_map["/test.md"];
    assert_eq!(file_item1[FILES_MAP_PREDICATE_LINK], "safe://top_xorurl");
    assert_eq!(file_item1[FILES_MAP_PREDICATE_TYPE], "md");
    if cfg!(windows) {
        assert_eq!(file_item1[FILES_MAP_PREDICATE_SIZE], "14"); // due to \r
    } else {
        assert_eq!(file_item1[FILES_MAP_PREDICATE_SIZE], "13");
    }
    let file_item2 = &files_map["/subfolder/subexists.md"];
    assert_eq!(file_item2[FILES_MAP_PREDICATE_LINK], "safe://second_xorurl");
    assert_eq!(file_item2[FILES_MAP_PREDICATE_TYPE], "md");
    if cfg!(windows) {
        assert_eq!(file_item2[FILES_MAP_PREDICATE_SIZE], "9"); // due to \r
    } else {
        assert_eq!(file_item2[FILES_MAP_PREDICATE_SIZE], "8");
    }
}

#[test]
fn test_files_container_create_file() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let filename = "tests/testfolder/test.md";
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create(filename, false, None));

    println!("\n\nP: {:?}\n\n", processed_files);
    println!("F: {:?}", files_map);

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 1);
    assert_eq!(files_map.len(), 1);
    let file_path = "/tests/testfolder/test.md";
    assert_eq!(processed_files[filename].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename].1,
        files_map[file_path][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_create_folder_without_end_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("tests/testfolder", true, None));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 4);
    assert_eq!(files_map.len(), 4);

    let filename1 = "tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        files_map["/testfolder/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        files_map["/testfolder/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        files_map["/testfolder/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        files_map["/testfolder/noextension"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_create_folder_with_end_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", true, None));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 4);
    assert_eq!(files_map.len(), 4);

    let filename1 = "./tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        files_map["/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "./tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        files_map["/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        files_map["/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "./tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        files_map["/noextension"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_create_set_root() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) = unwrap!(safe.files_container_create(
        "./tests/testfolder",
        true,
        Some("/myroot/folder".to_string())
    ));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 4);
    assert_eq!(files_map.len(), 4);

    let filename1 = "./tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        files_map["/myroot/folder/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "./tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        files_map["/myroot/folder/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        files_map["/myroot/folder/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "./tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        files_map["/myroot/folder/noextension"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", true, None));

    assert_eq!(processed_files.len(), 4);
    assert_eq!(files_map.len(), 4);

    let (version, new_processed_files, new_files_map) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder/",
        &xorurl,
        true,
        None,
        false
    ));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 1);
    assert_eq!(new_files_map.len(), 5);

    let filename1 = "./tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        new_files_map["/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "./tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        new_files_map["/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        new_files_map["/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "./tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        new_files_map["/noextension"][FILES_MAP_PREDICATE_LINK]
    );

    // and finally check the synced file is there
    let filename5 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(new_processed_files[filename5].0, FILE_ADDED_SIGN);
    assert_eq!(
        new_processed_files[filename5].1,
        new_files_map["/tests/testfolder/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync_with_delete() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", true, None));

    assert_eq!(processed_files.len(), 4);
    assert_eq!(files_map.len(), 4);

    let (version, new_processed_files, new_files_map) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder/",
        &xorurl,
        true,
        None,
        true // this sets the delete flag
    ));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 5);
    assert_eq!(new_files_map.len(), 1);

    // first check all previous files were removed
    let file_path1 = "/test.md";
    assert_eq!(new_processed_files[file_path1].0, FILE_DELETED_SIGN);
    assert_eq!(
        new_processed_files[file_path1].1,
        files_map[file_path1][FILES_MAP_PREDICATE_LINK]
    );

    let file_path2 = "/another.md";
    assert_eq!(new_processed_files[file_path2].0, FILE_DELETED_SIGN);
    assert_eq!(
        new_processed_files[file_path2].1,
        files_map[file_path2][FILES_MAP_PREDICATE_LINK]
    );

    let file_path3 = "/subfolder/subexists.md";
    assert_eq!(new_processed_files[file_path3].0, FILE_DELETED_SIGN);
    assert_eq!(
        new_processed_files[file_path3].1,
        files_map[file_path3][FILES_MAP_PREDICATE_LINK]
    );

    let file_path4 = "/noextension";
    assert_eq!(new_processed_files[file_path4].0, FILE_DELETED_SIGN);
    assert_eq!(
        new_processed_files[file_path4].1,
        files_map[file_path4][FILES_MAP_PREDICATE_LINK]
    );

    // and finally check the synced file was added
    let filename5 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(new_processed_files[filename5].0, FILE_ADDED_SIGN);
    assert_eq!(
        new_processed_files[filename5].1,
        new_files_map["/tests/testfolder/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_get_latest() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, _processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", true, None));

    let (version, fetched_files_map, native_type) =
        unwrap!(safe.files_container_get_latest(&xorurl));

    assert_eq!(version, 1);
    assert_eq!(native_type, FILES_CONTAINER_NATIVE_TYPE);
    assert_eq!(fetched_files_map.len(), 4);
    assert_eq!(files_map.len(), fetched_files_map.len());
    assert_eq!(files_map["/test.md"], fetched_files_map["/test.md"]);
    assert_eq!(files_map["/another.md"], fetched_files_map["/another.md"]);
    assert_eq!(
        files_map["/subfolder/subexists.md"],
        fetched_files_map["/subfolder/subexists.md"]
    );
    assert_eq!(files_map["/noextension"], fetched_files_map["/noextension"]);
}

#[test]
fn test_files_container_version() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, _, _) = unwrap!(safe.files_container_create("./tests/testfolder/", true, None));

    let (version, _, _) = unwrap!(safe.files_container_get_latest(&xorurl));
    assert_eq!(version, 1);

    let (version, _, _) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder/",
        &xorurl,
        true,
        None,
        true // this sets the delete flag
    ));
    assert_eq!(version, 2);

    let (version, _, _) = unwrap!(safe.files_container_get_latest(&xorurl));
    assert_eq!(version, 2);
}
