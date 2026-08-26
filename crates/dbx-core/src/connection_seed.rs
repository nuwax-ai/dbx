//! connections.json 种子导入（upsert 语义）——nuwax fork 本地扩展。
//!
//! 上游 `migrate_connections_json` 为 `INSERT OR IGNORE`：同 id 整行跳过、明文
//! 密码落 `connections.config_json`（读取时被 secrets 表覆盖 = 死数据）——外部
//! 重写文件无法吸收新凭据。本模块替换该导入步：
//!
//! - **文件存在 = 显式同步意图，按 id upsert 覆盖吸收**（宿主在 PG 凭据变更后
//!   重写 connections.json 并重启 dbx 即完成同步；UI 手改不重写文件，不受影响）；
//! - `config_json` 存脱敏版、密码播种进 `connection_secrets`（运行期唯一有效
//!   来源，与 UI 保存路径 `persist_connection_in_tx` 同款存储姿势）；
//! - `save_password=false` → 清除 secret（空串 = DELETE，同上游语义）；
//! - 消费后仍改名 `.bak`（上游消费标记语义不变）。
//!
//! 逻辑独立本文件（storage.rs 仅三处接口点），降低与上游 merge 的冲突面。

use std::path::Path;

use rusqlite::params;

use crate::models::connection::ConnectionConfig;
use crate::storage::{persist_secret_in_tx, sanitized_connection_config, Storage};

impl Storage {
    /// connections.json 种子导入（upsert）：按 id 覆盖 `connections.config_json`
    /// （脱敏）并播种 `connection_secrets.password`。
    pub(crate) async fn migrate_connections_json_upsert(&self, data_dir: &Path) -> Result<(), String> {
        let path = data_dir.join("connections.json");
        if tokio::fs::metadata(&path).await.is_err() {
            return Ok(());
        }
        let json = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
        let configs: Vec<ConnectionConfig> = serde_json::from_str(&json).unwrap_or_default();
        for config in configs {
            let id = config.id.clone();
            let save_password = config.save_password;
            let password = config.password.clone();
            let sanitized = sanitized_connection_config(&config);
            let config_json = serde_json::to_string(&sanitized).map_err(|e| e.to_string())?;
            self.with_conn(move |conn| {
                let tx = conn.transaction().map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO connections (id, config_json) VALUES (?1, ?2) \
                     ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json",
                    params![id, config_json],
                )
                .map(|_| ())
                .map_err(|e| e.to_string())?;
                let secret = if save_password { password.as_str() } else { "" };
                persist_secret_in_tx(&tx, &id, "password", secret)?;
                tx.commit().map(|_| ()).map_err(|e| e.to_string())
            })
            .await?;
        }
        let _ = tokio::fs::rename(&path, data_dir.join("connections.json.bak")).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_data_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dbx-conn-seed-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_json(password: &str, save_password: Option<bool>) -> String {
        let mut entry = format!(
            r#"{{"id":"local-pg","name":"Local PostgreSQL","db_type":"postgres","host":"127.0.0.1","port":5432,"username":"app","password":"{password}","database":"app""#
        );
        if let Some(sp) = save_password {
            entry.push_str(&format!(r#","save_password":{sp}"#));
        }
        entry.push('}');
        format!("[{entry}]")
    }

    async fn open_storage(data_dir: &Path) -> Storage {
        Storage::open(&data_dir.join("dbx.db")).await.expect("open storage")
    }

    #[tokio::test]
    async fn first_import_seeds_sanitized_config_and_password() {
        // 诊断锚点:json → Vec<ConnectionConfig> 反序列化必须成功(失败即空导入)
        let probe: Vec<ConnectionConfig> =
            serde_json::from_str(&seed_json("pw-one", None)).expect("seed json 反序列化");
        assert_eq!(probe.len(), 1, "seed json 应恰一条连接");

        let dir = temp_data_dir("first");
        std::fs::write(dir.join("connections.json"), seed_json("pw-one", None)).unwrap();
        let storage = open_storage(&dir).await;
        storage.migrate_connections_json_upsert(&dir).await.unwrap();

        let loaded = storage.load_connections().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "local-pg");
        assert_eq!(loaded[0].password, "pw-one", "secrets 播种后 load 回读新密码");

        // config_json 脱敏：明文密码不得落 connections 表
        let raw: String = storage
            .with_conn(|conn| {
                Ok(conn
                    .query_row("SELECT config_json FROM connections WHERE id = 'local-pg'", [], |row| row.get(0))
                    .map_err(|e| e.to_string())?)
            })
            .await
            .unwrap();
        assert!(!raw.contains("pw-one"), "config_json 应为脱敏版: {raw}");

        // 消费标记：文件改名 .bak
        assert!(!dir.join("connections.json").exists());
        assert!(dir.join("connections.json.bak").exists());
    }

    #[tokio::test]
    async fn reimport_same_id_updates_password() {
        let dir = temp_data_dir("reimport");
        std::fs::write(dir.join("connections.json"), seed_json("old-pw", None)).unwrap();
        let storage = open_storage(&dir).await;
        storage.migrate_connections_json_upsert(&dir).await.unwrap();
        assert_eq!(storage.load_connections().await.unwrap()[0].password, "old-pw");

        // 宿主重写文件（同 id 新密码）→ 再导入 → upsert 生效（上游 OR IGNORE 会跳过）
        std::fs::write(dir.join("connections.json"), seed_json("new-pw", None)).unwrap();
        storage.migrate_connections_json_upsert(&dir).await.unwrap();
        let loaded = storage.load_connections().await.unwrap();
        assert_eq!(loaded.len(), 1, "同 id 不应产生重复行");
        assert_eq!(loaded[0].password, "new-pw", "同 id 重导入必须吸收新密码（核心语义）");
    }

    #[tokio::test]
    async fn save_password_false_clears_secret() {
        let dir = temp_data_dir("no-save");
        std::fs::write(dir.join("connections.json"), seed_json("pw-x", Some(false))).unwrap();
        let storage = open_storage(&dir).await;
        storage.migrate_connections_json_upsert(&dir).await.unwrap();
        let loaded = storage.load_connections().await.unwrap();
        assert_eq!(loaded[0].password, "", "save_password=false → secret 清除（load 回读空）");
    }

    #[tokio::test]
    async fn invalid_json_imports_nothing_and_consumes_file() {
        let dir = temp_data_dir("bad-json");
        std::fs::write(dir.join("connections.json"), "not a json array").unwrap();
        let storage = open_storage(&dir).await;
        storage.migrate_connections_json_upsert(&dir).await.unwrap();
        assert!(storage.load_connections().await.unwrap().is_empty(), "坏 json → 空导入不报错（上游行为回归）");
        assert!(dir.join("connections.json.bak").exists());
    }
}
