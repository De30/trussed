use core::convert::TryInto;

pub use rand_core::{RngCore, SeedableRng};
use heapless_bytes::Bytes as ByteBuf;
use interchange::Responder;
use littlefs2::path::{Path, PathBuf};
use chacha20::ChaCha8Rng;


use crate::api::*;
use crate::platform::*;
use crate::config::*;
use crate::error::Error;
pub use crate::key::*;
use crate::mechanisms;
use crate::pipe::TrussedInterchange;
use crate::store;
pub use crate::store::{
    filestore::{ClientFilestore, Filestore},
    keystore::{ClientKeystore, Keystore},
};
use crate::types::*;
pub use crate::pipe::ServiceEndpoint;

// #[macro_use]
// mod macros;

macro_rules! rpc_trait { ($($Name:ident, $name:ident,)*) => { $(

    pub trait $Name {
        fn $name(_keystore: &mut impl Keystore, _request: request::$Name)
        -> Result<reply::$Name, Error> { Err(Error::MechanismNotAvailable) }
    }
)* } }

rpc_trait! {
    Agree, agree,
    Decrypt, decrypt,
    DeriveKey, derive_key,
    DeserializeKey, deserialize_key,
    Encrypt, encrypt,
    Exists, exists,
    GenerateKey, generate_key,
    Hash, hash,
    SerializeKey, serialize_key,
    Sign, sign,
    UnsafeInjectKey, unsafe_inject_key,
    UnwrapKey, unwrap_key,
    Verify, verify,
    // TODO: can the default implementation be implemented in terms of Encrypt?
    WrapKey, wrap_key,
}

// associated keys end up namespaced under "/fido2"
// example: "/fido2/keys/2347234"
// let (mut fido_endpoint, mut fido2_client) = Client::new("fido2");
// let (mut piv_endpoint, mut piv_client) = Client::new("piv");

#[derive(Clone)]
struct ReadDirFilesState {
    request: request::ReadDirFilesFirst,
    last: PathBuf,
}

// #[derive(Clone)]
// pub struct ReadDirState {
//     request: request::ReadDirFirst,
//     last: usize,
// }

pub struct ServiceResources<P>
where P: Platform
{
    pub(crate) platform: P,
    // Option?
    currently_serving: ClientId,
    // TODO: how/when to clear
    read_dir_files_state: Option<ReadDirFilesState>,
    read_dir_state: Option<crate::store::filestore::ReadDirState>,
    rng_state: Option<ChaCha8Rng>,
}

impl<P: Platform> ServiceResources<P> {

    pub fn new(platform: P) -> Self {
        Self {
            platform,
            currently_serving: PathBuf::new(),
            read_dir_files_state: None,
            read_dir_state: None,
            rng_state: None,
        }
    }
}

pub struct Service<P> where P: Platform {
    eps: Vec<ServiceEndpoint, MAX_SERVICE_CLIENTS>,
    resources: ServiceResources<P>,
}

// need to be able to send crypto service to an interrupt handler
unsafe impl<P: Platform> Send for Service<P> {}

impl<P: Platform> ServiceResources<P> {

    pub fn reply_to(&mut self, client_id: PathBuf, request: Request) -> Result<Reply, Error> {
        // TODO: what we want to do here is map an enum to a generic type
        // Is there a nicer way to do this?

        let full_store = self.platform.store();

        // prepare keystore, bound to client_id, for cryptographic calls
        let mut keystore: ClientKeystore<'_, P> = ClientKeystore::new(
            client_id.clone(),
            self.drbg().map_err(|_| Error::EntropyMalfunction)?,
            full_store,
        );
        let keystore = &mut keystore;

        // prepare filestore, bound to client_id, for storage calls
        let mut filestore: ClientFilestore<P::S> = ClientFilestore::new(
            client_id.clone(),
            full_store,
        );
        let filestore = &mut filestore;

