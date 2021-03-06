use std::fmt::Debug;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use sha256::digest;
use std::fs::metadata;
use std::io::{Error, ErrorKind};

#[derive(Serialize, Deserialize, Debug)]
/// A struct that represents a key-value store.
pub struct KVStore {
    /// The number of key-value mappings currently stored.
    size: usize,
    /// The location of the file system where key-value mappings are stored.
    path: String,
}

/// A trait that defines the operations that need to be supported.
pub trait Operations {
    /// A function that initializes a KVStore instance.
    ///
    /// If there is no directory at the provided path, this should create it. If there is an error
    /// while creating a directory, this should return an [std::io::Error].
    ///
    /// If there are **no** key-value mappings stored already under the directory, this
    /// should simply create a new KVStore instance that can store and retrieve key-value mappings
    /// using the directory. It should also correctly initialize the size to 0.
    ///
    /// If there **are** existing key-value mappings stored already under the directory, this
    /// should initialize a KVStore instance that is able to store and retrieve existing key-value
    /// mappings as well as new key-value mappings. It should also correctly initialize the size to
    /// the number of existing key-value mappings.
    fn new(path: &str) -> std::io::Result<Self>
    where
        Self: Sized;

    /// A function that returns the number of key-value mappings currently stored.
    fn size(self: &Self) -> usize;

    /// A function that inserts a new key-value mapping.
    ///
    /// If there is **no** key-value mapping stored already with the same key, it should return
    /// `Ok(())` if storing is successfully done.
    ///
    /// If there **is** a key-value mapping stored already with the same key, it should return an
    /// [std::io::Error].
    ///
    /// Make sure you read and understand the assignment document regarding how to store key-value
    /// mappings using files as well as how to structure sub-directories.
    ///
    /// Make sure you understand what the trait bounds mean for K and V.
    ///
    /// Refer to [https://docs.serde.rs/serde/](https://docs.serde.rs/serde/)
    /// and [https://serde.rs](https://serde.rs) for serde.
    fn insert<K, V>(self: &mut Self, key: K, value: V) -> std::io::Result<()>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::Serialize + Default + Debug;

    /// A function that returns a previously-inserted value.
    ///
    /// If there **is** a key-value mapping stored already with the same key, it should return
    /// the value.
    ///
    /// If there is **no** key-value mapping stored already with the same key, it should return
    /// an [std::io::Error].
    ///
    /// Make sure you understand what the trait bounds mean for K and V.
    ///
    /// Refer to [https://docs.serde.rs/serde/](https://docs.serde.rs/serde/)
    /// and [https://serde.rs](https://serde.rs) for serde.
    fn lookup<K, V>(self: &Self, key: K) -> std::io::Result<V>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::de::DeserializeOwned + Default + Debug;
    
    /// A function that removes a previously-inserted key-value mapping.
    ///
    /// If there **is** a key-value mapping stored already with the same key, it should return
    /// the value and delete the key-value mapping from the file system.
    ///
    /// If there is **no** key-value mapping stored already with the same key, it should
    /// return an [std::io::Error].
    ///
    /// If a sub-directory does not contain any key-value files, this should delete the
    /// sub-directory as well.
    ///
    /// Make sure you understand what the trait bounds mean for K and V.
    ///
    /// Refer to [https://docs.serde.rs/serde/](https://docs.serde.rs/serde/)
    /// and [https://serde.rs](https://serde.rs) for serde.
    fn remove<K, V>(self: &mut Self, key: K) -> std::io::Result<V>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::de::DeserializeOwned + Default + Debug;
}

fn create_file_path<'a>(path: &String, hashed_value: &'a str, extension: &'a str) -> String {
    format!("{}{}{}{}", path, "/", &hashed_value, extension)
}

fn combine_string<'a>(first: &'a str, second: &'a str) -> String {
    format!("{}{}", first, second)
}

