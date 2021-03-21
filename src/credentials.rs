use thiserror::Error;

use std::{io, path::Path, path::PathBuf};

type Result<T> = std::result::Result<T, CredentialsError>;

/// Error enum for crate functions. Used for all `Result` returns as the error
/// enum.
#[derive(Debug, Error)]
pub enum CredentialsError {
    /// Describes any errors from std::io::Error
    #[error("Failed to read credentials file")]
    Io(#[from] io::Error),

    /// Describes any errors from rusqlite
    #[error("Failed to parse sqlite file")]
    SqlLite(#[from] rusqlite::Error),

    /// Describes any errors for parsing environmental variables
    #[error("Failed to parse env vars")]
    Env(#[from] std::env::VarError),

    /// Set when no credentials exist
    #[error("No credentials exist")]
    NoCreds,

    /// Set when it's impossible to find your base directory
    #[error("No base directory on this OS. Unable to find default b2 accounts")]
    NoBaseDirs,
}

const KEY_ENV_VAR_NAME: &str = "B2_APPLICATION_KEY";
const KEY_ID_ENV_VAR_NAME: &str = "B2_APPLICATION_KEY_ID";

/// Holds the application key id and application key which make up your
/// credentials
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    /// The application key id
    pub application_key_id: String,

    /// The application key
    pub application_key: String,
}

impl Credentials {
    /// Returns the default credentials for b2. This function will search for b2
    /// credentials in the following order:
    ///
    /// 1. In the B2_APPLICATION_KEY and B2_APPLICATION_KEY_ID environmentals
    ///    variables
    ///
    /// 2. In the sqlite database pointed to by the environmental variable
    ///    B2_ACCOUNT_INFO
    ///
    /// 3. In the default sqlite database ~/.b2_account_info
    ///
    /// If any of those searches run into an unexpected error, that error is
    /// returned. Otherwise `CredentialsError::NoCreds` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let creds = b2creds::Credentials::default().unwrap();
    /// println!("Key ID: {} Key: {}", creds.application_key_id, creds.application_key);
    /// ```
    pub fn default() -> Result<Self> {
        match Self::from_env() {
            Ok(res) => Ok(res),
            Err(_) => Self::from_file(None, None),
        }
    }

    /// Attempts to extract b2 credentials from environmental variables.
    /// Specifically, it will search the B2_APPLICATION_KEY and
    /// B2_APPLICATION_KEY_ID environmentals variables.
    ///
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let creds = b2creds::Credentials::from_env().unwrap();
    /// println!("Key ID: {} Key: {}", creds.application_key_id, creds.application_key);
    /// ```
    pub fn from_env() -> Result<Self> {
        let key = match std::env::var(KEY_ENV_VAR_NAME) {
            Ok(value) => value,
            Err(e) => match e {
                std::env::VarError::NotPresent => return Err(CredentialsError::NoCreds),
                _ => return Err(CredentialsError::Env(e)),
            },
        };
        let key_id = match std::env::var(KEY_ID_ENV_VAR_NAME) {
            Ok(value) => value,
            Err(e) => match e {
                std::env::VarError::NotPresent => return Err(CredentialsError::NoCreds),
                _ => return Err(CredentialsError::Env(e)),
            },
        };

        Ok(Self {
            application_key_id: key_id,
            application_key: key,
        })
    }

    /// Attempts to extract b2 credentials from a b2 account info file. The path
    /// to this file maybe specified via the `db_path` argument. If that argument
    /// is None, the path set in the env variable B2_ACCOUNT_INFO is used, and if
    /// that environmental variable is not set, the path searched defaults to
    /// ~/.b2_account_info.
    ///
    /// The account info file may have multiple accounts stored inside. By
    /// default, the first account is chosen by users may specified by setting
    /// the `account_id` argument.
    ///
    /// # Arguments
    ///
    /// * `db_path` - The (optional) path to the credentials. If not set, it
    ///               defaults to the B2_ACCOUNT_INFO env variable and then ~/.b2_account_info.
    ///
    /// * `account_id` - The ID of the account whose credentials we are querying
    ///                  for. If None, the first account is used.
    ///
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let creds = b2creds::Credentials::from_file(None, None).unwrap();
    /// println!("Key ID: {} Key: {}", creds.application_key_id, creds.application_key);
    /// ```
    pub fn from_file(db_path: Option<&Path>, account_id: Option<&str>) -> Result<Self> {
        let db_path = if let Some(path) = db_path {
            path.to_path_buf()
        } else if let Ok(env_path) = std::env::var("B2_ACCOUNT_INFO") {
            PathBuf::from(env_path)
        } else {
            default_creds_file()?
        };
        Self::from_file_internal(&db_path, account_id)
    }

    fn from_file_internal(db_path: &std::path::Path, account_id: Option<&str>) -> Result<Self> {
        if !db_path.exists() {
            return Err(CredentialsError::NoCreds);
        }

        let conn = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        let mut query = String::from(
            "SELECT account_id, application_key, account_id_or_app_key_id FROM account",
        );
        if let Some(account_id) = account_id {
            query = format!("{} WHERE account_id = \"{}\"", query, account_id);
        }

        let mut stmt = conn.prepare(&query)?;

        let creds_iter = stmt.query_map(rusqlite::NO_PARAMS, |row| {
            Ok(Credentials {
                application_key_id: row.get(2).unwrap(),
                application_key: row.get(1).unwrap(),
            })
        })?;

        let mut creds_iter = creds_iter.filter_map(std::result::Result::ok);

        if let Some(cred) = creds_iter.next() {
            Ok(cred)
        } else {
            Err(CredentialsError::NoCreds)
        }
    }
}

/// Returns the default credentials file path.
/// ```
/// let cred_path = b2creds::default_creds_file().unwrap();
/// println!("B2 Creds Path: {}", cred_path.display());
/// ```
pub fn default_creds_file() -> Result<PathBuf> {
    let home_dir = directories::BaseDirs::new().ok_or(CredentialsError::NoBaseDirs)?;
    Ok(PathBuf::from(home_dir.home_dir()).join(".b2_account_info"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_expected() -> Result<()> {
        let cred_path = default_creds_file()?;
        let home = std::env::var("HOME")?;
        let expected_path = PathBuf::from(home).join(".b2_account_info");
        assert_eq!(cred_path, expected_path);
        Ok(())
    }

    #[test]
    fn from_env_fails_with_no_key_or_key_id() {
        clear_env();
        assert!(matches!(
            Credentials::from_env().unwrap_err(),
            CredentialsError::NoCreds
        ));
    }

    #[test]
    fn from_env_fails_with_no_key() {
        clear_env();
        std::env::set_var(KEY_ID_ENV_VAR_NAME, "keyid");
        assert!(matches!(
            Credentials::from_env().unwrap_err(),
            CredentialsError::NoCreds
        ));
    }
    #[test]
    fn from_env_fails_with_no_key_id() {
        clear_env();
        std::env::set_var(KEY_ENV_VAR_NAME, "key");
        assert!(matches!(
            Credentials::from_env().unwrap_err(),
            CredentialsError::NoCreds
        ));
    }

    #[test]
    fn from_env_works() -> Result<()> {
        clear_env();

        let key_id = "keyid";
        let key = "key";
        std::env::set_var(KEY_ENV_VAR_NAME, key);
        std::env::set_var(KEY_ID_ENV_VAR_NAME, key_id);

        let creds = Credentials::from_env()?;
        assert_eq!(creds.application_key, key);
        assert_eq!(creds.application_key_id, key_id);
        Ok(())
    }

    #[test]
    fn non_existant_path_fails() {
        clear_env();

        let bad_path = PathBuf::from("asgasgasldghuaskdjgkkajsjuugasdgasg");
        let creds = Credentials::from_file(Some(&bad_path), None);
        assert!(matches!(creds.unwrap_err(), CredentialsError::NoCreds));
    }

    #[test]
    fn non_sqlite_path_fails() -> Result<()> {
        clear_env();

        let file = tempfile::NamedTempFile::new()?;
        let creds = Credentials::from_file(Some(file.path()), None);
        assert!(matches!(creds.unwrap_err(), CredentialsError::SqlLite(_)));

        Ok(())
    }

    #[test]
    fn invalid_sqlite_db_fails() -> Result<()> {
        clear_env();

        let file = tempfile::NamedTempFile::new()?;

        let conn = rusqlite::Connection::open(file.path())?;

        conn.execute(
            "CREATE TABLE person (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    data            BLOB
                    )",
            rusqlite::params![],
        )?;
        conn.execute(
            "INSERT INTO person (name, data) VALUES (?1, ?2)",
            rusqlite::params!["Matt".to_string(), 0],
        )?;
        conn.flush_prepared_statement_cache();
        conn.close().unwrap();

        let creds = Credentials::from_file(Some(file.path()), None);
        assert!(matches!(creds.unwrap_err(), CredentialsError::SqlLite(_)));

        Ok(())
    }

    #[test]
    fn valid_sqlite_db_works() -> Result<()> {
        clear_env();

        let account_id = "123";
        let key = "key";
        let key_id = "key_id";

        let file = tempfile::NamedTempFile::new()?;

        let conn = rusqlite::Connection::open(file.path())?;

        conn.execute(
            "CREATE TABLE account (
                    account_id TEXT NOT NULL,
                    application_key TEXT NOT NULL,
                    account_id_or_app_key_id TEXT
                    )",
            rusqlite::params![],
        )?;

        conn.execute(
            "INSERT INTO account (account_id, application_key, account_id_or_app_key_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![account_id, key, key_id],
        )?;
        conn.flush_prepared_statement_cache();
        conn.close().unwrap();

        let creds = Credentials::from_file(Some(file.path()), None);
        assert!(matches!(creds, Ok(_)));
        let creds = creds.unwrap();
        assert_eq!(creds.application_key, key);
        assert_eq!(creds.application_key_id, key_id);

        Ok(())
    }

    #[test]
    fn empty_table_fails() -> Result<()> {
        clear_env();

        let file = tempfile::NamedTempFile::new()?;

        let conn = rusqlite::Connection::open(file.path())?;

        conn.execute(
            "CREATE TABLE account (
                    account_id TEXT NOT NULL,
                    application_key TEXT NOT NULL,
                    account_id_or_app_key_id TEXT
                    )",
            rusqlite::params![],
        )?;
        conn.flush_prepared_statement_cache();
        conn.close().unwrap();

        let creds = Credentials::from_file(Some(file.path()), None);
        assert!(matches!(creds.unwrap_err(), CredentialsError::NoCreds));

        Ok(())
    }

    #[test]
    fn account_id_works() -> Result<()> {
        clear_env();

        let account1_id = "123";
        let account1_key = "key";
        let account1_key_id = "key_id";

        let account2_id = "456";
        let account2_key = "yek";
        let account2_key_id = "id_key";

        let file = tempfile::NamedTempFile::new()?;

        let conn = rusqlite::Connection::open(file.path())?;

        conn.execute(
            "CREATE TABLE account (
                    account_id TEXT NOT NULL,
                    application_key TEXT NOT NULL,
                    account_id_or_app_key_id TEXT
                    )",
            rusqlite::params![],
        )?;

        conn.execute(
            "INSERT INTO account (account_id, application_key, account_id_or_app_key_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![account1_id, account1_key, account1_key_id],
        )?;
        conn.execute(
            "INSERT INTO account (account_id, application_key, account_id_or_app_key_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![account2_id, account2_key, account2_key_id],
        )?;
        conn.flush_prepared_statement_cache();
        conn.close().unwrap();

        let creds = Credentials::from_file(Some(file.path()), Some(account1_id));
        assert!(matches!(creds, Ok(_)));
        let creds = creds.unwrap();
        assert_eq!(creds.application_key, account1_key);
        assert_eq!(creds.application_key_id, account1_key_id);

        let creds = Credentials::from_file(Some(file.path()), Some(account2_id));
        assert!(matches!(creds, Ok(_)));
        let creds = creds.unwrap();
        assert_eq!(creds.application_key, account2_key);
        assert_eq!(creds.application_key_id, account2_key_id);

        let creds = Credentials::from_file(Some(file.path()), Some("DNE"));
        assert!(matches!(creds.unwrap_err(), CredentialsError::NoCreds));

        Ok(())
    }

    fn clear_env() {
        std::env::remove_var(KEY_ID_ENV_VAR_NAME);
        std::env::remove_var(KEY_ENV_VAR_NAME);
    }
}