        match request {
            Request::DummyRequest => {
                Ok(Reply::DummyReply)
            },

            Request::Agree(request) => {
                match request.mechanism {

                    Mechanism::P256 => mechanisms::P256::agree(keystore, request),
                    Mechanism::X255 => mechanisms::X255::agree(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Agree(reply))
            },

            Request::Decrypt(request) => {
                match request.mechanism {

                    Mechanism::Aes256Cbc => mechanisms::Aes256Cbc::decrypt(keystore, request),
                    Mechanism::Chacha8Poly1305 => mechanisms::Chacha8Poly1305::decrypt(keystore, request),
                    Mechanism::Tdes => mechanisms::Tdes::decrypt(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Decrypt(reply))
            },

            Request::DeriveKey(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::derive_key(keystore, request),
                    Mechanism::P256 => mechanisms::P256::derive_key(keystore, request),
                    Mechanism::Sha256 => mechanisms::Sha256::derive_key(keystore, request),
                    Mechanism::X255 => mechanisms::X255::derive_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::DeriveKey(reply))
            },

            Request::DeserializeKey(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::deserialize_key(keystore, request),
                    Mechanism::P256 => mechanisms::P256::deserialize_key(keystore, request),
                    Mechanism::X255 => mechanisms::X255::deserialize_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::DeserializeKey(reply))
            }

            Request::Encrypt(request) => {
                match request.mechanism {

                    Mechanism::Aes256Cbc => mechanisms::Aes256Cbc::encrypt(keystore, request),
                    Mechanism::Chacha8Poly1305 => mechanisms::Chacha8Poly1305::encrypt(keystore, request),
                    Mechanism::Tdes => mechanisms::Tdes::encrypt(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Encrypt(reply))
            },

            Request::Delete(request) => {
                // let success = store::delete_anywhere(&request.key.object_id);
                let key_types = [
                    KeyType::Secret,
                    KeyType::Public,
                ];

                let locations = [
                    StorageLocation::Internal,
                    StorageLocation::External,
                    StorageLocation::Volatile,
                ];

                let success = key_types.iter().any(|key_type| {
                    let path = self.key_path(*key_type, &request.key.object_id);
                    locations.iter().any(|location| {
                        store::delete(self.platform.store(), *location, &path)
                    })
                });

                Ok(Reply::Delete(reply::Delete { success } ))
            },

            Request::Exists(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::exists(keystore, request),
                    Mechanism::P256 => mechanisms::P256::exists(keystore, request),
                    Mechanism::Totp => mechanisms::Totp::exists(keystore, request),
                    Mechanism::X255 => mechanisms::X255::exists(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Exists(reply))
            },

            Request::GenerateKey(request) => {
                match request.mechanism {
                    Mechanism::Chacha8Poly1305 => mechanisms::Chacha8Poly1305::generate_key(keystore, request),
                    Mechanism::Ed255 => mechanisms::Ed255::generate_key(keystore, request),
                    Mechanism::HmacSha256 => mechanisms::HmacSha256::generate_key(keystore, request),
                    Mechanism::P256 => mechanisms::P256::generate_key(keystore, request),
                    Mechanism::X255 => mechanisms::X255::generate_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),
                }.map(|reply| Reply::GenerateKey(reply))
            },

            Request::UnsafeInjectKey(request) => {
                match request.mechanism {
                    Mechanism::Tdes => mechanisms::Tdes::unsafe_inject_key(keystore, request),
                    Mechanism::Totp => mechanisms::Totp::unsafe_inject_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),
                }.map(|reply| Reply::UnsafeInjectKey(reply))
            },

