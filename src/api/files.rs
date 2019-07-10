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
use log::{debug, info};
use relative_path::RelativePath;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
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

const MAX_RECURSIVE_DEPTH: usize = 10000;

#[allow(dead_code)]
impl Safe {
    /// # Create a FilesContaier.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use safe_cli::Safe;
    /// # let mut safe = Safe::new("base32z".to_string());
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", None, true).unwrap();
    /// assert!(xorurl.contains("safe://"))
    /// ```
    pub fn files_container_create(
        &mut self,
        location: &str,
        dest: Option<String>,
        recursive: bool,
    ) -> Result<(XorUrl, ProcessedFiles, FilesMap), String> {
        // TODO: Enable source for funds / ownership
        // Warn about ownership?

        // Let's upload the files and generate the list of local files paths
        let processed_files = file_system_dir_walk(self, location, recursive, true)?;

        // The FilesContainer is created as a AppendOnlyData with a single entry containing the
        // timestamp as the entry's key, and the serialised FilesMap as the entry's value
        // TODO: use RDF format
        let files_map = files_map_create(&processed_files, location, dest)?;
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
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", None, true).unwrap();
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
    /// let (xorurl, _processed_files, _files_map) = safe.files_container_create("tests/testfolder", None, true).unwrap();
    /// let (version, new_processed_files, new_files_map) = safe.files_container_sync("tests/testfolder", &xorurl, true, false).unwrap();
    /// println!("FilesContainer fetched is at version: {}", version);
    /// println!("The local files that were synced up are: {:?}", new_processed_files);
    /// println!("The FilesMap of the updated FilesContainer now is: {:?}", new_files_map);
    /// ```
    pub fn files_container_sync(
        &mut self,
        location: &str,
        xorurl: &str,
        recursive: bool,
        delete: bool,
    ) -> Result<(u64, ProcessedFiles, FilesMap), String> {
        let (mut version, current_files_map, _): (u64, FilesMap, String) =
            self.files_container_get_latest(xorurl)?;

        // Let's generate the list of local files paths, without uploading any new file yet
        let processed_files = file_system_dir_walk(self, location, recursive, false)?;

        let xorurl_encoder = XorUrlEncoder::from_url(xorurl)?;
        let dest_path = Some(xorurl_encoder.path().to_string());
        let (processed_files, new_files_map, success_count): (ProcessedFiles, FilesMap, u64) =
            files_map_sync(
                self,
                current_files_map,
                location,
                processed_files,
                dest_path,
                delete,
            )?;

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

fn gen_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

// Simply change Windows style path separator into `/`
fn normalise_path_separator(from: &str) -> String {
    str::replace(&from, "\\", "/").to_string()
}

// From the location path and the destination path chosen by the user, calculate
// the destination path considering ending '/' in both the  location and dest path
fn get_base_paths(location: &str, dest_path: Option<String>) -> Result<(String, String), String> {
    // Let's normalise the path to use '/' (instead of '\' as on Windows)
    let location_base_path = normalise_path_separator(location);

    let new_dest_path = match dest_path {
        Some(path) => {
            if path.is_empty() {
                "/".to_string()
            } else {
                path
            }
        }
        None => "/".to_string(),
    };

    // Let's first check if it ends with '/'
    let dest_base_path = if new_dest_path.ends_with('/') {
        if location.ends_with('/') {
            new_dest_path
        } else {
            // Location is a folder, then append it to dest path
            let parts_vec: Vec<&str> = location.split('/').collect();
            let dir_name = parts_vec[parts_vec.len() - 1];
            format!("{}{}", new_dest_path, dir_name)
        }
    } else {
        // Then just append an ending '/'
        format!("{}/", new_dest_path)
    };

    Ok((location_base_path, dest_base_path))
}

// Generate a FileItem for a file which can then be added to a FilesMap
// This is now a pseudo-RDF but will eventually be converted to be an RDF graph
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

// From the provided list of local files paths, find the local changes made in comparison with the
// target FilesContainer, uploading new files as necessary, and creating a new FilesMap with file's
// metadata and their corresponding links, as well as generating the report of processed files
fn files_map_sync(
    safe: &mut Safe,
    mut current_files_map: FilesMap,
    location: &str,
    new_content: ProcessedFiles,
    dest_path: Option<String>,
    delete: bool,
) -> Result<(ProcessedFiles, FilesMap, u64), String> {
    let (location_base_path, dest_base_path) = get_base_paths(location, dest_path)?;
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

        let file_name = RelativePath::new(
            &key.to_string()
                .replace(&location_base_path, &dest_base_path),
        )
        .normalize();
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

// Upload a files to the Network as a Published-ImmutableData
fn upload_file(safe: &mut Safe, path: &Path) -> Result<XorUrl, String> {
    let data = fs::read(path)
        .map_err(|err| format!("Failed to read file from local location: {}", err))?;
    safe.files_put_published_immutable(&data)
}

// Walk the local filesystem starting from `location`, creating a list of files paths,
// and if requested with `upload_files` arg, upload the files to the network filling up
// the list of files with their corresponding XOR-URLs
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

    if metadata.is_dir() || !recursive {
        // TODO: option to enable following symlinks and hidden files?
        // We now compare both FilesMaps to upload the missing files
        let max_depth = if recursive { MAX_RECURSIVE_DEPTH } else { 1 };
        let mut processed_files = BTreeMap::new();
        WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| not_hidden_and_valid_depth(e, max_depth))
            .filter_map(|v| v.ok())
            .for_each(|child| {
                let current_file_path = child.path();
                let current_path_str = current_file_path.to_str().unwrap_or_else(|| "").to_string();
                info!("{}", current_path_str);
                let normalised_path = normalise_path_separator(&current_path_str);
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
                            processed_files.insert(normalised_path.clone(), ("".to_string(), "".to_string()));
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

        Ok(processed_files)
    } else {
        // Recursive only works on a dir path. Let's error as the user may be making a mistake
        // so it's better for the user to double check and either provide the correct path
        // or remove the `--recursive` from the args
        Err(format!(
            "'{}' is not a directory. The \"--recursive\" arg is only supported for folders.",
            location
        ))
    }
}

fn not_hidden_and_valid_depth(entry: &DirEntry, max_depth: usize) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() <= max_depth && (entry.depth() == 0 || !s.starts_with('.')))
        .unwrap_or(false)
}

// From the provided list of local files paths and corresponding files XOR-URLs,
// create a FilesMap with file's metadata and their corresponding links
fn files_map_create(
    content: &ProcessedFiles,
    location: &str,
    dest_path: Option<String>,
) -> Result<FilesMap, String> {
    let mut files_map = FilesMap::default();
    let now = gen_timestamp();

    let (location_base_path, dest_base_path) = get_base_paths(location, dest_path)?;
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
                .replace(&location_base_path, &dest_base_path),
        )
        .normalize();

        // Above normalize removes initial slash, and uses '\' if it's on Windows
        let final_name = format!("/{}", normalise_path_separator(new_file_name.as_str()));

        debug!("FileItem item inserted with filename {:?}", &final_name);
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
    let files_map = unwrap!(files_map_create(
        &processed_files,
        "./tests/testfolder",
        Some("".to_string())
    ));
    assert_eq!(files_map.len(), 2);
    let file_item1 = &files_map["/testfolder/test.md"];
    assert_eq!(file_item1[FILES_MAP_PREDICATE_LINK], "safe://top_xorurl");
    assert_eq!(file_item1[FILES_MAP_PREDICATE_TYPE], "md");
    assert_eq!(file_item1[FILES_MAP_PREDICATE_SIZE], "12");

    let file_item2 = &files_map["/testfolder/subfolder/subexists.md"];
    assert_eq!(file_item2[FILES_MAP_PREDICATE_LINK], "safe://second_xorurl");
    assert_eq!(file_item2[FILES_MAP_PREDICATE_TYPE], "md");
    if cfg!(windows) {
        assert_eq!(file_item2[FILES_MAP_PREDICATE_SIZE], "9"); // due to \r
    } else {
        assert_eq!(file_item2[FILES_MAP_PREDICATE_SIZE], "7");
    }
}

#[test]
fn test_files_container_create_file() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let filename = "tests/testfolder/test.md";
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create(filename, None, false));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 1);
    assert_eq!(files_map.len(), 1);
    let file_path = "/test.md";
    assert_eq!(processed_files[filename].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename].1,
        files_map[file_path][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_create_folder_without_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("tests/testfolder", None, true));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

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
fn test_files_container_create_folder_with_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

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
fn test_files_container_create_dest_path_without_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) = unwrap!(safe.files_container_create(
        "./tests/testfolder",
        Some("/myroot".to_string()),
        true
    ));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

    let filename1 = "./tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        files_map["/myroot/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "./tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        files_map["/myroot/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        files_map["/myroot/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "./tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        files_map["/myroot/noextension"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_create_dest_path_with_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) = unwrap!(safe.files_container_create(
        "./tests/testfolder",
        Some("/myroot/".to_string()),
        true
    ));

    assert!(xorurl.starts_with("safe://"));
    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

    let filename1 = "./tests/testfolder/test.md";
    assert_eq!(processed_files[filename1].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename1].1,
        files_map["/myroot/testfolder/test.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename2 = "./tests/testfolder/another.md";
    assert_eq!(processed_files[filename2].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename2].1,
        files_map["/myroot/testfolder/another.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename3 = "./tests/testfolder/subfolder/subexists.md";
    assert_eq!(processed_files[filename3].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename3].1,
        files_map["/myroot/testfolder/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );

    let filename4 = "./tests/testfolder/noextension";
    assert_eq!(processed_files[filename4].0, FILE_ADDED_SIGN);
    assert_eq!(
        processed_files[filename4].1,
        files_map["/myroot/testfolder/noextension"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

    let (version, new_processed_files, new_files_map) =
        unwrap!(safe.files_container_sync("./tests/testfolder/subfolder/", &xorurl, true, false));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 2);
    assert_eq!(new_files_map.len(), 7);

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
        new_files_map["/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync_with_delete() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);

    let (version, new_processed_files, new_files_map) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder/",
        &xorurl,
        true,
        true // this sets the delete flag
    ));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 7);
    assert_eq!(new_files_map.len(), 2);

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
        new_files_map["/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync_target_path_without_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);
    let xorurl_with_path = format!("{}/path/when/sync", xorurl);
    let (version, new_processed_files, new_files_map) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder",
        &xorurl_with_path,
        true,
        false
    ));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 2);
    assert_eq!(new_files_map.len(), 7);

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
        new_files_map["/path/when/sync/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_sync_target_path_with_trailing_slash() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    assert_eq!(processed_files.len(), 5);
    assert_eq!(files_map.len(), 5);
    let xorurl_with_path = format!("{}/path/when/sync/", xorurl);
    let (version, new_processed_files, new_files_map) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder",
        &xorurl_with_path,
        true,
        false
    ));

    assert_eq!(version, 2);
    assert_eq!(new_processed_files.len(), 2);
    assert_eq!(new_files_map.len(), 7);

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
        new_files_map["/path/when/sync/subfolder/subexists.md"][FILES_MAP_PREDICATE_LINK]
    );
}

#[test]
fn test_files_container_get_latest() {
    use unwrap::unwrap;
    let mut safe = Safe::new("base32z".to_string());
    let (xorurl, _processed_files, files_map) =
        unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    let (version, fetched_files_map, native_type) =
        unwrap!(safe.files_container_get_latest(&xorurl));

    assert_eq!(version, 1);
    assert_eq!(native_type, FILES_CONTAINER_NATIVE_TYPE);
    assert_eq!(fetched_files_map.len(), 5);
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
    let (xorurl, _, _) = unwrap!(safe.files_container_create("./tests/testfolder/", None, true));

    let (version, _, _) = unwrap!(safe.files_container_get_latest(&xorurl));
    assert_eq!(version, 1);

    let (version, _, _) = unwrap!(safe.files_container_sync(
        "./tests/testfolder/subfolder/",
        &xorurl,
        true,
        true // this sets the delete flag
    ));
    assert_eq!(version, 2);

    let (version, _, _) = unwrap!(safe.files_container_get_latest(&xorurl));
    assert_eq!(version, 2);
}
