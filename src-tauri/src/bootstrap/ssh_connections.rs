use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use crate::contexts::ssh_connections::application::{
    connection_pool::RemoteSshConnectionPool, host_trust::SshHostTrustService,
    SshConnectionApplicationService,
};
use crate::contexts::ssh_connections::{
    RusshSshConnector, SqliteSshConnectionRepository, SshConnectionCredentialAdapter,
    SystemRemoteSshPoolClock, SystemSshConnectionClock, TcpSshConnectionTester,
    UuidSshConnectionIdentity,
};
use crate::contexts::workspaces::domain::{
    REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS, REMOTE_TERMINAL_POOL_CAPACITY,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn assemble_ssh_connections_api(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) -> SshConnectionsApi {
    let repository = Arc::new(SqliteSshConnectionRepository::new(database));
    let credentials = Arc::new(SshConnectionCredentialAdapter::new());
    let clock = Arc::new(SystemSshConnectionClock);
    let host_trust = SshHostTrustService::new(
        repository.clone(),
        clock.clone(),
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory)),
    );
    let connector = Arc::new(RusshSshConnector::new(
        credentials.clone(),
        Arc::new(host_trust.clone()),
    ));
    let pool = RemoteSshConnectionPool::new(
        connector,
        Arc::new(SystemRemoteSshPoolClock),
        REMOTE_TERMINAL_POOL_CAPACITY,
        Duration::from_secs(REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS),
    );
    SshConnectionsApi::new(
        SshConnectionApplicationService::new(
            repository,
            credentials,
            Arc::new(TcpSshConnectionTester),
            clock,
            Arc::new(UuidSshConnectionIdentity),
        ),
        host_trust,
        pool,
    )
}