            Request::Hash(request) => {
                match request.mechanism {

                    Mechanism::Sha256 => mechanisms::Sha256::hash(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Hash(reply))
            },

            Request::LocateFile(request) => {

                let user_dir = match request.dir {
                    Some(dir) => dir,
                    None => PathBuf::from(b"/"),
                };
                let base_path = self.dataspace_path(&user_dir);
                let base_path = self.namespace_path(&base_path);
                info_now!("base path {:?}", &base_path);

                fn recursively_locate<S: 'static + crate::types::LfsStorage>(
                    fs: &'static crate::store::Fs<S>,
                    path: PathBuf,
                    filename: &Path
                )
                    -> Result<Option<PathBuf>, littlefs2::io::Error>
                {
                    // info_now!("entering `rec-loc` with path {:?} and filename {:?}",
                              // &path, filename);
                    // let fs = store.vfs();
                    fs.read_dir_and_then(&path, |dir| {
                        // info_now!("looking in {:?}", &path).ok();
                        for (i, entry) in dir.enumerate() {
                            let entry = entry.unwrap();
                            let mut is_special_dir = PathBuf::from(entry.file_name()) == PathBuf::from(b".");
                            is_special_dir |= PathBuf::from(entry.file_name()) == PathBuf::from(b"..");
                            if (i < 2) != is_special_dir {
                                // info_now!("i = {}, is_special_dir = {:?}", i, is_special_dir).ok();
                                panic!("i = {}, is_special_dir = {:?}, filename = {:?}",
                                    i,
                                    is_special_dir,
                                    entry.file_name(),
                                );

                            }
                            if i < 2 {
                                // info_now!(":: skipping {:?}", &entry.path()).ok();
                                continue;
                            }
                            if entry.file_type().is_file() {
                                // info_now!(":: comparing with {:?}", entry.file_name()).ok();
                                if PathBuf::from(entry.file_name()) == PathBuf::from(filename) {
                                    info_now!("found it");
                                    return Ok(Some(PathBuf::from(entry.path())));
                                }
                                continue;
                            }
                            if entry.file_type().is_dir() {
                                // info_now!("recursing into {:?} with path {:?}",
                                //           &entry.file_name(),
                                //           &entry.path(),
                                //           ).ok();
                                if let Some(path) = recursively_locate(fs, PathBuf::from(entry.path()), filename)? {
                                    return Ok(Some(path));
                                };
                            }
                        }
                        Ok(None)
                    })
                }

                assert!(request.location == StorageLocation::Internal);
                let path = recursively_locate(self.platform.store().ifs(), base_path, &request.filename).unwrap();
                let path = match path.as_ref() {
                    Some(path) => Some(self.denamedataspace_path(path)),
                    None => None,
                };
                    // .map_err(|_| Error::InternalError)?;

                Ok(Reply::LocateFile(reply::LocateFile { path }) )

            }

            // This is now preferably done using littlefs-fuse (when device is not yet locked),
            // and should be removed from firmware completely
            Request::DebugDumpStore(_request) => {

                info_now!(":: PERSISTENT");
                recursively_list(self.platform.store().ifs(), PathBuf::from(b"/"));

                info_now!(":: VOLATILE");
                recursively_list(self.platform.store().vfs(), PathBuf::from(b"/"));

                fn recursively_list<S: 'static + crate::types::LfsStorage>(fs: &'static crate::store::Fs<S>, path: PathBuf) {
                    // let fs = store.vfs();
                    fs.read_dir_and_then(&path, |dir| {
                        for (i, entry) in dir.enumerate() {
                            let entry = entry.unwrap();
                            if i < 2 {
                                // info_now!("skipping {:?}", &entry.path()).ok();
                                continue;
                            }
                            info_now!("{:?} p({:?})", entry.path(), &path);
                            if entry.file_type().is_dir() {
                                recursively_list(fs, PathBuf::from(entry.path()));
                            }
                            if entry.file_type().is_file() {
                                let _contents: Vec<u8, consts::U256> = fs.read(entry.path()).unwrap();
                                // info_now!("{} ?= {}", entry.metadata().len(), contents.len()).ok();
                                // info_now!("{:?}", &contents).ok();
                            }
                        }
                        Ok(())
                    }).unwrap();
                }