impl Operations for KVStore {
    fn new(path: &str) -> std::io::Result<Self> {
        let path = if path.eq("") { "." } else { path };
        
        fs::create_dir_all(&path)?;
        
        let mut sanitized_path = String::from(path);
        let length = sanitized_path.len();
        let last_char = &sanitized_path[length-1..];  // https://stackoverflow.com/questions/48642342/how-to-get-the-last-character-of-a-str

        // Ensure path ends in "/"
        if !last_char.contains(&String::from("/")){
            sanitized_path = sanitized_path + "/";
        }

        let is_empty = Path::new(path).read_dir()?.next().is_none();

        match is_empty {
            // No existing key-value mappings
            true => { 
                let new_kvstore = KVStore {
                    size: 0,
                    path: sanitized_path,
                };
                Ok(new_kvstore)
            },
            false => {  
                let mut counter = 0;
                for dir in fs::read_dir(path)? {
                    let dir = dir?;

                    let pathname = dir.path();
                    let dir_name = pathname.to_str().unwrap();

                    // Check if it is a directory
                    // https://stackoverflow.com/questions/30309100/how-to-check-if-a-given-path-is-a-file-or-directory
                    if metadata(dir_name).unwrap().is_dir() {
                        for file in fs::read_dir(dir_name)? {      
                            let file = file?;                 
                            let pathname = file.path();            
                            let filename2 = pathname.to_str().unwrap();
                            if filename2.contains(&String::from(".key")) {
                                counter = counter + 1;       
                            }
                        }
                    }
                }
                
                let new_kvstore = KVStore {  
                    size: counter,
                    path: sanitized_path,
                };
                Ok(new_kvstore)
            }
        }
        
    }

    fn size(self: &Self) -> usize {
        self.size
    }

    fn insert<K, V>(self: &mut Self, key: K, value: V) -> std::io::Result<()>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::Serialize + Default + Debug,
    {        
        let serialize_key = serde_json::to_string(&key).unwrap();
        let serialize_value = serde_json::to_string(&value).unwrap();

        let hashed_key = digest(&serialize_key);
        let first_ten_key = &hashed_key[0..10];

        let desired_subdirectory_path = combine_string(&self.path, &first_ten_key);

        if Path::new(&desired_subdirectory_path).exists() {
            let key_file_path = format!("{}{}{}{}", desired_subdirectory_path, "/", &hashed_key, ".key");
            if Path::new(&key_file_path).exists() {
                return Err(Error::new(ErrorKind::AlreadyExists, "There is a key-value mapping stored already with the same key."));
            }
        } else {
            fs::create_dir(&desired_subdirectory_path)?;
        }

        // Create the key and value files
        let key_file_path = create_file_path(&desired_subdirectory_path, &hashed_key, ".key");
        let value_file_path = create_file_path(&desired_subdirectory_path, &hashed_key, ".value");
        fs::write(&key_file_path, serialize_key).expect("Unable to write file");
        fs::write(&value_file_path, serialize_value).expect("Unable to write file");  
        self.size += 1;
        Ok(())
    }

    fn lookup<K, V>(self: &Self, key: K) -> std::io::Result<V>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::de::DeserializeOwned + Default + Debug
    {
        let serialize_key = serde_json::to_string(&key).unwrap();
        let hashed_key = digest(&serialize_key);

        let sub_dir = combine_string(&self.path, &hashed_key[0..10]);

        if Path::new(&sub_dir).exists() {
            let key_file_path = format!("{}{}{}{}", sub_dir, "/", &hashed_key, ".key");

            if Path::new(&key_file_path).exists() {
                let entire_file_path = format!("{}{}{}{}", sub_dir, "/" ,&hashed_key, ".value");
                let contents = fs::read_to_string(entire_file_path)?;
                let deserialize_value = serde_json::from_str(&contents)?;

                return Ok(deserialize_value);
            } else {
                return Err(Error::new(ErrorKind::NotFound, "No key-value mapping exists with this key."));
            }
        } else {
            return Err(Error::new(ErrorKind::NotFound, "No key-value mapping exists with this key."));
        }
    }

