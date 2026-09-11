use std::collections::{HashMap, HashSet};

use ssh_key::authorized_keys::Entry;

use crate::{
    aio::{self, extract_path_dir_and_name},
    keys::generate_random_string,
};

use super::RepositoryFile;

/*pub struct FileProcessInstruction {
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
}*/

#[derive(Debug, Clone)]
pub struct FileTranformations {
    pub to_delete: HashSet<String>,
    pub to_keep: HashSet<String>,
    //pub to_copy: HashMap<String, (Option<String>, HashSet<String>)>,
    pub to_copy: HashMap<String, (Option<String>, Vec<String>)>,
    pub to_move: HashMap<String, String>,
    pub to_transfer: HashMap<u128, Vec<String>>,
}

pub type TempPaths = HashMap<String, String>;

#[derive(Debug)]
pub enum FileProcessError {
    HashConversionError,
}

// current: local machine
// goal: remote machine, which the current tries to replicate

// mk: okay, i somehow managed to do it, it was a bit tricky, i know there are better ways to achieve this, but apparently it works
// mk: just a note for myself or others: i would consider improving this in the future
pub fn compute(
    current_state: &HashMap<String, RepositoryFile>,
    goal_state: &HashMap<String, RepositoryFile>,
) -> Result<FileTranformations, FileProcessError> {
    let mut goal_hashes: HashMap<u128, HashSet<String>> = HashMap::new();
    let mut current_hashes: HashMap<u128, HashSet<String>> = HashMap::new();

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

    let mut to_keep: HashSet<String> = HashSet::new();
    let mut to_copy: HashMap<String, (Option<String>, Vec<String>)> = HashMap::new();
    let mut to_move: HashMap<String, String> = HashMap::new();
    let mut to_transfer: HashMap<u128, Vec<String>> = HashMap::new();
    //let mut to_transfer: HashMap<String, HashSet<String>> = HashMap::new(); // <dest_goal_and_current_path, other_files_that_have_the_same_content or temp_path> // <content_hash, dest_goal_and_current_path>

    let mut temp_paths: HashMap<String, String> = HashMap::new(); // <temp_path, supposed_to_be_path>

    // mk: for keeps, copies and deletes
    for g_element in goal_hashes.iter() {
        let Some(c_element) = current_hashes.get(g_element.0) else {
            // mk: actually network transfer + copy?
            to_transfer
                .entry(*g_element.0)
                .or_default()
                .extend(g_element.1.iter().cloned());
            continue;
        };

        let intersection = g_element.1.intersection(c_element);
        to_keep.extend(intersection.cloned());
        let mut g_exclusives = g_element.1.difference(c_element);

        if g_element.1.len() < c_element.len() {
            // to_delete
            let delete_or_move = c_element.difference(g_element.1);

            for dm in delete_or_move {
                let g = g_exclusives.next();
                match g {
                    Some(g) => {
                        to_move.insert(dm.clone(), g.clone());
                    }
                    None => {
                        to_delete.insert(dm.clone());
                    }
                }
            }
        } else if g_element.1.len() > c_element.len()
        // mk: alr there are 2 ways to handle a single move, i could make an else condition and just make a move, but it still works this way
        // mk: nvm, apparently this causes issues if there are elements to keep
        {
            // to_copy
            let mut copy_or_move = c_element.difference(g_element.1);
            //let mut copy_to_or_move_to = g_element.1.difference(c_element);
            //let mut g_exclusives = g_element.1.difference(c_element);
            println!("hash: \"{}\" {:?}", g_element.0, g_exclusives);
            let mut intersection = g_element.1.intersection(c_element);

            let a_common_one = intersection.next();
            /*let Some(a_common_one) = intersection.next() else {
                continue;
            };*/

            let current_origin: &String;

            let copy_set: HashSet<String> = HashSet::new();
            //let mut copy_set_ref = &mut copy_set;

            match a_common_one {
                Some(common) => {
                    // no rename
                    current_origin = common;
                    to_copy.insert(common.clone(), (None, copy_set.iter().cloned().collect())); // mk: was just `copy_set` with hashset, the perfomance is fucked
                }
                None => {
                    // rename
                    let Some(one_current_entry) = copy_or_move.next() else {
                        continue;
                    };

                    let Some(one_goal_entry) = g_exclusives.next() else {
                        continue;
                    };

                    current_origin = one_current_entry;

                    to_copy.insert(
                        one_current_entry.clone(),
                        (
                            Some(one_goal_entry.clone()),
                            copy_set.iter().cloned().collect(),
                        ), // mk: same here qwq
                    );
                }
            }

            for g in g_exclusives {
                let cm = copy_or_move.next();
                match cm {
                    Some(cm) => {
                        // move
                        to_move.insert(cm.clone(), g.clone());
                    }
                    None => {
                        // copy
                        if let Some(copy_tuple) = to_copy.get_mut(current_origin) {
                            let copy_set = &mut copy_tuple.1;
                            //copy_set.insert(g.clone());
                            copy_set.push(g.clone());
                        }
                    }
                }
            }
        } else {
            let mut just_move = c_element.difference(g_element.1);
            for g in g_exclusives {
                let cm = just_move.next();
                match cm {
                    Some(cm) => {
                        // move
                        to_move.insert(cm.clone(), g.clone());
                    }
                    None => {
                        // it's not supposed to end here
                    }
                }
            }
        }
    }

    Ok(FileTranformations {
        to_delete,
        to_keep,
        to_copy,
        to_move,
        to_transfer,
    })
}