                Ok(Reply::DebugDumpStore(reply::DebugDumpStore {}) )

            }

            Request::ReadDirFirst(request) => {
                let maybe_entry = match filestore.read_dir_first(&request.dir, request.location, request.not_before_filename.as_ref())? {
                    Some((entry, read_dir_state)) => {
                        self.read_dir_state = Some(read_dir_state);
                        Some(entry)
                    }
                    None => {
                        self.read_dir_state = None;
                        None

                    }
                };
                Ok(Reply::ReadDirFirst(reply::ReadDirFirst { entry: maybe_entry } ))
            }

            Request::ReadDirNext(_request) => {
                // ensure next call has nothing to work with, unless we store state again
                let read_dir_state = self.read_dir_state.take();

                let maybe_entry = match read_dir_state {
                    None => None,
                    Some(state) => {
                        match filestore.read_dir_next(state)? {
                            Some((entry, read_dir_state)) => {
                                self.read_dir_state = Some(read_dir_state);
                                Some(entry)
                            }
                            None => {
                                self.read_dir_state = None;
                                None
                            }
                        }
                    }
                };

                Ok(Reply::ReadDirNext(reply::ReadDirNext { entry: maybe_entry } ))
            }

            Request::ReadDirFilesFirst(request) => {
                assert!(request.location == StorageLocation::Internal);

                // let path = self.namespace_path(&request.dir);
                let path = self.dataspace_path(&request.dir);
                let path = self.namespace_path(&path);

                let fs = self.platform.store().ifs();

                let result = fs.read_dir_and_then(&path, |dir| {
                    for entry in dir {
                        // let entry = entry?;//.map_err(|_| Error::InternalError)?;
                        let entry = entry.unwrap();
                        if entry.file_type().is_dir() {
                            continue;
                        }

                        let name = entry.file_name();

                        if let Some(user_attribute) = request.user_attribute.as_ref() {
                            let mut path = path.clone();
                            path.push(name);
                            let attribute = fs.attribute(&path, crate::config::USER_ATTRIBUTE_NUMBER)
                                .map_err(|_e| {
                                    info!("error getting attribute: {:?}", &_e);
                                    littlefs2::io::Error::Io
                                }
                            )?;

                            match attribute {
                                None => continue,
                                Some(attribute) => {
                                    if user_attribute != attribute.data() {
                                        continue;
                                    }
                                }
                            }
                        }

                        return Ok(entry);
                    }

                    Err(littlefs2::io::Error::NoSuchEntry)
                });
                let entry = if result.is_err() {
                    let err = result.err().unwrap();
                    info_now!("read_dir error: {:?}", &err);
                    return match err {
                        // Return no data if path is invalid
                        littlefs2::io::Error::NoSuchEntry =>
                            Ok(Reply::ReadDirFilesFirst(reply::ReadDirFilesFirst {
                                data: None,
                            } )),

                        _ => Err(Error::InternalError),
                    };
                } else {
                    result.unwrap()
                };

                let data = store::read(self.platform.store(), request.location, entry.path())?;

                self.read_dir_files_state = Some(ReadDirFilesState {
                    request,
                    last: entry.file_name().into(),
                });

                Ok(Reply::ReadDirFilesFirst(reply::ReadDirFilesFirst {
                    data: Some(data),
                } ))
            }

            Request::ReadDirFilesNext(_request) => {
                // TODO: ergonooomics

                let ReadDirFilesState { request, last } = match &self.read_dir_files_state {
                    Some(state) => state.clone(),
                    None => panic!("call ReadDirFilesFirst before ReadDirFilesNext"),
                };

                // let path = self.namespace_path(&request.dir);
                let path = self.dataspace_path(&request.dir);
                let path = self.namespace_path(&path);
                let fs = self.platform.store().ifs();

                let mut found_last = false;
                let entry = fs.read_dir_and_then(&path, |dir| {
                    for entry in dir {

                        let entry = entry.unwrap();

                        if entry.file_type().is_dir() {
                            continue;
                        }

                        let name = entry.file_name();

                        if !found_last {
                            let name: PathBuf = name.into();
                            // info_now!("comparing {:} with last {:?}", &name, &last).ok();
                            // TODO: This failed when all bytes (including trailing null) were
                            // compared. It turned out that `last` had a trailing 240 instead.
                            if last == name {
                                found_last = true;
                                // info_now!("found last").ok();
                            }
                            continue;
                        }

                        // info_now!("next file found: {:?}", name.as_ref()).ok();

                        if let Some(user_attribute) = request.user_attribute.as_ref() {
                            let mut path = path.clone();
                            path.push(name);
                            let attribute = fs.attribute(&path, crate::config::USER_ATTRIBUTE_NUMBER)
                                .map_err(|_e| {
                                    info!("error getting attribute: {:?}", &_e);
                                    littlefs2::io::Error::Io
                                }
                            )?;

                            match attribute {
                                None => continue,
                                Some(attribute) => {
                                    if user_attribute != attribute.data() {
                                        continue;
                                    }
                                }
                            }
                        }

                        return Ok(entry);
                    }

                    Err(littlefs2::io::Error::NoSuchEntry)

                });

                let data = match entry {
                    Err(littlefs2::io::Error::NoSuchEntry) => None,
                    Ok(entry) => {
                        let data = store::read(self.platform.store(), request.location, entry.path())?;

                        self.read_dir_files_state = Some(ReadDirFilesState {
                            request,
                            last: entry.file_name().into(),
                        });

                        Some(data)

                    }
                    Err(_) => return Err(Error::InternalError),
                };

                Ok(Reply::ReadDirFilesNext(reply::ReadDirFilesNext {
                    data,
                } ))
            }

            Request::RemoveDir(request) => {
                filestore.remove_dir(&request.path, &request.location)?;
                Ok(Reply::RemoveDir(reply::RemoveDir {} ))
            }

            Request::RemoveFile(request) => {
                filestore.remove_file(&request.path, &request.location)?;
                Ok(Reply::RemoveFile(reply::RemoveFile {} ))
            }

            Request::ReadFile(request) => {
                Ok(Reply::ReadFile(reply::ReadFile {
                    data: filestore.read(&request.path, &request.location)?
                }))
            }

            Request::RandomByteBuf(request) => {
                if request.count < 1024 {
                    let mut bytes = Message::new();
                    bytes.resize_default(request.count).unwrap();
                    self.drbg()?.fill_bytes(&mut bytes);
                    Ok(Reply::RandomByteBuf(reply::RandomByteBuf { bytes } ))
                } else {
                    Err(Error::MechanismNotAvailable)
                }
            }

            Request::SerializeKey(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::serialize_key(keystore, request),
                    Mechanism::P256 => mechanisms::P256::serialize_key(keystore, request),
                    Mechanism::X255 => mechanisms::X255::serialize_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::SerializeKey(reply))
            }

            Request::Sign(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::sign(keystore, request),
                    Mechanism::HmacSha256 => mechanisms::HmacSha256::sign(keystore, request),
                    Mechanism::P256 => mechanisms::P256::sign(keystore, request),
                    Mechanism::P256Prehashed => mechanisms::P256Prehashed::sign(keystore, request),
                    Mechanism::Totp => mechanisms::Totp::sign(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Sign(reply))
            },

            Request::WriteFile(request) => {
                filestore.write(&request.path, &request.location, &request.data)?;
                Ok(Reply::WriteFile(reply::WriteFile {} ))
            }

            Request::UnwrapKey(request) => {
                match request.mechanism {

                    Mechanism::Chacha8Poly1305 => mechanisms::Chacha8Poly1305::unwrap_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::UnwrapKey(reply))
            }

            Request::Verify(request) => {
                match request.mechanism {

                    Mechanism::Ed255 => mechanisms::Ed255::verify(keystore, request),
                    Mechanism::P256 => mechanisms::P256::verify(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::Verify(reply))
            },

            Request::WrapKey(request) => {
                match request.mechanism {

                    Mechanism::Aes256Cbc => mechanisms::Aes256Cbc::wrap_key(keystore, request),
                    Mechanism::Chacha8Poly1305 => mechanisms::Chacha8Poly1305::wrap_key(keystore, request),
                    _ => Err(Error::MechanismNotAvailable),

                }.map(|reply| Reply::WrapKey(reply))
            },

            Request::RequestUserConsent(request) => {
                assert_eq!(request.level, consent::Level::Normal);

                let starttime = self.platform.user_interface().uptime();
                let timeout = core::time::Duration::from_millis(request.timeout_milliseconds as u64);

                self.platform.user_interface().set_status(ui::Status::WaitingForUserPresence);
                loop {
                    let nowtime = self.platform.user_interface().uptime();
                    if (nowtime - starttime) > timeout {
                        let result = Err(consent::Error::TimedOut);
                        return Ok(Reply::RequestUserConsent(reply::RequestUserConsent { result } ));
                    }
                    let up = self.platform.user_interface().check_user_presence();
                    match request.level {
                        // If Normal level consent is request, then both Strong and Normal
                        // indications will result in success.
                        consent::Level::Normal => {
                            if up == consent::Level::Normal ||
                                up == consent::Level::Strong {
                                    break;
                                }
                        },
                        // Otherwise, only strong level indication will work.
                        consent::Level::Strong => {
                            if up == consent::Level::Strong {
                                break;
                            }
                        }
                        _ => {
                            break;
                        }
                    }
                }
                self.platform.user_interface().set_status(ui::Status::Idle);

                let result = Ok(());
                Ok(Reply::RequestUserConsent(reply::RequestUserConsent { result } ))
            }

            Request::Reboot(request) => {
                self.platform.user_interface().reboot(request.to);
            }

            _ => {
                // #[cfg(test)]
                // println!("todo: {:?} request!", &request);
                Err(Error::RequestNotAvailable)
            },
        }
    }

    // This and the following method are here, because ServiceResources knows
    // the current "client", while Store does not
    //
    // TODO: This seems like a design problem
    pub fn namespace_path(&self, path: &Path) -> PathBuf {
        // TODO: check no escapes!
        let mut namespaced_path = PathBuf::new();
        namespaced_path.push(&self.currently_serving);
        namespaced_path.push(path);
        namespaced_path
    }

    pub fn dataspace_path(&self, path: &Path) -> PathBuf {
        // TODO: check no escapes!
        let mut dataspaced_path = PathBuf::new();
        dataspaced_path.push(b"dat\0".try_into().unwrap());
        dataspaced_path.push(path);
        dataspaced_path
    }

    pub fn denamespace_path(&self, path: &Path) -> PathBuf {
        // info_now!("denamespacing {:?}", path).ok();
        let bytes = path.as_ref().as_bytes();
        let absolute = bytes[0] == b'/';
        let offset = if absolute { 1 } else { 0 };

        let end_of_namespace = bytes[1..].iter().position(|&x| x == b'/')
            // oh oh oh
            .unwrap();
        let buf = PathBuf::from(&bytes[end_of_namespace + 1 + offset..]);
        // info_now!("buf out: {:?}", &buf).ok();
        buf
    }

    pub fn dedataspace_path(&self, path: &Path) -> PathBuf {
        // info_now!("dedataspacing {:?}", path).ok();
        let bytes = path.as_ref().as_bytes();
        let absolute = bytes[0] == b'/';
        let offset = if absolute { 1 } else { 0 };

        let end_of_dataspace = bytes[1..].iter().position(|&x| x == b'/')
            // oh oh oh
            .unwrap();
        let buf = PathBuf::from(&bytes[end_of_dataspace + 1 + offset..]);
        // info_now!("buf out: {:?}", &buf).ok();
        buf
    }

    pub fn denamedataspace_path(&self, path: &Path) -> PathBuf {
        self.dedataspace_path(&self.denamespace_path(path))
    }

    pub fn key_path(&self, key_type: KeyType, key_id: &UniqueId) -> PathBuf {
        let mut path = PathBuf::new();
        // TODO: huh?!?!
        // If I change these prefixes to shorter,
        // DebugDumpStore skips the directory contents
        path.push(match key_type {
            KeyType::Public => b"pub\0".try_into().unwrap(),
            KeyType::Secret => b"sec\0".try_into().unwrap(),
        });
        path.push(&PathBuf::from(key_id.hex().as_ref()));
        // no dataspacing
        self.namespace_path(&path)
    }

    pub fn generate_unique_id(&mut self) -> Result<UniqueId, Error> {
        let mut unique_id = [0u8; 16];

        self.fill_random_bytes(&mut unique_id)?;

        // #[cfg(all(test, feature = "verbose-tests"))]
        // println!("unique id {:?}", &unique_id);
        Ok(UniqueId(unique_id))
    }

    pub fn drbg(&mut self) -> Result<&mut ChaCha8Rng, Error> {

        // Check if our RNG is loaded.
        if self.rng_state.is_none() {

            let path = PathBuf::from(b"rng-state.bin");

            // If it hasn't been saved to flash yet, generate it from HW RNG.
            let stored_seed = if ! path.exists(&self.platform.store().ifs()) {
                let mut stored_seed = [0u8; 32];
                self.platform.rng().try_fill_bytes(&mut stored_seed)
                    .map_err(|_| Error::EntropyMalfunction)?;
                stored_seed
            } else {
                // Use the last saved state.
                let stored_seed_bytebuf: ByteBuf<consts::U32> = store::read(self.platform.store(), StorageLocation::Internal, &path)?;
                let mut stored_seed = [0u8; 32];
                stored_seed.clone_from_slice(&stored_seed_bytebuf);
                stored_seed
            };

            // Generally, the TRNG is fed through a DRBG to whiten its output.
            //
            // In principal seeding a DRBG like Chacha8Rng from "good" HW/external entropy
            // should be good enough for the lifetime of the key.
            //
            // Since we have a TRNG though, we might as well mix in some new entropy
            // on each boot. We do not do so on each DRBG draw to avoid excessive flash writes.
            // (e.g., if some app exposes unlimited "read-entropy" functionality to users).
            //
            // Additionally, we use a twist on the ideas of Haskell's splittable RNGs, and store
            // the hash of the current seed as input seed for the next boot. In this way, even if
            // the HW entropy "goes bad" (e.g., starts returning all zeros), the properties of the
            // hash function should ensure that there are no cycles or repeats of entropy in the
            // output to apps.

            // 1. First, draw fresh entropy from the HW TRNG.
            let mut entropy = [0u8; 32];
            self.platform.rng().try_fill_bytes(&mut entropy)
                .map_err(|_| Error::EntropyMalfunction)?;

            // 2. Mix into our previously stored seed.
            let mut our_seed = [0u8; 32];
            for i in 0..32 {
                our_seed[i] = stored_seed[i] ^ entropy[i];
            }

            // Initialize ChaCha8 construction with our seed.
            self.rng_state = Some(chacha20::ChaCha8Rng::from_seed(our_seed));

            // 3. Store hash of seed for next boot.
            use sha2::digest::Digest;
            let mut hash = sha2::Sha256::new();
            hash.input(&our_seed);
            let seed_to_store = hash.result();

            store::store(self.platform.store(), StorageLocation::Internal, &path, seed_to_store.as_ref()).unwrap();
        }

        // no panic - just ensured existence
        let chacha = self.rng_state.as_mut().unwrap();
        Ok(chacha)
    }

    pub fn fill_random_bytes(&mut self, bytes: &mut[u8]) -> Result<(), Error> {
        Ok(self.drbg()?.fill_bytes(bytes))
    }

}

