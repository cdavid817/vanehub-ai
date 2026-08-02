use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::communications::api::{CommunicationsApi, WeChatAuthorizationApi};
use crate::contexts::communications::application::{
    CommunicationsApplicationPorts, CommunicationsApplicationService, CommunicationsClockPort,
    CommunicationsLoggingPort,
};
use crate::contexts::communications::infrastructure::{
    BusyMessageProvider, CommunicationsAgentExecutionAdapter, CommunicationsCredentialAdapter,
    CommunicationsInboundBridge, CommunicationsLoggingAdapter, CommunicationsOperationAdapter,
    CommunicationsSessionBindingAdapter, CommunicationsTransportAdapter, ConnectorRuntimeManager,
    SqliteCommunicationsRepository, SystemCommunicationsClock,
};
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::desktop::domain::NativeCopy;
use crate::contexts::operations::api::{DiagnosticLogPort, OperationLogPort, OperationsApi};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

/// 通信模块对外组合输出结构体
/// 封装通信主API与微信授权子API，作为组装函数返回统一出口
pub(crate) struct CommunicationsComposition {
    pub(crate) api: CommunicationsApi,
    pub(crate) wechat_authorization: WeChatAuthorizationApi,
}

/// 通信模块组装所需全部外部依赖集合
/// 聚合各业务域、基础设施、配置、日志目录等外部传入依赖
pub(crate) struct CommunicationsDependencies {
    pub(crate) database: NativeDatabase,
    pub(crate) operations: OperationsApi,
    pub(crate) agents: AgentRuntimeApi,
    pub(crate) sessions: SessionsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) desktop_settings: DesktopSettingsApi,
    pub(crate) fallback_log_directory: PathBuf,
}

/// 通信模块核心组装入口方法
/// 职责：完成通信领域所有适配器、仓储、服务、运行时、入站桥接实例化与依赖注入，组装对外API实例
/// 参数 dependencies：外部传入全量业务与基础设施依赖
/// 返回 Result<通信对外组合实例, 错误字符串>
pub(crate) fn assemble_communications(
    dependencies: CommunicationsDependencies,
) -> Result<CommunicationsComposition, String> {
    // 步骤1：初始化全局统一日志适配器，传入兜底日志目录
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(
        dependencies.fallback_log_directory,
    ));
    // 转换日志适配器为诊断日志端口抽象，供通信模块日志层使用
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    // 转换日志适配器为操作日志端口抽象，记录业务操作流水
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging;
    // 步骤2：构建通信专属日志适配器，聚合诊断日志与操作日志能力
    let logging = Arc::new(CommunicationsLoggingAdapter::new(
        diagnostics,
        operation_logs,
    ));
    // 克隆桌面配置依赖，用于多语言繁忙提示文案读取
    let desktop_settings = dependencies.desktop_settings.clone();
    // 步骤3：构建消息过载提示文案提供器闭包，根据系统语言返回对应多语言提示
    let busy_message: BusyMessageProvider = Arc::new(move || {
        let copy = match desktop_settings.get_settings() {
            Ok(view) => NativeCopy::for_language(view.settings.application_language()),
            Err(_) => NativeCopy::resolve("zh-CN"),
        };
        copy.communications_overload.to_string()
    });
    // 步骤4：初始化通信入站消息桥接器，承载外部消息接收、限流提示能力
    let inbound = Arc::new(CommunicationsInboundBridge::new(
        logging.clone(),
        busy_message,
    ));
    // 步骤5：初始化连接器运行管理器，绑定入站桥，管控连接器运行生命周期
    let runtime = ConnectorRuntimeManager::new(inbound.clone());
    // 步骤6：初始化sqlite通信仓储，注入底层数据库实例，持久化通信会话、消息数据
    let repository = SqliteCommunicationsRepository::new(dependencies.database);
    // 步骤7：注入系统时钟实现，提供统一时间获取能力
    let clock: Arc<dyn CommunicationsClockPort> = Arc::new(SystemCommunicationsClock);
    // 步骤8：组装通信应用服务所需全部端口适配器，完成领域层依赖注入
    let service = CommunicationsApplicationService::new(CommunicationsApplicationPorts {
        repository: Arc::new(repository.clone()),
        credentials: Arc::new(CommunicationsCredentialAdapter::new()),
        transports: Arc::new(CommunicationsTransportAdapter::new(
            runtime,
            repository.clone(),
        )),
        agents: Arc::new(CommunicationsAgentExecutionAdapter::new(
            dependencies.agents,
            dependencies.sessions.clone(),
            dependencies.workspaces,
        )),
        sessions: Arc::new(CommunicationsSessionBindingAdapter::new(
            repository,
            dependencies.sessions,
            clock.clone(),
        )),
        operations: Arc::new(CommunicationsOperationAdapter::new(dependencies.operations)),
        clock,
        logging: logging as Arc<dyn CommunicationsLoggingPort>,
    });
    // 步骤9：实例化通信对外主API，绑定组装完成的应用服务
    let api = CommunicationsApi::new(service);
    // 步骤10：将主API挂载至入站消息桥，使外部消息可调用通信业务能力；挂载失败转换错误信息返回
    inbound
        .attach(api.clone())
        .map_err(|error| error.safe_code().to_string())?;
    // 步骤11：组装最终对外输出结构体，生成微信授权子API与主API并返回
    Ok(CommunicationsComposition {
        wechat_authorization: WeChatAuthorizationApi::new(api.clone()),
        api,
    })
}
