use crate::contexts::ssh_connections::api::SshConnectionsApi;
use crate::contexts::ssh_connections::application::{
    SshConnectionApplicationService, UuidSshConnectionIdentity,
};
use crate::contexts::ssh_connections::{
    SqliteSshConnectionRepository, SshConnectionCredentialAdapter, SystemSshConnectionClock,
    TcpSshConnectionTester,
};
use crate::platform::database::NativeDatabase;
use std::sync::Arc;

pub(crate) fn assemble_ssh_connections_api(database: NativeDatabase) -> SshConnectionsApi {
    SshConnectionsApi::new(SshConnectionApplicationService::new(
        Arc::new(SqliteSshConnectionRepository::new(database)),
        Arc::new(SshConnectionCredentialAdapter::new()),
        Arc::new(TcpSshConnectionTester),
        Arc::new(SystemSshConnectionClock),
        Arc::new(UuidSshConnectionIdentity),
    ))
}