impl<P: Platform> Service<P> {

    pub fn new(platform: P) -> Self {
        let resources = ServiceResources::new(platform);
        Self { eps: Vec::new(), resources }
    }

    /// Add a new client, claiming one of the statically configured
    /// interchange pairs.
    pub fn try_new_client<S: crate::platform::Syscall>(&mut self, client_id: &str, syscall: S)
        -> Result<crate::client::ClientImplementation<S>, ()>
    {
        use interchange::Interchange;
        let (requester, responder) = TrussedInterchange::claim().ok_or(())?;
        let client_id = ClientId::from(client_id.as_bytes());
        self.add_endpoint(responder, client_id).map_err(|_service_endpoint| ())?;

        Ok(crate::client::ClientImplementation::new(requester, syscall))
    }

    /// Specialization of `try_new_client`, using `self`'s implementation of `Syscall`
    /// (directly call self for processing). This method is only useful for single-threaded
    /// single-app runners.
    pub fn try_as_new_client(&mut self, client_id: &str)
        -> Result<crate::client::ClientImplementation<&mut Service<P>>, ()>
    {
        use interchange::Interchange;
        let (requester, responder) = TrussedInterchange::claim().ok_or(())?;
        let client_id = ClientId::from(client_id.as_bytes());
        self.add_endpoint(responder, client_id).map_err(|_service_endpoint| ())?;

        Ok(crate::client::ClientImplementation::new(requester, self))
    }


