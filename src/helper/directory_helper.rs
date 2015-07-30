// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences".to_string()).
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

/// DirectoryHelper provides helper functions to perform Operations on Directory
pub struct DirectoryHelper {
    client: ::std::sync::Arc<::std::sync::Mutex<::maidsafe_client::client::Client>>,
}

impl DirectoryHelper {
    /// Create a new DirectoryHelper instance
    pub fn new(client: ::std::sync::Arc<::std::sync::Mutex<::maidsafe_client::client::Client>>) -> DirectoryHelper {
        DirectoryHelper {
            client: client,
        }
    }

    /// Creates a Directory in the network.
    /// Returns the created DirectoryListing
    pub fn create(&self,
                  directory_name  : String,
                  tag_type        : u64,
                  user_metadata   : Vec<u8>,
                  versioned       : bool,
                  access_level    : ::AccessLevel,
                  parent_directory: Option<&mut ::directory_listing::DirectoryListing>) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        let directory = ::directory_listing::DirectoryListing::new(directory_name,
                                                                   tag_type,
                                                                   user_metadata,
                                                                   versioned,
                                                                   access_level,
                                                                   parent_directory.iter().next().map(|directory| {
                                                                       let key = directory.get_info().get_key();
                                                                       (key.0.clone(), key.1)
                                                                   }));

        let structured_data = try!(self.save_directory_listing(&directory));
        try!(self.client.lock().unwrap().put(structured_data.name(),
                                             ::maidsafe_client::client::Data::StructuredData(structured_data.clone())));

        if let Some(mut parent_directory) = parent_directory {
            try!(parent_directory.upsert_sub_directory(directory.get_info().clone()));
            try!(self.update_directory_listing_and_parent(&parent_directory));
        };

