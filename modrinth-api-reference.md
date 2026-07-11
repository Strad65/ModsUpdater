# Modrinth API v2 参考文档

> 基于官方文档 [docs.modrinth.com](https://docs.modrinth.com/api/) 整理
> 当前 API 版本: v2 (Labrinth v2.7.0)

---

## 目录

1. [基础信息](#1-基础信息)
2. [认证](#2-认证)
3. [速率限制](#3-速率限制)
4. [标识符规范](#4-标识符规范)
5. [通用错误格式](#5-通用错误格式)
6. [数据结构](#6-数据结构)
7. [API 端点](#7-api-端点)
   - [搜索](#71-搜索)
   - [项目 (Projects)](#72-项目-projects)
   - [版本 (Versions)](#73-版本-versions)
   - [版本文件 (Version Files)](#74-版本文件-version-files)
   - [用户 (Users)](#75-用户-users)
   - [通知 (Notifications)](#76-通知-notifications)
   - [团队 (Teams)](#77-团队-teams)
   - [举报/主题 (Reports/Threads)](#78-举报主题-reportsthreads)
   - [标签 (Tags)](#79-标签-tags)
   - [统计 (Statistics)](#710-统计-statistics)

---

## 1. 基础信息

| 项目 | 值 |
|------|-----|
| **生产环境 Base URL** | `https://api.modrinth.com` |
| **测试环境 Base URL** | `https://staging-api.modrinth.com` |
| **API 版本** | v2 |
| **所有端点前缀** | `/v2/` |
| **CORS** | 完全支持，通配符同源策略 |

### User-Agent 要求

**必须**提供唯一标识的 `User-Agent` 头，否则可能被屏蔽。

推荐格式:
```
github_username/project_name/1.56.0 (contact@domain.com)
```

示例:
```
myusername/mod-updater/1.0.0 (dev@example.com)
```

---

## 2. 认证

| 方案 | 类型 | Header |
|------|------|--------|
| TokenAuth | `apiKey` | `Authorization` |

### Token 类型

- **Personal Access Tokens (PATs)**: 从用户设置页面生成，前缀 `mrp_`
- **OAuth2**: 标准 OAuth2 流程
- **GitHub Tokens**: 向后兼容，将在 API v3 中移除

### 请求格式

```
Authorization: mrp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### 需要认证的操作

- 创建/修改/删除数据
- 访问私有数据 (草稿、通知、邮箱、付款记录)

### 权限范围 (Scopes)

每个端点有对应的 scope 要求。如果 token 缺少所需 scope，返回 **401**。

Scope 定义参考: [labrinth/src/models/pats.rs](https://github.com/modrinth/labrinth/blob/master/src/models/pats.rs#L15)

---

## 3. 速率限制

| 限制 | 值 |
|------|-----|
| **频率** | 每分钟 300 请求 (per IP) |
| **Token vs 无 Token** | 无区别 |

### 响应头

| Header | 含义 |
|--------|------|
| `X-Ratelimit-Limit` | 每分钟最大请求数 |
| `X-Ratelimit-Remaining` | 当前窗口剩余次数 |
| `X-Ratelimit-Reset` | 窗口重置剩余秒数 |

---

## 4. 标识符规范

| 实体 | 标识符格式 | 说明 |
|------|-----------|------|
| 项目 (Project) | 8 位 base62 字符串 | 如 `AABBCCDD`，也支持 slug |
| 版本 (Version) | 8 位 base62 字符串 | 如 `EEFFGGHH` |
| 用户 (User) | 8 位 base62 字符串 | 也支持 username |
| 主题 (Thread) | 8 位 base62 字符串 | — |
| 团队 (Team) | 8 位 base62 字符串 | — |
| 举报 (Report) | 8 位 base62 字符串 | — |
| 版本文件 (File) | sha1 或 sha512 哈希 | 十六进制编码 |

> **重要**: ID 是永久的；slug/username 可能变更，建议始终使用 ID 进行持久存储。

---

## 5. 通用错误格式

### 400 Bad Request
```json
{
  "error": "invalid_input",
  "description": "具体错误描述"
}
```

### 404 Not Found
请求的资源不存在或无权限访问。

### 401 Unauthorized
Token 无效或缺少所需 scope。

### 410 Gone
请求了已废弃的 API 版本。

---

## 6. 数据结构

### 6.1 项目 (Project)

```jsonc
{
  // === 基本信息 ===
  "id": "AABBCCDD",                     // required, base62 字符串
  "slug": "my-awesome-mod",             // 正则: ^[\w!@$()`.+,"\-']{3,64}$
  "title": "My Awesome Mod",            // 项目名称
  "description": "A short description", // 简短描述
  "body": "Long form description...",   // 长描述 (Markdown)
  "body_url": null,                     // 遗留字段，始终为 null

  // === 分类 ===
  "project_type": "mod",                // required, 枚举值见 6.5
  "categories": ["adventure", "magic"], // 主分类列表
  "additional_categories": [],          // 次要/可搜索分类

  // === 客户端/服务端支持 ===
  "client_side": "required",            // 枚举值见 6.6
  "server_side": "optional",            // 枚举值见 6.6

  // === 统计 ===
  "downloads": 12345,                   // required, 总下载次数
  "followers": 678,                     // required, 关注者数量

  // === 视觉 ===
  "icon_url": "https://...",            // nullable, 项目图标 URL
  "color": 0xAABBCC,                    // nullable, 图标自动生成的颜色
  "gallery": [                          // 画廊图片数组
    {
      "url": "https://...",             // required
      "featured": true,                 // required, 是否为特色图片
      "title": "Screenshot 1",          // nullable
      "description": "In-game view",    // nullable
      "created": "2024-01-15T...",     // required, ISO-8601
      "ordering": 0                     // 排序字段
    }
  ],

  // === 许可证 ===
  "license": {
    "id": "MIT",                        // SPDX 标识符
    "name": "MIT License",              // 许可证全名
    "url": "https://opensource.org/licenses/MIT" // nullable
  },

  // === 链接 ===
  "issues_url": "https://...",          // nullable, Issue 追踪器
  "source_url": "https://...",          // nullable, 源代码仓库
  "wiki_url": "https://...",            // nullable, Wiki 页面
  "discord_url": "https://...",         // nullable, Discord 邀请
  "donation_urls": [                    // 赞助链接
    {
      "id": "patreon",                  // 平台 ID
      "platform": "Patreon",            // 平台名称
      "url": "https://patreon.com/..."  // 链接
    }
  ],

  // === 状态 ===
  "status": "approved",                 // 见下方状态枚举
  "requested_status": null,             // nullable, 申请的状态变更
  "monetization_status": "monetized",   // monetized | demonetized | force-demonetized

  // === 日期 ===
  "published": "2024-01-01T00:00:00Z", // required, ISO-8601
  "updated": "2024-06-15T12:30:00Z",   // required, ISO-8601
  "approved": "2024-01-01T...",         // nullable, ISO-8601
  "queued": null,                       // nullable, ISO-8601

  // === 团队与审核 ===
  "team": "BBCCDDEE",                   // required, 所属团队 ID
  "thread_id": "FFGGHHII",             // 审核讨论串 ID
  "moderator_message": {                // 管理员留言
    "message": "Message text",
    "body": null                        // nullable
  },

  // === 版本信息 ===
  "versions": ["EEFFGGHH", ...],       // 版本 ID 列表 (草稿可为空)
  "game_versions": ["1.20.1", ...],    // 支持的 Minecraft 版本
  "loaders": ["fabric", "forge"]       // 支持的加载器
}
```

#### 项目状态枚举

| 值 | 含义 |
|----|------|
| `approved` | 已批准，公开可见 |
| `archived` | 已归档，搜索不可见 |
| `rejected` | 审核未通过 |
| `draft` | 草稿，仅创建者可见 |
| `unlisted` | 未列出，有链接可访问 |
| `processing` | 处理中 |
| `withheld` | 因版权等原因保留 |
| `scheduled` | 计划发布 |
| `private` | 私有 |
| `unknown` | 未知状态 |

#### requested_status 可取值

`approved`, `archived`, `unlisted`, `private`, `draft`

---

### 6.2 版本 (Version)

```jsonc
{
  // === 基本信息 ===
  "id": "EEFFGGHH",                     // required, base62 字符串
  "project_id": "AABBCCDD",             // required, 所属项目 ID
  "author_id": "USERID01",              // required, 发布者 ID
  "name": "Version 1.0.0",             // 版本名称
  "version_number": "1.0.0",           // 版本号 (推荐遵循 semver)

  // === 内容 ===
  "changelog": "## Changes\n- Fix bug", // nullable, 更新日志 (Markdown)
  "changelog_url": null,                // nullable, 遗留字段，始终为 null

  // === 日期与统计 ===
  "date_published": "2024-01-15T...",  // required, ISO-8601
  "downloads": 5678,                    // required, 下载次数

  // === 类型与状态 ===
  "version_type": "release",            // release | beta | alpha
  "status": "listed",                   // 见下方状态枚举
  "requested_status": null,             // nullable, listed | archived | draft | unlisted
  "featured": false,                    // 是否为特色版本

  // === 兼容性 ===
  "game_versions": ["1.20.1", "1.20.2"], // Minecraft 版本列表
  "loaders": ["fabric", "forge"],       // 加载器列表 (资源包用 "minecraft")

  // === 文件 ===
  "files": [
    {
      "hashes": {
        "sha512": "abc123...",          // required
        "sha1": "def456..."             // required
      },
      "url": "https://cdn.modrinth.com/data/.../file.jar",  // required, 直接下载链接
      "filename": "mod-1.0.0.jar",      // required
      "primary": true,                  // required, 是否为主要文件 (每版本只有一个)
      "size": 1048576,                  // required, 文件大小 (字节)
      "file_type": null                 // nullable, 见下方文件类型枚举
    }
  ],

  // === 依赖 ===
  "dependencies": [
    {
      "dependency_type": "required",    // required, 见下方依赖类型枚举
      "version_id": "VERID0001",        // nullable, 依赖的具体版本 ID
      "project_id": "PRJID0001",        // nullable, 依赖的项目 ID
      "file_name": null                 // nullable, 主要用于 modpack 外部依赖
    }
  ]
}
```

#### 版本状态枚举

`listed`, `archived`, `draft`, `unlisted`, `scheduled`, `unknown`

#### 文件类型枚举

| 值 | 含义 |
|----|------|
| `required-resource-pack` | 必需的资源包 |
| `optional-resource-pack` | 可选的资源包 |
| `sources-jar` | 源代码 JAR |
| `dev-jar` | 开发版 JAR |
| `javadoc-jar` | Javadoc JAR |
| `signature` | 签名文件 |
| `unknown` | 未知类型 |

#### 依赖类型枚举

| 值 | 含义 |
|----|------|
| `required` | 必需依赖 |
| `optional` | 可选依赖 |
| `incompatible` | 不兼容 |
| `embedded` | 内嵌依赖 |

---

### 6.3 搜索结果项 (Search Hit)

相比完整的 Project 对象，搜索结果增加了以下字段，减少了 `body`、`team` 等详细信息:

```jsonc
{
  "project_id": "AABBCCDD",             // required
  "project_type": "mod",                // required
  "all_project_types": ["mod"],         // required, 所有版本中的项目类型
  "slug": "my-mod",
  "title": "My Mod",
  "description": "A short description",
  "author": "username",                 // required, 作者用户名
  "categories": ["adventure"],
  "display_categories": ["adventure"],
  "versions": ["1.20.1", "1.20.2"],    // required
  "latest_version": "1.20.2",
  "downloads": 12345,                   // required
  "follows": 678,                       // required
  "date_created": "2024-01-01T...",    // required, ISO-8601
  "date_modified": "2024-06-15T...",   // required, ISO-8601
  "license": "MIT",                     // required, SPDX ID 字符串
  "client_side": "required",
  "server_side": "optional",
  "icon_url": "https://...",            // nullable
  "color": 0xAABBCC,                    // nullable
  "thread_id": "FFGGHHII",
  "monetization_status": "monetized",
  "gallery": ["https://...", ...],
  "featured_gallery": "https://..."     // nullable, 特色画廊图片
}
```

---

### 6.4 标签类别 (Category)

```jsonc
{
  "icon": "<svg>...</svg>",             // required, SVG 图标
  "name": "adventure",                  // required, 分类名称
  "project_type": "mod",                // required, 适用的项目类型
  "header": "resolutions"               // required, 所属分组标题
}
```

---

### 6.5 项目类型 (project_type)

```
"mod" | "modpack" | "resourcepack" | "shader"
```

---

### 6.6 客户端/服务端类型 (side_type)

```
"required" | "optional" | "unsupported" | "unknown"
```

---

### 6.7 游戏版本 (Game Version)

```jsonc
{
  "version": "1.20.1",                  // required, 版本号
  "version_type": "release",            // required, release | snapshot | alpha | beta
  "date": "2023-06-12T...",            // required, ISO-8601
  "major": true                         // required, 是否为主版本 (用于 Featured Versions)
}
```

---

### 6.8 加载器 (Loader)

```jsonc
{
  "icon": "<svg>...</svg>",             // required, SVG 图标
  "name": "fabric",                     // required, 加载器名称
  "supported_project_types": ["mod", "modpack"]  // required, 支持的 project type
}
```

---

### 6.9 许可证 (License Tag)

```jsonc
{
  "short": "MIT",                       // required, SPDX 短标识符
  "name": "MIT License"                 // required, 许可证全名
}
```

> **注意**: 此端点已标记为 deprecated，官方建议直接使用 SPDX ID。

---

### 6.10 赞助平台 (Donation Platform)

```jsonc
{
  "short": "patreon",                   // required, 平台短标识符
  "name": "Patreon"                     // required, 平台全名
}
```

---

## 7. API 端点

### 7.1 搜索

#### `GET /v2/search` — 搜索项目

**查询参数:**

| 参数 | 类型 | 默认值 | 约束 | 说明 |
|------|------|--------|------|------|
| `query` | string | — | — | 搜索关键词 |
| `facets` | string | — | — | 筛选条件 (见下方 facet 语法) |
| `index` | string | `relevance` | `relevance`, `downloads`, `follows`, `newest`, `updated` | 排序方式 |
| `offset` | integer | — | — | 跳过结果数 |
| `limit` | integer | `10` | max: `100` | 返回结果数 |

**Facet 语法 (本质是嵌套的 JSON 数组的字符串表示):**

```jsonc
// 格式: [["type:operator:value", ...], [...], ...]

// OR: 同一内层数组中的元素是 OR 关系
[["versions:1.16.5", "versions:1.17.1"]]  // 支持 1.16.5 或 1.17.1

// AND: 不同内层数组之间是 AND 关系
[
  ["versions:1.16.5"],
  ["project_type:modpack"]
]
// 支持 1.16.5 AND 类型是 modpack
```

**可用的 facet 类型:**

| 类型 | 说明 | 常用度 |
|------|------|--------|
| `project_type` | 项目类型 | 常用 |
| `all_project_types` | 匹配所有版本中的类型 | 常用 |
| `categories` | 分类 (含 loaders) | 常用 |
| `versions` | Minecraft 版本 | 常用 |
| `client_side` | 客户端支持 | 常用 |
| `server_side` | 服务端支持 | 常用 |
| `open_source` | 是否开源 | 常用 |
| `title` | 标题 | 较少用 |
| `author` | 作者 | 较少用 |
| `follows` | 关注数 | 较少用 |
| `project_id` | 项目 ID | 较少用 |
| `license` | 许可证 | 较少用 |
| `downloads` | 下载数 | 较少用 |
| `color` | 主题色 | 较少用 |
| `created_timestamp` | 创建时间戳 (Unix) | 较少用 |
| `modified_timestamp` | 修改时间戳 (Unix) | 较少用 |
| `date_created` | 创建日期 (ISO-8601) | 较少用 |
| `date_modified` | 修改日期 (ISO-8601) | 较少用 |

**支持的操作符:** `:` (等于/`=`), `!=`, `>=`, `>`, `<=`, `<`

**响应格式:**
```jsonc
{
  "hits": [ /* SearchHit 对象数组，见 6.3 */ ],
  "offset": 0,      // required, 跳过的结果数
  "limit": 10,      // required, 返回的结果数
  "total_hits": 42  // required, 总匹配数
}
```

---

### 7.2 项目 (Projects)

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/project/{id}` | 获取单个项目 | 否 |
| `GET` | `/v2/projects` | 批量获取项目 | 否 |
| `GET` | `/v2/projects/random` | 随机获取项目 | 否 |
| `GET` | `/v2/project/{id}/check` | 检查 slug/ID 有效性 | 否 |
| `GET` | `/v2/project/{id}/dependencies` | 获取项目依赖 | 否 |
| `POST` | `/v2/project` | 创建项目 | **是** |
| `PATCH` | `/v2/project/{id}` | 修改项目 | **是** |
| `PATCH` | `/v2/projects` | 批量修改项目 | **是** |
| `DELETE` | `/v2/project/{id}` | 删除项目 | **是** |
| `POST` | `/v2/project/{id}/follow` | 关注项目 | **是** |
| `DELETE` | `/v2/project/{id}/follow` | 取消关注 | **是** |
| `PATCH` | `/v2/project/{id}/icon` | 修改项目图标 | **是** |
| `DELETE` | `/v2/project/{id}/icon` | 删除项目图标 | **是** |
| `POST` | `/v2/project/{id}/gallery` | 添加画廊图片 | **是** |
| `PATCH` | `/v2/project/{id}/gallery` | 修改画廊图片 | **是** |
| `DELETE` | `/v2/project/{id}/gallery` | 删除画廊图片 | **是** |
| `POST` | `/v2/project/{id}/schedule` | 计划发布项目 | **是** |

#### `GET /v2/project/{id}` — 获取项目

- **路径参数**: `id` — 项目 ID (base62) 或 slug
- **响应**: [Project 对象](#61-项目-project)

#### `GET /v2/projects` — 批量获取项目

- **查询参数**: `ids` — JSON 编码的 ID 数组
- **示例**: `GET /v2/projects?ids=%5B%22AABBCCDD%22%2C%22BBCCDDEE%22%5D`
- **响应**: `Project[]`

---

### 7.3 版本 (Versions)

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/project/{id}/version` | 列出项目的所有版本 | 否 |
| `GET` | `/v2/version/{id}` | 获取单个版本 | 否 |
| `GET` | `/v2/version/{id_or_number}` | 按版本号或 ID 获取 | 否 |
| `GET` | `/v2/versions` | 批量获取版本 | 否 |
| `POST` | `/v2/version` | 创建版本 | **是** |
| `PATCH` | `/v2/version/{id}` | 修改版本 | **是** |
| `DELETE` | `/v2/version/{id}` | 删除版本 | **是** |
| `POST` | `/v2/version/{id}/file` | 向版本添加文件 | **是** |
| `POST` | `/v2/version/{id}/schedule` | 计划发布版本 | **是** |

#### `GET /v2/project/{id}/version` — 列出项目版本

- **路径参数**: `id` — 项目 ID 或 slug
- **查询参数**:
  - `loaders` — JSON 编码的加载器数组，如 `["fabric","forge"]`
  - `game_versions` — JSON 编码的版本数组，如 `["1.20.1","1.20.2"]`
  - `featured` — bool，是否仅返回特色版本
- **响应**: `Version[]`

#### `GET /v2/versions` — 批量获取版本

- **查询参数**: `ids` — JSON 编码的版本 ID 数组
- **示例**: `GET /v2/versions?ids=%5B%22EEFFGGHH%22%2C%22FFGGHHII%22%5D`
- **响应**: `Version[]`

---

### 7.4 版本文件 (Version Files)

这是 **mod 更新器最关键的 API 分类**。

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/version_file/{hash}` | 根据哈希获取版本 | 否 |
| `DELETE` | `/v2/version_file/{hash}` | 删除版本文件 | **是** |
| `POST` | `/v2/version_file/{hash}/update` | 根据哈希检查单个更新 | 否 |
| `POST` | `/v2/version_files` | 批量根据哈希获取版本 | 否 |
| `POST` | `/v2/version_files/update` | 批量检查更新 | 否 |

---

#### `POST /v2/version_file/{hash}/update` — 检查单个文件更新

**用途**: 已知一个文件的哈希，查询是否有更新的版本。

**路径参数:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `hash` | string | 文件哈希 (十六进制) |

**查询参数:**

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `algorithm` | string | `sha1` | 哈希算法: `sha1` 或 `sha512` |

**请求体 (JSON):**
```jsonc
{
  "loaders": ["fabric", "forge"],       // required, 加载器过滤
  "game_versions": ["1.20.1"]           // required, Minecraft 版本过滤
}
```

**响应体**: 匹配的 [Version 对象](#62-版本-version)

**错误**: `400` — 请求无效；`404` — 未找到匹配版本

---

#### `POST /v2/version_files` — 批量根据哈希获取版本

**用途**: 一次查询多个文件的哈希对应的版本信息。

**请求体 (JSON):**
```jsonc
{
  "hashes": ["abc123...", "def456..."], // required, 哈希列表
  "algorithm": "sha512"                 // required, sha1 或 sha512
}
```

**响应体**: 一个哈希 → 版本的 Map:
```jsonc
{
  "abc123...": { /* Version 对象 */ },
  "def456...": { /* Version 对象 */ }
}
```

**注意**: 未匹配到的哈希不会出现在响应中。

---

#### `POST /v2/version_files/update` — 批量检查更新 ⭐

**这是 mod 更新器最核心的端点**，用于一次性检查多个 mod 是否有更新。

**请求体 (JSON):**
```jsonc
{
  "hashes": [
    "abc123...",                         // required, 文件哈希列表
    "def456..."
  ],
  "algorithm": "sha512",                 // required, sha1 或 sha512
  "loaders": ["fabric"],                 // required, 加载器过滤
  "game_versions": ["1.20.1", "1.20.2"] // required, Minecraft 版本过滤
}
```

**响应体**: 仅返回有更新的文件映射:
```jsonc
{
  "abc123...": {
    "id": "NEWVER01",
    "project_id": "AABBCCDD",
    "author_id": "USERID01",
    "name": "Version 2.0.0",
    "version_number": "2.0.0",
    "changelog": "Major update!",
    "date_published": "2024-06-15T...",
    "downloads": 1234,
    "version_type": "release",
    "status": "listed",
    "featured": false,
    "game_versions": ["1.20.1", "1.20.2"],
    "loaders": ["fabric"],
    "dependencies": [
      {
        "dependency_type": "required",
        "version_id": "DEPVER01",
        "project_id": "DEPPRJ01",
        "file_name": null
      }
    ],
    "files": [
      {
        "hashes": {
          "sha512": "newhash123...",
          "sha1": "newhash456..."
        },
        "url": "https://cdn.modrinth.com/data/.../mod-2.0.0.jar",
        "filename": "mod-2.0.0.jar",
        "primary": true,
        "size": 2097152,
        "file_type": null
      }
    ]
  }
}
```

**关键行为**:
- 仅返回**有更新**的文件 — 如果某个哈希已是最新，它不会出现在响应中
- `loaders` 和 `game_versions` 用于过滤结果，确保只返回兼容的更新版本
- 可同时检查 sha1 和 sha512 哈希

---

#### `GET /v2/version_file/{hash}` — 根据哈希获取版本

- **路径参数**: `hash` — 文件哈希
- **查询参数**: `algorithm` — `sha1` (默认) 或 `sha512`
- **响应**: [Version 对象](#62-版本-version)

---

### 7.5 用户 (Users)

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/user/{id}` | 获取用户信息 | 否 |
| `GET` | `/v2/user` | 从 Token 获取当前用户 | **是** |
| `GET` | `/v2/users` | 批量获取用户 | 否 |
| `PATCH` | `/v2/user/{id}` | 修改用户信息 | **是** |
| `PATCH` | `/v2/user/{id}/icon` | 修改头像 | **是** |
| `DELETE` | `/v2/user/{id}/icon` | 删除头像 | **是** |
| `GET` | `/v2/user/{id}/projects` | 获取用户的项目 | 否 |
| `GET` | `/v2/user/{id}/follows` | 获取用户关注的項目 | 否 |
| `GET` | `/v2/user/{id}/payouts` | 获取付款记录 | **是** |
| `POST` | `/v2/user/{id}/payouts` | 提现 | **是** |

---

### 7.6 通知 (Notifications)

所有通知端点都需要认证。

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/v2/user/{id}/notifications` | 获取用户通知 |
| `GET` | `/v2/notification/{id}` | 获取单个通知 |
| `GET` | `/v2/notifications` | 批量获取通知 |
| `PATCH` | `/v2/notification/{id}` | 标记为已读 |
| `PATCH` | `/v2/notifications` | 批量标记为已读 |
| `DELETE` | `/v2/notification/{id}` | 删除通知 |
| `DELETE` | `/v2/notifications` | 批量删除通知 |

---

### 7.7 团队 (Teams)

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/project/{id}/members` | 获取项目团队成员 | 否 |
| `GET` | `/v2/team/{id}/members` | 获取团队成员 | 否 |
| `GET` | `/v2/teams` | 批量获取团队 | 否 |
| `POST` | `/v2/team/{id}/members` | 添加成员 | **是** |
| `POST` | `/v2/team/{id}/join` | 加入团队 | **是** |
| `DELETE` | `/v2/team/{id}/members/{user_id}` | 移除成员 | **是** |
| `PATCH` | `/v2/team/{id}/members/{user_id}` | 修改成员信息 | **是** |
| `PATCH` | `/v2/team/{id}/owner` | 转让所有权 | **是** |

---

### 7.8 举报/主题 (Reports/Threads)

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| `GET` | `/v2/report` | 获取公开的举报 | **是** |
| `POST` | `/v2/report` | 举报项目/用户/版本 | **是** |
| `GET` | `/v2/report/{id}` | 获取举报详情 | **是** |
| `PATCH` | `/v2/report/{id}` | 修改举报 | **是** |
| `GET` | `/v2/reports` | 批量获取举报 | **是** |
| `GET` | `/v2/thread/{id}` | 获取主题 | **是** |
| `POST` | `/v2/thread/{id}/message` | 发送消息 | **是** |
| `GET` | `/v2/threads` | 批量获取主题 | **是** |
| `DELETE` | `/v2/thread/{id}/message` | 删除消息 | **是** |

---

### 7.9 标签 (Tags)

所有标签端点均为公开 `GET` 请求，无需认证。适合应用启动时拉取一次并缓存。

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/v2/tag/category` | 获取分类列表 |
| `GET` | `/v2/tag/loader` | 获取加载器列表 |
| `GET` | `/v2/tag/game_version` | 获取 Minecraft 版本列表 |
| `GET` | `/v2/tag/license` | 获取许可证列表 *(deprecated)* |
| `GET` | `/v2/tag/license/{id}` | 获取指定许可证文本 |
| `GET` | `/v2/tag/donation_platform` | 获取赞助平台列表 |
| `GET` | `/v2/tag/report_type` | 获取举报类型列表 |
| `GET` | `/v2/tag/project_type` | 获取项目类型列表 |
| `GET` | `/v2/tag/side_type` | 获取 side type 列表 |

#### 各端点响应格式

**`/v2/tag/category`** → `Category[]` (见 [6.4](#64-标签类别-category))

**`/v2/tag/loader`** → `Loader[]` (见 [6.8](#68-加载器-loader))

**`/v2/tag/game_version`** → `GameVersion[]` (见 [6.7](#67-游戏版本-game-version))

**`/v2/tag/license`** → `LicenseTag[]` (见 [6.9](#69-许可证-license-tag)) *(deprecated)*

**`/v2/tag/donation_platform`** → `DonationPlatform[]` (见 [6.10](#610-赞助平台-donation-platform))

**`/v2/tag/project_type`** → `string[]` (见 [6.5](#65-项目类型-project_type))

**`/v2/tag/side_type`** → `string[]` (见 [6.6](#66-客户端服务端类型-side_type))

---

### 7.10 统计 (Statistics)

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/v2/statistics` | 获取 Modrinth 实例的各种统计数据 |

---

## 8. Mod 更新器最佳实践

基于上述 API 规范，开发 Modrinth mod 更新器时建议采用以下策略：

### 8.1 检查更新的推荐流程

```
1. 收集本地 mod 文件 → 计算 sha512 (或 sha1) 哈希
2. 获取当前配置的 loader(s) 和 Minecraft 版本号
3. POST /v2/version_files/update
   → 传入 { hashes, algorithm, loaders, game_versions }
4. 解析响应中哪些哈希有更新
5. 对于每个有更新的 mod:
   a. 从 files[0].url 下载新文件
   b. 使用 files[0].hashes 验证下载完整性
   c. 检查 dependencies 是否有新的必需依赖
```

### 8.2 哈希选择

- **推荐 sha512**: 更安全，降低碰撞风险
- sha1 可作为后备，但不是所有文件都有 sha1

### 8.3 缓存建议

启动时拉取一次 (可在运行时定期刷新):
- `/v2/tag/loader` — 加载器列表
- `/v2/tag/game_version` — 游戏版本列表
- `/v2/tag/category` — 分类列表

### 8.4 需要注意的行为

- `POST /v2/version_files/update` **只返回有更新的文件**，无更新的哈希不会出现在响应中
- 不要假设响应中的键序，用哈希值作为 key 来查找
- 同一项目可能有多个兼容的 loader/version，`loaders` 和 `game_versions` 参数会过滤
- 如果配置了多个 loader，建议分别在各自的 `loaders` 数组中查询
- 每个文件的 `files[]` 中 `primary: true` 的那个是你的主要下载目标
- 始终验证下载后文件的哈希

### 8.5 速率限制

- 每分钟 300 请求
- 批量端点 (`/v2/version_files/update`) 是你最好的朋友 — 用 1 次请求检查所有 mod
- 单个查询 (`/v2/version_file/{hash}/update`) 仅在没有批量需求时使用

### 8.6 User-Agent

务必设置:
```
yourname/mod-updater/1.0.0 (you@example.com)
```

---

## 参考链接

- 官方 API 文档: <https://docs.modrinth.com/api/>
- Labrinth 源码: <https://github.com/modrinth/labrinth>
- Scope 定义: <https://github.com/modrinth/labrinth/blob/master/src/models/pats.rs#L15>
- 支持邮箱: <support@modrinth.com>