    pub fn add_endpoint(&mut self, interchange: Responder<TrussedInterchange>, client_id: ClientId) -> Result<(), ServiceEndpoint> {
        self.eps.push(ServiceEndpoint { interchange, client_id })
    }

    pub fn set_seed_if_uninitialized(&mut self, seed: &[u8; 32]) {

        let path = PathBuf::from(b"rng-state.bin");
        if ! path.exists(&self.resources.platform.store().ifs()) {
            store::store(self.resources.platform.store(), StorageLocation::Internal, &path, &seed[..]).unwrap();
        }

    }

    // currently, this just blinks the green heartbeat LED (former toggle_red in app_rtic.rs)
    //
    // in future, this would
    // - generate more interesting LED visuals
    // - return "when" next to be called
    // - potentially read out button status and return "async"
    pub fn update_ui(&mut self) /* -> u32 */ {
        self.resources.platform.user_interface().refresh();
    }

    // process one request per client which has any
    pub fn process(&mut self) {
        // split self since we iter-mut over eps and need &mut of the other resources
        let eps = &mut self.eps;
        let resources = &mut self.resources;

        for ep in eps.iter_mut() {
            if let Some(request) = ep.interchange.take_request() {
                // #[cfg(test)] println!("service got request: {:?}", &request);

                resources.currently_serving = ep.client_id.clone();
                let reply_result = resources.reply_to(ep.client_id.clone(), request);
                ep.interchange.respond(reply_result).ok();

            }
        }
        debug_now!("I/E/V : {}/{}/{} >",
              self.resources.platform.store().ifs().available_blocks().unwrap(),
              self.resources.platform.store().efs().available_blocks().unwrap(),
              self.resources.platform.store().vfs().available_blocks().unwrap(),
        );
    }
}

impl<P> crate::client::Syscall for &mut Service<P>
where P: Platform
{
    fn syscall(&mut self) {
        self.process();
    }
}
