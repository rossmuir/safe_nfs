// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
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


/// NFS Errors
pub enum NfsError {
    AlreadyExists,
    ClientError(::maidsafe_client::errors::ClientError),
    DestinationAndSourceAreSame,
    DirectoryNotFound,
    FailedToUpdateDirectory,
    FailedToUpdateFile,
    FileExistsInDestination,
    FileNotFound,
    InvalidRangeSpecified,
    MetadataIsEmpty,
    MetaDataMissingOrCorrupted,
    NameIsEmpty,
    NotFound,
}

impl From<::maidsafe_client::errors::ClientError> for NfsError {
    fn from(error: ::maidsafe_client::errors::ClientError) -> NfsError {
        NfsError::ClientError(error)
    }
}

// TODO refactor as in maidsafe_dns
impl ::std::fmt::Debug for NfsError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            NfsError::AlreadyExists                 => ::std::fmt::Display::fmt("NfsError::AlreadyExists", f),
            NfsError::ClientError(_)                => ::std::fmt::Display::fmt("NfsError::ClientError", f), // TODO Improve these containing nested stuff to print as well
            NfsError::DestinationAndSourceAreSame   => ::std::fmt::Display::fmt("NfsError::DestinationAndSourceAreSame", f),
            NfsError::DirectoryNotFound             => ::std::fmt::Display::fmt("NfsError::DirectoryNotFound", f),
            NfsError::FailedToUpdateDirectory       => ::std::fmt::Display::fmt("NfsError::FailedToUpdateDirectory", f),
            NfsError::FailedToUpdateFile            => ::std::fmt::Display::fmt("NfsError::FailedToUpdateFile", f),
            NfsError::FileExistsInDestination       => ::std::fmt::Display::fmt("NfsError::FileExistsInDestination", f),
            NfsError::FileNotFound                  => ::std::fmt::Display::fmt("NfsError::FileNotFound", f),
            NfsError::InvalidRangeSpecified         => ::std::fmt::Display::fmt("NfsError::InvalidRangeSpecified", f),
            NfsError::MetadataIsEmpty               => ::std::fmt::Display::fmt("NfsError::MetadataIsEmpty", f),
            NfsError::MetaDataMissingOrCorrupted    => ::std::fmt::Display::fmt("NfsError::MetaDataMissingOrCorrupted", f),
            NfsError::NameIsEmpty                   => ::std::fmt::Display::fmt("NfsError::NameIsEmpty", f),
            NfsError::NotFound                      => ::std::fmt::Display::fmt("NfsError::NotFound", f),
        }
    }
}