use std::collections::{HashMap, HashSet};

use crate::aio;

use super::RepositoryFile;

pub struct FileProcessInstruction {
    file: String,
    operation: FileProcessOperation,
}

pub enum FileProcessOperation {
    Transfer,
    Delete,
    Update,
    Rename,
    Copy,
    Keep,
}

pub enum FileProcessError {
    HashConversionError,
}

// current: local machine
// goal: remote machine, which the current tries to replicate

pub fn compute(
    current_state: &HashMap<String, RepositoryFile>,
    goal_state: &HashMap<String, RepositoryFile>,
) -> Result<Vec<FileProcessInstruction>, FileProcessError> {
    let mut goal_hashes: HashMap<u128, HashSet<String>> = HashMap::new();
    let mut current_hashes: HashMap<u128, HashSet<String>> = HashMap::new();
    let mut instructions = Vec::new();

    for g_element in goal_state {
        let hash = aio::hash_str_to_u128(&g_element.1.hash)
            .map_err(|_| FileProcessError::HashConversionError)?;

        goal_hashes
            .entry(hash)
            .or_default()
            .insert(g_element.0.clone());
    }

    let mut to_delete: HashSet<String> = HashSet::new(); // <current_file_path>

    for c_element in current_state {
        let hash = aio::hash_str_to_u128(&c_element.1.hash)
            .map_err(|_| FileProcessError::HashConversionError)?;

        if !goal_hashes.contains_key(&hash) {
            // mk: already exclude hashes that are not in the client
            to_delete.insert(c_element.0.clone());
            continue;
        }

        current_hashes
            .entry(hash)
            .or_default()
            .insert(c_element.0.clone());
    }

    let mut to_keep: HashSet<String> = HashSet::new(); // <current_file_path>
    let mut to_copy: HashMap<String, String> = HashMap::new(); // <src_current_path, dest_current_path or temp_path> // <content_hash, (src_current_path, dest_current_path)>
    let mut to_move: HashMap<String, String> = HashMap::new(); // <src_current_path, dest_current_path or temp_path> // <content_hash, (src_current_path, dest_current_path)>
    let mut to_transfer: HashMap<u128, HashSet<String>> = HashMap::new(); // <other_hash, goal_files_that_have_the_hash or temp_path>
    //let mut to_transfer: HashMap<String, HashSet<String>> = HashMap::new(); // <dest_goal_and_current_path, other_files_that_have_the_same_content or temp_path> // <content_hash, dest_goal_and_current_path>

    let mut temp_paths: HashMap<String, String> = HashMap::new(); // <temp_path, supposed_to_be_path>

    // mk: for keeps, copies and deletes
    for g_element in goal_hashes.iter() {
        let Some(c_element) = current_hashes.get(g_element.0) else {
            // mk: actually network transfer + copy?
            to_transfer
                .entry(g_element.0.clone())
                .or_default()
                .extend(g_element.1.iter().cloned());
            continue;
        };

        let old_keep_len = to_keep.len();
        let intersection = g_element.1.intersection(c_element);
        to_keep.extend(intersection.cloned());
        let intersection_size = to_keep.len() - old_keep_len;

        if g_element.1.len() < c_element.len() {
            // to_delete
            let delete_or_move = c_element.difference(&g_element.1);
        } else if g_element.1.len() > c_element.len() {
            // to_copy
            let copy_or_move = c_element.difference(&g_element.1);
            let g_exclusives = g_element.1.difference(&c_element);

            
        }
    }

    Ok(instructions)
}
