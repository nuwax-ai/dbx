//! 容器内本地 PostgreSQL 预置连接（nuwax fork 本地扩展）。
//!
//! 语义：宿主容器（app-runtime / agent-runner）内运行 dbx-web 时，启动即
//! 确保存在一条 **unix socket 免密** 的 `local-pg` 连接——
//! `host=/var/run/postgresql` + 空密码，靠容器 initdb 的
//! `pg_hba auth-local=trust` 直连（密码与 dbx 完全解耦：每用户密码不同、
//! 平台改密多少次都无感）。凭据不落盘（sanitized config 入库 + secret 为空）。
//!
//! 判定开关：env `POSTGRES_USER` 存在（容器语义；本地裸跑 dbx 无此 env 不注入，
//! 不打扰非容器用户）。
//!
//! 每次启动**刷新为出厂态**：id=local-pg 不存在→插入；存在但形态不符（如存量
//! TCP+密码连接，或用户误编辑）→覆盖升级；形态符→跳过。用户自建的其它连接
//! 不受影响（全量列表仅替换 local-pg 一项）。

use crate::models::connection::ConnectionConfig;
use crate::storage::Storage;

/// local-pg 固定 id 与 socket 目录（容器 PG 的 pg_hba local=trust 通道）。
pub const LOCAL_PG_ID: &str = "local-pg";
const LOCAL_PG_SOCKET_DIR: &str = "/var/run/postgresql";

/// 启动时确保 local-pg socket 连接存在且为出厂形态。
///
/// 带 info 日志（`ensure-local-pg` 特征串供镜像产物构建校验）。
pub async fn ensure_local_pg_connection(storage: &Storage) -> Result<(), String> {
    let Some(user) = std::env::var("POSTGRES_USER").ok().filter(|u| !u.trim().is_empty()) else {
        return Ok(());
    };
    let database = std::env::var("POSTGRES_DB").ok().filter(|d| !d.trim().is_empty());
    ensure_local_pg_with(storage, &user, database.as_deref()).await
}

/// 核心逻辑（env 已解析；测试直调，避开进程级 env 与并行测试竞态）。
pub async fn ensure_local_pg_with(storage: &Storage, user: &str, database: Option<&str>) -> Result<(), String> {
    let desired = factory_config(user, database);

    let mut configs = storage.load_connections().await?;
    match configs.iter().position(|c| c.id == LOCAL_PG_ID) {
        Some(index) if is_factory_shape(&configs[index], &desired) => Ok(()),
        Some(index) => {
            log::info!("ensure-local-pg: upgrade existing connection to socket/trust form (id={LOCAL_PG_ID})");
            configs[index] = desired;
            storage.save_connections(&configs).await
        }
        None => {
            log::info!("ensure-local-pg: seed container-local PostgreSQL connection (socket/trust, id={LOCAL_PG_ID})");
            configs.push(desired);
            storage.save_connections(&configs).await
        }
    }
}

/// 出厂形态：socket host + 空密码（save_password 默认 true + 空密码 =
/// `persist_connection_in_tx` 播种空 secret，连接时无凭据，trust 直连）。
fn factory_config(user: &str, database: Option<&str>) -> ConnectionConfig {
    serde_json::from_str(&format!(
        r#"{{"id":"{LOCAL_PG_ID}","name":"Local PostgreSQL","db_type":"postgres","host":"{LOCAL_PG_SOCKET_DIR}","port":5432,"username":"{user}","password":"","database":{}}}"#,
        database
            .map(|d| format!("\"{d}\""))
            .unwrap_or_else(|| "null".into()),
    ))
    .expect("factory local-pg config json")
}

/// 形态判定含用户名/库名——容器重建换 `POSTGRES_USER/POSTGRES_DB`（dbx.db 在
/// 卷上留存旧值）时必须刷新，否则 trust 直连因 user 不符而失败。
fn is_factory_shape(current: &ConnectionConfig, desired: &ConnectionConfig) -> bool {
    current.host == desired.host
        && current.db_type == desired.db_type
        && current.username == desired.username
        && current.database == desired.database
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_data_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dbx-local-pg-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn open_storage(data_dir: &std::path::Path) -> Storage {
        Storage::open(&data_dir.join("dbx.db")).await.expect("open storage")
    }

    #[tokio::test]
    async fn seeds_when_missing_and_idempotent() {
        let dir = temp_data_dir("seed");
        let storage = open_storage(&dir).await;
        ensure_local_pg_with(&storage, "app", Some("appdb")).await.unwrap();
        let configs = storage.load_connections().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].id, LOCAL_PG_ID);
        assert_eq!(configs[0].host, "/var/run/postgresql");
        assert_eq!(configs[0].username, "app");
        assert_eq!(configs[0].database.as_deref(), Some("appdb"));
        assert_eq!(configs[0].password, "", "无凭据(trust)");
        // 二次 ensure：形态符→幂等跳过
        ensure_local_pg_with(&storage, "app", Some("appdb")).await.unwrap();
        assert_eq!(storage.load_connections().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn upgrades_legacy_tcp_connection_and_keeps_others() {
        let dir = temp_data_dir("upgrade");
        let storage = open_storage(&dir).await;
        // 存量：TCP+密码形态的 local-pg + 用户自建连接
        let legacy = serde_json::from_str(
            r#"{"id":"local-pg","name":"Local PostgreSQL","db_type":"postgres","host":"127.0.0.1","port":5432,"username":"app","password":"old-pw","database":"appdb"}"#,
        )
        .unwrap();
        let user_own = serde_json::from_str(
            r#"{"id":"my-remote","name":"My Remote","db_type":"postgres","host":"10.0.0.5","port":5432,"username":"u","password":"p","database":"d"}"#,
        )
        .unwrap();
        storage.save_connections(&[legacy, user_own]).await.unwrap();

        ensure_local_pg_with(&storage, "app", Some("appdb")).await.unwrap();
        let configs = storage.load_connections().await.unwrap();
        assert_eq!(configs.len(), 2, "仅替换 local-pg,自建连接保留");
        let local = configs.iter().find(|c| c.id == LOCAL_PG_ID).unwrap();
        assert_eq!(local.host, "/var/run/postgresql", "升级为 socket 形态");
        assert_eq!(local.password, "", "凭据清除");
        assert!(configs.iter().any(|c| c.id == "my-remote" && c.host == "10.0.0.5"), "用户自建连接不受影响");
    }

    #[tokio::test]
    async fn username_change_refreshes_connection() {
        // 容器重建换 POSTGRES_USER(dbx.db 在卷上存旧值)→必须刷新,否则 trust
        // 直连因 user 不符失败
        let dir = temp_data_dir("rename");
        let storage = open_storage(&dir).await;
        ensure_local_pg_with(&storage, "old-user", Some("d")).await.unwrap();
        ensure_local_pg_with(&storage, "new-user", Some("d2")).await.unwrap();
        let configs = storage.load_connections().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].username, "new-user", "user 随 env 刷新");
        assert_eq!(configs[0].database.as_deref(), Some("d2"));
    }
}
