use crate::contexts::retrieval::application::{
    RetrievalConfiguration, RetrievalConfigurationRepository,
};
use crate::contexts::retrieval::domain::RetrievalError;
use crate::platform::clock::SystemClock;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, OptionalExtension};

// Task 12 的 bootstrap 装配会构造它并把它交给 RetrievalApi；届时移除本属性。
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct SqliteRetrievalConfigurationRepository {
    database: NativeDatabase,
}

// 同上，随 SqliteRetrievalConfigurationRepository 一起在 Task 12 移除。
#[allow(dead_code)]
impl SqliteRetrievalConfigurationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl RetrievalConfigurationRepository for SqliteRetrievalConfigurationRepository {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let configuration = connection
            .query_row(
                "SELECT source_profile_id, embedding_model FROM retrieval_configuration WHERE id = 1",
                [],
                |row| {
                    Ok(RetrievalConfiguration {
                        source_profile_id: row.get(0)?,
                        embedding_model: row.get(1)?,
                    })
                },
            )
            // 未配置就是零行，不是错误——"还没配置"是正常状态，不该让调用方去区分
            // "查询失败" 和 "本来就没有"。
            .optional()
            .map_err(storage_error)?;
        Ok(configuration.unwrap_or_default())
    }

    fn save(&self, profile_id: &str, embedding_model: &str) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                r#"
                INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
                VALUES (1, ?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    source_profile_id = excluded.source_profile_id,
                    embedding_model = excluded.embedding_model,
                    updated_at = excluded.updated_at
                "#,
                params![profile_id, embedding_model, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

// 被本文件的 load/save 调用，但两者都要到 Task 9（检索服务读配置）和 Task 12（api 读写配置）
// 才在生产代码路径上可达；届时移除本属性。
#[allow(dead_code)]
fn database_error(error: DatabaseError) -> RetrievalError {
    match error {
        DatabaseError::Database(error) => storage_error(error),
        DatabaseError::Storage(message) => RetrievalError::Storage(message),
    }
}

// 同上，随 database_error 一起在 Task 9/12 移除。
#[allow(dead_code)]
fn storage_error(error: rusqlite::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn fixture(label: &str) -> (TempDirectory, SqliteRetrievalConfigurationRepository) {
        let directory = TempDirectory::new(label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (directory, SqliteRetrievalConfigurationRepository::new(database))
    }

    #[test]
    fn an_unconfigured_database_loads_an_empty_configuration() {
        let (_directory, repository) = fixture("retrieval config empty");
        let configuration = repository.load().expect("load");
        assert_eq!(configuration, RetrievalConfiguration::default());
        assert_eq!(configuration.resolved_model(), None);
    }

    #[test]
    fn saving_twice_updates_the_single_row_instead_of_failing() {
        let (_directory, repository) = fixture("retrieval config overwrite");
        repository.save("profile-a", "model-a").expect("first save");
        repository.save("profile-b", "model-b").expect("second save");

        let configuration = repository.load().expect("load");
        assert_eq!(configuration.resolved_model(), Some(("profile-b", "model-b")));
    }

    #[test]
    fn a_configuration_missing_either_half_is_not_resolved() {
        assert_eq!(
            RetrievalConfiguration {
                source_profile_id: Some("p".to_string()),
                embedding_model: None,
            }
            .resolved_model(),
            None
        );
        assert_eq!(
            RetrievalConfiguration {
                source_profile_id: None,
                embedding_model: Some("m".to_string()),
            }
            .resolved_model(),
            None
        );
    }
}
