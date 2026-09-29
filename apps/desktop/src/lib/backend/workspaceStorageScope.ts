import { dbxWebBasePath } from "@/lib/common/webPath";

let cachedSuffix: string | undefined;

/**
 * nuwax fork: 浏览器存储按挂载路径隔离工作台。
 *
 * 同源反代部署（RCoder `/api/v1/userapp/proxy/dbx/{env}/{user_id}/{app_id}/`）下，
 * 多个 dbx 工作台共享同一个浏览器 origin——IndexedDB/localStorage 只按 origin 隔离，
 * 标签、SQL 草稿、结果缓存会互相覆盖/串台。以 `dbxWebBasePath()` 推断出的挂载前缀
 * 作为工作台 key，给所有浏览器侧存储桶名追加后缀：
 *
 *   /api/v1/userapp/proxy/dbx/dev/1/197  →  :api-v1-userapp-proxy-dbx-dev-1-197
 *
 * 桌面版（tauri 协议）与根路径部署（`/`）推导出的 base 为空 → 后缀为空串，
 * 存储名与官方逐字节相同：现有用户零迁移，官方测试零适配（jsdom pathname 为 `/`）。
 * 后缀在首个调用时计算并缓存（location.pathname 在文档生命周期内对同一路径稳定，
 * `/login` 结尾已由 `dbxWebBasePath` 剥除）。
 */
export function browserStorageScopeSuffix(): string {
  if (cachedSuffix === undefined) {
    const base = dbxWebBasePath().replace(/^\/+/, "");
    cachedSuffix = base ? `:${base.replace(/[^A-Za-z0-9_.-]+/g, "-")}` : "";
  }
  return cachedSuffix;
}