    fn remove<K, V>(self: &mut Self, key: K) -> std::io::Result<V>
    where
        K: serde::Serialize + Default + Debug,
        V: serde::de::DeserializeOwned + Default + Debug
    {
        let serialize_key = serde_json::to_string(&key).unwrap();
        let hashed_key = digest(&serialize_key);

        let sub_dir = combine_string(&self.path, &hashed_key[0..10]);

        if Path::new(&sub_dir).exists() {
            let key_file_path = format!("{}{}{}{}", sub_dir, "/", &hashed_key, ".key");

            if Path::new(&key_file_path).exists() {
                fs::remove_file(key_file_path)?;

                let val_file_path = format!("{}{}{}{}", sub_dir, "/" ,&hashed_key, ".value");
                
                let contents = fs::read_to_string(&val_file_path)?;
                let deserialize_value = serde_json::from_str(&contents)?;
                fs::remove_file(val_file_path)?;

                if Path::new(&sub_dir).read_dir()?.next().is_none() {
                    fs::remove_dir_all(sub_dir)?;
                }
                self.size -= 1;
                return Ok(deserialize_value);
            } else {
                return Err(Error::new(ErrorKind::NotFound, "No key-value mapping exists with this key."));
            }
        } else {
            return Err(Error::new(ErrorKind::NotFound, "No key-value mapping exists with this key."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_no_dir() {
        let path = "0";
        rm_rf::ensure_removed(path).unwrap();

        let kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, "0/");
    }

    #[test]
    #[should_panic]
    fn test_new_no_dir_error() {
        KVStore::new("/").unwrap();
    }

    #[test]
    fn test_new_no_existing_dir_and_kvs() {
        let path = "1";
        rm_rf::ensure_removed(path).unwrap();

        fs::create_dir_all(&path).unwrap();

        let kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, "1/");
    }

    #[test]
    fn test_new_existing_dir() {
        let path = "2";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();
        
        kvs.insert(String::from("key"), 1 as i32).unwrap();

        assert_eq!(kvs.size, 1);
        assert_eq!(kvs.path, "2/");

        let kvs2 = KVStore::new(path).unwrap();

        assert_eq!(kvs2.size, 1);
        assert_eq!(kvs2.path, "2/");
    }

    #[test]
    fn test_size_empty() {
        let path = "3";
        rm_rf::ensure_removed(path).unwrap();

        let kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size(), 0);
    }

    #[test]
    fn test_size_nonempty() {
        let path = "4";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();
        kvs.insert(String::from("key"), 1 as i32).unwrap();
        
        assert_eq!(kvs.size(), 1);
    }

    #[test]
    fn test_size_nonempty_lots() {
        let path = "5";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        for i in 0..100 {
            kvs.insert(String::from(i.to_string()), i as i32).unwrap();
        }
        
        assert_eq!(kvs.size(), 100);
    }


    #[test]
    fn test_insert() {
        let path = "6";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), 1 as i32).unwrap();

        assert_eq!(kvs.size, 1);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.remove::<String, i32>(String::from("key")).unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    #[should_panic(expected = r#"There is a key-value mapping stored already with the same key."#)]
    fn test_insert_existing_key() {
        let path = "7";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), 1 as i32).unwrap();
        kvs.insert(String::from("key"), 2 as i32).unwrap();
    }

    #[test]
    fn test_insert_custom_type() {
        #[derive(Serialize, Deserialize, Debug, Default)]
        struct Point {
            x: f32,
            y: f32,
        }

        let point: Point = Point { x: 10.3, y: 0.4 };
        let path = "8";

        rm_rf::ensure_removed(path).unwrap();
        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), point).unwrap();

        assert_eq!(kvs.size, 1);
    }

    #[test]
    fn test_lookup_exists() {
        let path = "9";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), 1 as i32).unwrap();

        assert_eq!(kvs.size, 1);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));
        assert_eq!(kvs.lookup::<String, i32>(String::from("key")).unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = r#"No key-value mapping exists with this key."#)]
    fn test_lookup_does_not_exist() {
        let path = "10";
        rm_rf::ensure_removed(path).unwrap();

        let kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.lookup::<String, i32>(String::from("key")).unwrap();
    }

    #[test]
    #[should_panic(expected = r#"No key-value mapping exists with this key."#)]
    fn test_lookup_removed() {
        let path = "11";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), 1 as i32).unwrap();

        assert_eq!(kvs.size, 1);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.remove::<String, i32>(String::from("key")).unwrap();

        assert_eq!(kvs.size, 0);

        kvs.lookup::<String, i32>(String::from("key")).unwrap();
    }

    #[test]
    fn test_remove_custom_type() {
        #[derive(Serialize, Deserialize, Debug, Default)]
        struct Point {
            x: f32,
            y: f32,
        }

        let point: Point = Point { x: 10.3, y: 0.4 };
        let path = "12";
        rm_rf::ensure_removed(path).unwrap();
        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), point).unwrap();

        assert_eq!(kvs.size, 1);

        kvs.remove::<String, Point>(String::from("key")).unwrap();

        assert_eq!(kvs.size, 0);
    }

    #[test]
    #[should_panic(expected = r#"No key-value mapping exists with this key."#)]
    fn test_remove_does_not_exist() {
        let path = "13";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.remove::<String, i32>(String::from("key")).unwrap();
    }

    #[test]
    fn test_remove_delete_sub_dir() {
        let path = "14";
        rm_rf::ensure_removed(path).unwrap();

        let mut kvs = KVStore::new(path).unwrap();

        assert_eq!(kvs.size, 0);
        assert_eq!(kvs.path, format!("{}{}", path, "/"));

        kvs.insert(String::from("key"), 1 as i32).unwrap();

        // inject a second key into the dir
        let temp_file = "14/c0c1baf2fa/temp.key";
        fs::write(&temp_file, serde_json::to_string("temp").unwrap()).expect("Unable to write file");

        kvs.remove::<String, i32>(String::from("key")).unwrap();

        assert_eq!(Path::new("14/c0c1baf2fa").read_dir().unwrap().next().is_none(), false);
    }
}