        Ok(directory)
    }

    /// Deletes a sub directory
    pub fn delete(&self,
                  parent_directory   : &mut ::directory_listing::DirectoryListing,
                  directory_to_delete: &String) -> Result<(), ::errors::NfsError> {
            let pos = try!(parent_directory.get_sub_directory_index(directory_to_delete).ok_or(::errors::NfsError::DirectoryNotFound)); {
            parent_directory.get_mut_sub_directories().remove(pos);
            try!(self.update_directory_listing_and_parent(parent_directory));
            Ok(())
        }
    }

    /// Updates an existing DirectoryListing in the network.
    /// Returns the Updated DirectoryListing
    pub fn update(&self, directory: &::directory_listing::DirectoryListing) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        self.update_directory_listing(directory)
    }

    /// Updates an existing DirectoryListing in the network.
    /// Returns the Updated Parent DirectoryListing (if no parent then None is returned)
    pub fn update_directory_listing_and_parent(&self, directory: &::directory_listing::DirectoryListing) -> Result<Option<::directory_listing::DirectoryListing>, ::errors::NfsError> {
        try!(self.update_directory_listing(directory));
        if let Some(parent_dir_key) = directory.get_metadata().get_parent_dir_key() {
            let mut parent_directory = try!(self.get(parent_dir_key, directory.get_metadata().is_versioned(), directory.get_metadata().get_access_level()));
            try!(parent_directory.upsert_sub_directory(directory.get_info().clone()));
            Ok(Some(try!(self.update_directory_listing(&parent_directory))))
        } else {
            Ok(None)
        }
    }

    /// Return the versions of the directory
    pub fn get_versions(&self, directory_key: (&::routing::NameType, u64)) -> Result<Vec<::routing::NameType>, ::errors::NfsError> {
        let structured_data = try!(self.get_structured_data(directory_key.0, ::VERSIONED_DIRECTORY_LISTING_TAG));
        Ok(try!(::maidsafe_client::structured_data_operations::versioned::get_all_versions(&mut *self.client.lock().unwrap(), &structured_data)))
    }

    /// Return the DirectoryListing for the specified version
    pub fn get_by_version(&self,
                          directory_key: (&::routing::NameType, u64),
                          access_level : &::AccessLevel,
                          version      : ::routing::NameType) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
          let immutable_data = try!(self.get_immutable_data(version, ::maidsafe_client::client::ImmutableDataType::Normal));
          ::directory_listing::DirectoryListing::decrypt(self.client.clone(), directory_key.0, access_level, immutable_data.value().clone())
    }

    /// Return the DirectoryListing for the latest version
    pub fn get(&self,
               directory_key: (&::routing::NameType, u64),
               versioned: bool,
               access_level: &::AccessLevel) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        let structured_data = try!(self.get_structured_data(directory_key.0, directory_key.1));
        if versioned {
           let versions = try!(::maidsafe_client::structured_data_operations::versioned::get_all_versions(&mut *self.client.lock().unwrap(), &structured_data));
           let latest_version = versions.last().unwrap();
           self.get_by_version(directory_key, access_level, latest_version.clone())
        } else {
            let private_key = self.client.lock().unwrap().get_public_encryption_key().clone();
            let secret_key = self.client.lock().unwrap().get_secret_encryption_key().clone();
            let nonce = ::directory_listing::DirectoryListing::generate_nonce(directory_key.0);
            let encryption_keys = match *access_level {
                ::AccessLevel::Private => Some((&private_key,
                                                &secret_key,
                                                &nonce)),
                ::AccessLevel::Public => None,
            };
            let structured_data = try!(::maidsafe_client::structured_data_operations::unversioned::get_data(self.client.clone(),
                                                                                                            &structured_data,
                                                                                                            encryption_keys));
            ::directory_listing::DirectoryListing::decrypt(self.client.clone(),
                                                           &directory_key.0,
                                                           access_level,
                                                           structured_data)
        }
    }


    /// Returns the Root Directory
    pub fn get_user_root_directory_listing(&self) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        match self.client.lock().unwrap().get_user_root_directory_id().clone() {
            Some(id) => {
                self.get((id, ::UNVERSIONED_DIRECTORY_LISTING_TAG), false, &::AccessLevel::Private)
            },
            None => {
                let created_directory = try!(self.create(::ROOT_DIRECTORY_NAME.to_string(),
                                                         ::UNVERSIONED_DIRECTORY_LISTING_TAG,
                                                         Vec::new(),
                                                         false,
                                                         ::AccessLevel::Private,
                                                         None));
                try!(self.client.lock().unwrap().set_user_root_directory_id(created_directory.get_key().0.clone()));
                Ok(created_directory)
            }
        }
    }

    /// Returns the Configuration DirectoryListing from the configuration root folder
    /// Creates the directory if the directory does not exists
    #[allow(dead_code)]
    pub fn get_configuration_directory_listing(&self, directory_name: String) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        let mut config_directory_listing = match self.client.lock().unwrap().get_configuration_root_directory_id().clone() {
            Some(id) => try!(self.get((id, ::UNVERSIONED_DIRECTORY_LISTING_TAG), false, &::AccessLevel::Private)),
            None => {
                let created_directory = try!(self.create(::CONFIGURATION_DIRECTORY_NAME.to_string(),
                                                         ::UNVERSIONED_DIRECTORY_LISTING_TAG,
                                                         Vec::new(),
                                                         false,
                                                         ::AccessLevel::Private,
                                                         None));
                try!(self.client.lock().unwrap().set_configuration_root_directory_id(created_directory.get_key().0.clone()));
                created_directory
            }
        };
        match config_directory_listing.get_sub_directories().iter().position(|dir_info| *dir_info.get_name() == directory_name) {
            Some(index) => Ok(try!(self.get(config_directory_listing.get_sub_directories()[index].get_key(),
                                            false,
                                            &::AccessLevel::Private))),
            None => {
                self.create(directory_name, ::UNVERSIONED_DIRECTORY_LISTING_TAG, Vec::new(), false, ::AccessLevel::Private, Some(&mut config_directory_listing))
            },
        }
    }

    fn save_directory_listing(&self, directory: &::directory_listing::DirectoryListing) -> Result<::maidsafe_client::client::StructuredData, ::errors::NfsError> {
        let signing_key = self.client.lock().unwrap().get_secret_signing_key().clone();
        let owner_key = self.client.lock().unwrap().get_public_signing_key().clone();
        let access_level = directory.get_metadata().get_access_level();
        let versioned = directory.get_metadata().is_versioned();
        let encrypted_data = match *access_level {
            ::AccessLevel::Private => try!(directory.encrypt(self.client.clone())),
            ::AccessLevel::Public => try!(::maidsafe_client::utility::serialise(&directory)),
        };
        if versioned {
            let version = try!(self.save_as_immutable_data(encrypted_data,
                                                           ::maidsafe_client::client::ImmutableDataType::Normal));
            Ok(try!(::maidsafe_client::structured_data_operations::versioned::create(&mut *self.client.lock().unwrap(),
                                                                                     version,
                                                                                     ::VERSIONED_DIRECTORY_LISTING_TAG,
                                                                                     directory.get_key().0.clone(),
                                                                                     0,
                                                                                     vec![owner_key],
                                                                                     Vec::new(),
                                                                                     &signing_key)))
        } else {
            let private_key = self.client.lock().unwrap().get_public_encryption_key().clone();
            let secret_key = self.client.lock().unwrap().get_secret_encryption_key().clone();
            let nonce = ::directory_listing::DirectoryListing::generate_nonce(&directory.get_key().0);
            let encryption_keys = match *access_level {
                ::AccessLevel::Private => Some((&private_key,
                                                &secret_key,
                                                &nonce)),
                ::AccessLevel::Public => None,
            };
            Ok(try!(::maidsafe_client::structured_data_operations::unversioned::create(self.client.clone(),
                                                                                       ::UNVERSIONED_DIRECTORY_LISTING_TAG,
                                                                                       directory.get_key().0.clone(),
                                                                                       0,
                                                                                       encrypted_data,
                                                                                       vec![owner_key.clone()],
                                                                                       Vec::new(),
                                                                                       &signing_key,
                                                                                       encryption_keys)))
        }
    }

    fn update_directory_listing(&self, directory: &::directory_listing::DirectoryListing) -> Result<::directory_listing::DirectoryListing, ::errors::NfsError> {
        let directory_key = directory.get_info().get_key();
        let structured_data = try!(self.get_structured_data(&directory_key.0, directory_key.1));

        let signing_key = self.client.lock().unwrap().get_secret_signing_key().clone();
        let owner_key = self.client.lock().unwrap().get_public_signing_key().clone();
        let access_level = directory.get_metadata().get_access_level();
        let versioned = directory.get_metadata().is_versioned();
        let encrypted_data = match *access_level {
            ::AccessLevel::Private => try!(directory.encrypt(self.client.clone())),
            ::AccessLevel::Public => try!(::maidsafe_client::utility::serialise(&directory)),
        };
        let updated_structured_data = if versioned {
            let version = try!(self.save_as_immutable_data(encrypted_data,
                                                           ::maidsafe_client::client::ImmutableDataType::Normal));
            try!(::maidsafe_client::structured_data_operations::versioned::append_version(&mut *self.client.lock().unwrap(),
                                                                                          structured_data,
                                                                                          version,
                                                                                          &signing_key))
        } else {
            let private_key = self.client.lock().unwrap().get_public_encryption_key().clone();
            let secret_key = self.client.lock().unwrap().get_secret_encryption_key().clone();
            let nonce = ::directory_listing::DirectoryListing::generate_nonce(&directory.get_key().0);
            let encryption_keys = match *access_level {
                ::AccessLevel::Private => Some((&private_key,
                                                &secret_key,
                                                &nonce)),
                ::AccessLevel::Public => None,
            };
            try!(::maidsafe_client::structured_data_operations::unversioned::create(self.client.clone(),
                                                                                    ::UNVERSIONED_DIRECTORY_LISTING_TAG,
                                                                                    directory.get_key().0.clone(),
                                                                                    structured_data.get_version() + 1,
                                                                                    encrypted_data,
                                                                                    vec![owner_key.clone()],
                                                                                    Vec::new(),
                                                                                    &signing_key,
                                                                                    encryption_keys))
        };
        try!(self.client.lock().unwrap().post(updated_structured_data.name(),
                                                 ::maidsafe_client::client::Data::StructuredData(updated_structured_data)));
        self.get(directory.get_key(), directory.get_metadata().is_versioned(), access_level)
    }

    /// Saves the data as ImmutableData in the network and returns the name
    fn save_as_immutable_data(&self,
                              data     : Vec<u8>,
                              data_type: ::maidsafe_client::client::ImmutableDataType) -> Result<::routing::NameType, ::errors::NfsError> {
        let immutable_data = ::maidsafe_client::client::ImmutableData::new(data_type, data);
        let name = immutable_data.name();
        try!(self.client.lock().unwrap().put(name.clone(), ::maidsafe_client::client::Data::ImmutableData(immutable_data)));
        Ok(name)
    }

    fn get_structured_data(&self,
                           id      : &::routing::NameType,
                           type_tag: u64) -> Result<::maidsafe_client::client::StructuredData, ::errors::NfsError> {
        let mut response_getter = try!(self.client.lock().unwrap().get(::maidsafe_client::client::StructuredData::compute_name(type_tag, id),
                                                                       ::maidsafe_client::client::DataRequest::StructuredData(type_tag)));
        let data = try!(response_getter.get());
        match data {
            ::maidsafe_client::client::Data::StructuredData(structured_data) => Ok(structured_data),
            _ => Err(::errors::NfsError::from(::maidsafe_client::errors::ClientError::ReceivedUnexpectedData)),
        }
    }

    /// Get ImmutableData from the Network
    fn get_immutable_data(&self,
                          id       : ::routing::NameType,
                          data_type: ::maidsafe_client::client::ImmutableDataType) -> Result<::maidsafe_client::client::ImmutableData, ::errors::NfsError> {
        let mut response_getter = try!(self.client.lock().unwrap().get(id, ::maidsafe_client::client::DataRequest::ImmutableData(data_type)));
        let data = try!(response_getter.get());
        match data {
            ::maidsafe_client::client::Data::ImmutableData(immutable_data) => Ok(immutable_data),
            _ => Err(::errors::NfsError::from(::maidsafe_client::errors::ClientError::ReceivedUnexpectedData)),
        }
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_dir_listing() {
        let test_client = eval_result!(::maidsafe_client::utility::test_utils::get_client());
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(test_client));
        let dir_helper = DirectoryHelper::new(client.clone());
        // Create a Directory
        let directory = eval_result!(dir_helper.create("DirName".to_string(),
                                     ::VERSIONED_DIRECTORY_LISTING_TAG,
                                     None,
                                     true,
                                     ::AccessLevel::Private,
                                     None));
        let fetched = eval_result!(dir_helper.get(directory.get_key(), directory.get_metadata().is_versioned(), directory.get_metadata().get_access_level()));
        assert_eq!(directory, fetched);
        // Create a Child directory and update the parent_directory
        let child_directory = eval_result!(dir_helper.create("Child".to_string(),
                                           ::VERSIONED_DIRECTORY_LISTING_TAG,
                                           None,
                                           true,
                                           ::AccessLevel::Private,
                                           Some(directory.get_info())));
        // Assert whether parent is updated
        let parent = eval_result!(dir_helper.get(directory.get_key(), directory.get_metadata().is_versioned(), directory.get_metadata().get_access_level()));
        assert!(parent.find_sub_directory("Child".to_string()).is_some());
    }

    #[test]
    fn user_root_configuration() {
        let test_client = eval_result!(::maidsafe_client::utility::test_utils::get_client());
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(test_client));
        let dir_helper = DirectoryHelper::new(client.clone());

        let root_dir = eval_result!(dir_helper.get_user_root_directory_listing());
        let created_dir = eval_result!(dir_helper.create("DirName".to_string(),
                                                         ::VERSIONED_DIRECTORY_LISTING_TAG,
                                                         None,
                                                         true,
                                                         ::AccessLevel::Private,
                                                         Some(root_dir.get_info())));
        let root_dir = eval_result!(dir_helper.get_user_root_directory_listing());
        assert!(root_dir.find_sub_directory("DirName".to_string()).is_some());
    }

    #[test]
    fn configuration_directory() {
        let test_client = eval_result!(::maidsafe_client::utility::test_utils::get_client());
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(test_client));
        let dir_helper = DirectoryHelper::new(client.clone());
        let config_dir = eval_result!(dir_helper.get_configuration_directory_listing("DNS".to_string()));
        assert_eq!(config_dir.get_info().get_name().clone(), "DNS".to_string());
        let id = config_dir.get_info().get_key().0.clone();
        let config_dir = eval_result!(dir_helper.get_configuration_directory_listing("DNS".to_string()));
        assert_eq!(config_dir.get_info().get_key().0.clone(), id);
    }


    #[test]
    fn update_and_versioning() {
        let test_client = eval_result!(::maidsafe_client::utility::test_utils::get_client());
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(test_client));
        let dir_helper = DirectoryHelper::new(client.clone());

        let mut dir_listing = eval_result!(dir_helper.create("DirName2".to_string(),
                                                             ::VERSIONED_DIRECTORY_LISTING_TAG,
                                                             None,
                                                             false,
                                                             ::AccessLevel::Private,
                                                             None));

        let mut versions = eval_result!(dir_helper.get_versions(dir_listing.get_key()));
        assert_eq!(versions.len(), 1);

        dir_listing.get_mut_metadata().set_name("NewName".to_string());
        assert!(dir_helper.update(&dir_listing).is_ok());

        versions = eval_result!(dir_helper.get_versions(dir_listing.get_key()));
        assert_eq!(versions.len(), 2);

        let rxd_dir_listing = eval_result!(dir_helper.get_by_version(dir_listing.get_key(), dir_listing.get_metadata().get_access_level(), versions[versions.len()].clone()));
        assert_eq!(rxd_dir_listing, dir_listing);

        let rxd_dir_listing = eval_result!(dir_helper.get_by_version(dir_listing.get_key(), dir_listing.get_metadata().get_access_level(), versions[0].clone()));
        assert_eq!(*rxd_dir_listing.get_metadata().get_name(), "DirName2".to_string());

    }
}
*/