pub fn filter_temp<F>(
    tranformations: &mut FileTranformations,
    existing_file_predicate: F,
) -> HashMap<String, String>
where
    F: Fn(&str) -> bool,
{
    let mut temp_paths: TempPaths = HashMap::new();

    let to_move = &mut tranformations.to_move;
    let to_copy = &mut tranformations.to_copy;
    let to_transfer = &mut tranformations.to_transfer;
    let to_delete = &mut tranformations.to_delete;

    fn generate_temp_path<F>(path: &str, existing_predicate: F) -> String
    where
        F: Fn(&str) -> bool,
    {
        let path = extract_path_dir_and_name(path);

        let mut output;
        loop {
            let random = generate_random_string(8);
            output = format!("{}/{}.tmp", path.0, random);
            if !existing_predicate(&output) {
                break;
            }
        }

        if output.starts_with('/') {
            output.remove(0);
        }

        output
    }

    // mk: i will use smart pointers (Rc<str>) in the future, now i just want something functional
    let mut seen: HashSet<String> = HashSet::from_iter(to_move.keys().cloned());
    seen.extend(to_copy.keys().cloned());
    seen.extend(to_delete.iter().cloned());
    //seen.extend(to_transfer.keys().cloned());

    let seen: HashMap<String, Option<String>> = seen.into_iter().map(|x| (x, None)).collect();

    for value in to_move.values_mut() {
        let Some(seen_value) = seen.get(value) else {
            continue;
        };

        let temp_path = match seen_value {
            Some(path) => path.clone(),
            None => {
                let path = generate_temp_path(value, &existing_file_predicate);
                temp_paths.insert(path.clone(), value.clone());
                path
            }
        };

        *value = temp_path.clone();
    }

    for value in to_copy.values_mut() {
        if let Some(value_move) = &mut value.0
            && let Some(seen_value) = seen.get(value_move)
        {
            let temp_path = match seen_value {
                Some(path) => path.clone(),
                None => {
                    let path = generate_temp_path(value_move, &existing_file_predicate);
                    temp_paths.insert(path.clone(), value_move.clone());
                    path
                }
            };

            *value_move = temp_path.clone();
        };

        for value_copy in value.1.iter_mut() {
            let Some(seen_value) = seen.get(value_copy) else {
                continue;
            };

            let temp_path = match seen_value {
                Some(path) => path.clone(),
                None => {
                    let path = generate_temp_path(value_copy, &existing_file_predicate);
                    temp_paths.insert(path.clone(), value_copy.clone());
                    path
                }
            };

            *value_copy = temp_path.clone();
        }
    }

    for value in to_transfer.values_mut() {
        for value_tranform in value.iter_mut() {
            let Some(seen_value) = seen.get(value_tranform) else {
                continue;
            };

            let temp_path = match seen_value {
                Some(path) => path.clone(),
                None => {
                    let path = generate_temp_path(value_tranform, &existing_file_predicate);
                    temp_paths.insert(path.clone(), value_tranform.clone());
                    path
                }
            };

            *value_tranform = temp_path.clone();
        }
    }

    temp_paths
}

// mk: i had to ask the rust community on discord, to fix the borrow problem, they suggested the following elegant O(n) solution
// mk: my original solution had O(n^2) complexity
// mk: but there's a lil issue and i can't use this, a few small changes should be enough to fix it tho
// mk: following snippet credits: oklyth, thank you rustacean :)
/*fn ensure_unique(paths: &mut HashMap<Rc<str>, Rc<str>>) {
    let mut seen: HashSet<Rc<str>> = HashSet::from_iter(paths.keys().cloned());
    let mut replacements: HashMap<Rc<str>, Rc<str>> = HashMap::new();
    for value in paths.values_mut() {
        // if we've already seen this path before...
        if !seen.insert(value.clone()) {
            // try to reuse an existing temporary path, otherwise make a new one
            *value = replacements
                .entry(value.clone())
                .or_insert_with_key(|v| make_temporary(v))
                .clone();
        }
    }
}

fn make_temporary(original: &str) -> Rc<str> {
    todo!()
}*/

// mk: the following are the old types i was using, comments i made may be not accurate due to the fact i changed it multiple times over time
/*
    let mut to_keep: HashSet<String> = HashSet::new(); // <current_file_path>
    let mut to_copy: HashMap<String, (Option<String>, HashSet<String>)> = HashMap::new(); // <src_current_path, dest_current_path or temp_path> // <content_hash, (src_current_path, dest_current_path)>
    let mut to_move: HashMap<String, String> = HashMap::new(); // <src_current_path, dest_current_path or temp_path> // <content_hash, (src_current_path, dest_current_path)>
    let mut to_transfer: HashMap<u128, HashSet<String>> = HashMap::new(); // <other_hash, goal_files_that_have_the_hash or temp_path>
    //let mut to_transfer: HashMap<String, HashSet<String>> = HashMap::new(); // <dest_goal_and_current_path, other_files_that_have_the_same_content or temp_path> // <content_hash, dest_goal_and_current_path>

    let mut temp_paths: HashMap<String, String> = HashMap::new(); // <temp_path, supposed_to_be_path>
*/

impl FileTranformations {
    // keeped, moved, copied, transfered, deleted
    pub fn count(&self) -> (usize, usize, usize, usize, usize) {
        let mut move_count: usize = self.to_move.len();
        let mut copy_count: usize = 0;

        for entry in self.to_copy.iter() {
            if entry.1.0.is_some() {
                move_count += 1;
            }

            copy_count += entry.1.1.len();
        }

        for entry in self.to_transfer.iter() {
            copy_count += entry.1.len() - 1;
        }

        (
            self.to_keep.len(),
            move_count,
            copy_count,
            self.to_transfer.len(),
            self.to_delete.len(),
        )
    }
}
