use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use modrinth_updater_core::{
    load_latest_report, parse_mod_names_from_file, run_updates, save_report, scan_directory,
    AppConfig, LocalMod, ModInfo, ModrinthClient,
};
use std::path::PathBuf;
use tracing::info;

// ── i18n / Localization ──────────────────────────────────────────────

/// Simple bilingual (EN + ZH) localization.
/// Detects system language from `LANG` env var at startup.
/// When Chinese is detected, prints English followed by Chinese on the next line.
struct L10n {
    zh: bool,
}

impl L10n {
    fn detect() -> Self {
        let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
        let lc_all = std::env::var("LC_ALL").unwrap_or_default().to_lowercase();
        let is_zh = lang.starts_with("zh") || lc_all.starts_with("zh");
        Self { zh: is_zh }
    }

    /// Return English only, or "EN\nZH" when Chinese locale.
    fn t(&self, en: &str, zh: &str) -> String {
        if self.zh {
            format!("{}\n{}", en, zh)
        } else {
            en.to_string()
        }
    }

    fn println(&self, en: &str, zh: &str) {
        println!("{}", self.t(en, zh));
    }

    fn eprintln(&self, en: &str, zh: &str) {
        eprintln!("{}", self.t(en, zh));
    }
}

// ── Default output directory for reports ─────────────────────────────

const DEFAULT_OUTPUT_DIR: &str = "./mods_update";

fn ensure_output_dir() -> PathBuf {
    let dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn default_report_path() -> PathBuf {
    let dir = ensure_output_dir();
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    dir.join(format!("report-{}.md", ts))
}

// ── CLI Definition ───────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "modrinth-updater",
    version = "0.1.0",
    about = "Update Minecraft mods via the Modrinth API",
    long_about = "Update Minecraft mods via the Modrinth API\n通过 Modrinth API 更新 Minecraft mods\n\nA cross-platform tool to check and apply updates for Minecraft mods.\n跨平台 Minecraft mod 更新工具，通过 Modrinth API 检查并应用更新。",
    after_help = "COMMON OPTIONS / 常用参数:\n  -d, --dir <DIR>          Mods directory / mods 目录\n  -l, --loader <LOADER>    Mod loader (fabric, forge, etc.) / 加载器 [default: fabric]\n  -g, --game-version <VER> Minecraft version / 游戏版本 [default: 1.21.1]\n  -n, --name <NAME>...     Search mods by name / 按名称搜索\n  -f, --from-file <FILE>   Read mod names from file / 从文件读取 mod 名称\n  -r, --recursive          Scan subdirectories / 递归扫描\n  -o, --output <PATH>      Report output path / 报告输出路径\n  -y, --yes                Skip confirm (update only) / 跳过确认",
    verbatim_doc_comment
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan mods and check for available updates (no files changed)
    /// 扫描 mods 并检查可用更新（不修改文件）
    #[command(verbatim_doc_comment)]
    Check {
        /// Directory containing mod .jar files
        /// mod .jar 文件所在的目录
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Search for mods by name (can be specified multiple times)
        /// 通过名字搜索 mod（可多次指定）
        #[arg(short, long)]
        name: Vec<String>,

        /// Read mod names from a text file (one per line)
        /// 从文本文件读取 mod 名称（每行一个）
        #[arg(short = 'f', long)]
        from_file: Option<PathBuf>,

        /// Mod loader: fabric, forge, quilt, neoforge, etc.
        /// 模组加载器
        #[arg(short, long, default_value = "fabric")]
        loader: String,

        /// Minecraft version
        /// Minecraft 版本
        #[arg(short, long, default_value = "1.21.1")]
        game_version: String,

        /// Scan subdirectories recursively
        /// 递归扫描子目录
        #[arg(short, long)]
        recursive: bool,

        /// Save a markdown report to this path
        /// 将 Markdown 报告保存到此路径
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Check for updates and apply them
    /// 检查并应用更新
    #[command(verbatim_doc_comment)]
    Update {
        /// Directory containing mod .jar files
        /// mod .jar 文件所在的目录
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Search for mods by name (can be specified multiple times)
        /// 通过名字搜索 mod（可多次指定）
        #[arg(short, long)]
        name: Vec<String>,

        /// Read mod names from a text file (one per line)
        /// 从文本文件读取 mod 名称（每行一个）
        #[arg(short = 'f', long)]
        from_file: Option<PathBuf>,

        /// Mod loader: fabric, forge, quilt, neoforge, etc.
        /// 模组加载器
        #[arg(short, long, default_value = "fabric")]
        loader: String,

        /// Minecraft version
        /// Minecraft 版本
        #[arg(short, long, default_value = "1.21.1")]
        game_version: String,

        /// Scan subdirectories recursively
        /// 递归扫描子目录
        #[arg(short, long)]
        recursive: bool,

        /// Only check, don't actually download or replace files
        /// 仅检查，不实际下载或替换文件
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        /// 跳过确认提示
        #[arg(short = 'y', long)]
        yes: bool,

        /// Save a markdown report to this path
        /// 将 Markdown 报告保存到此路径
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a markdown report from the last update run
    /// 从上次更新生成 Markdown 报告
    #[command(verbatim_doc_comment)]
    Report {
        /// Save report to this path (default: ./mods_update/)
        /// 保存报告路径（默认：./mods_update/）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show or modify configuration
    /// 显示或修改配置
    #[command(verbatim_doc_comment)]
    Config {
        /// Set default mods directory
        /// 设置默认 mods 目录
        #[arg(long)]
        set_dir: Option<PathBuf>,

        /// Set default loader
        /// 设置默认加载器
        #[arg(long)]
        set_loader: Option<String>,

        /// Set default game version
        /// 设置默认游戏版本
        #[arg(long)]
        set_game_version: Option<String>,
    },
}

// ── Entry Point ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("modrinth_updater_core=info,modrinth_updater_cli=info")),
        )
        .init();

    let l10n = L10n::detect();
    let cli = Cli::parse();

    match cli.command {
        Command::Check { dir, name, from_file, loader, game_version, recursive, output } => {
            cmd_check(&l10n, dir, name, from_file, loader, game_version, recursive, output).await?
        }
        Command::Update { dir, name, from_file, loader, game_version, recursive, dry_run, yes, output } => {
            cmd_update(&l10n, dir, name, from_file, loader, game_version, recursive, dry_run, yes, output).await?
        }
        Command::Report { output } => cmd_report(&l10n, output)?,
        Command::Config { set_dir, set_loader, set_game_version } => {
            cmd_config(&l10n, set_dir, set_loader, set_game_version)?
        }
    }

    Ok(())
}

// ── Command Implementations ──────────────────────────────────────────

async fn cmd_check(
    l10n: &L10n,
    dir: Option<PathBuf>,
    names: Vec<String>,
    from_file: Option<PathBuf>,
    loader: String,
    game_version: String,
    recursive: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    let config = AppConfig::load()?;
    let client = ModrinthClient::new(config.user_agent())?;

    let local_mods = gather_mods(l10n, dir, names, from_file, recursive, &config, &client).await?;

    if local_mods.is_empty() {
        l10n.println("No mods found to check.", "未找到任何 mod 可检查。");
        return Ok(());
    }

    println!(
        "{}",
        l10n.t(
            &format!("Checking {} mod(s) for updates (loader={}, game_version={})...", local_mods.len(), loader, game_version),
            &format!("正在检查 {} 个 mod 的更新 (加载器={}, 游戏版本={})...", local_mods.len(), loader, game_version),
        )
    );

    let mod_infos = modrinth_updater_core::analyze_mods(
        &client, &local_mods, &[loader.clone()], &[game_version.clone()],
    ).await?;

    let updates: Vec<&ModInfo> = mod_infos.iter().filter(|m| m.update_version.is_some()).collect();

    // Table headers
    let header_mod = l10n.t("Mod", "Mod 名称");
    let header_cur = l10n.t("Current Version", "当前版本");
    let header_lat = l10n.t("Latest Version", "最新版本");
    println!("┌────┬──────────────────────────────────────────────┬────────────────────┬────────────────────┐");
    println!("│ #  │ {:44} │ {:18} │ {:18} │", header_mod, header_cur, header_lat);
    println!("├────┼──────────────────────────────────────────────┼────────────────────┼────────────────────┤");

    for (i, m) in updates.iter().enumerate() {
        let name = truncate_str(&m.local.filename, 44);
        let current = m.current_version.as_ref().and_then(|v| v.version_number.as_deref()).unwrap_or("?");
        let latest = m.update_version.as_ref().and_then(|v| v.version_number.as_deref()).unwrap_or("?");
        let latest_type = m.update_version.as_ref().map(|v| format!("{:?}", v.version_type)).unwrap_or_default();
        println!(
            "│ {:2} │ {:44} │ {:18} │ {:>6} ({}) │",
            i + 1, name,
            truncate_str(current, 18),
            truncate_str(latest, 11),
            latest_type
        );
    }

    println!("└────┴──────────────────────────────────────────────┴────────────────────┴────────────────────┘");
    println!();
    l10n.println(
        &format!("{} mod(s) scanned, {} update(s) available, {} up-to-date", mod_infos.len(), updates.len(), mod_infos.len() - updates.len()),
        &format!("已扫描 {} 个 mod，{} 个有可用更新，{} 个已是最新", mod_infos.len(), updates.len(), mod_infos.len() - updates.len()),
    );

    let loaders = vec![loader.clone()];
    let versions = vec![game_version.clone()];
    let output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let report = run_updates(&client, &local_mods, &loaders, &versions, true, &output_dir).await?;
    save_report(&report)?;

    let out_path = output.unwrap_or_else(default_report_path);
    let md = report.to_markdown();
    std::fs::write(&out_path, &md).context("Failed to write report")?;
    l10n.println(&format!("Report saved to {:?}", out_path), &format!("报告已保存到 {:?}", out_path));

    Ok(())
}

async fn cmd_update(
    l10n: &L10n,
    dir: Option<PathBuf>,
    names: Vec<String>,
    from_file: Option<PathBuf>,
    loader: String,
    game_version: String,
    recursive: bool,
    dry_run: bool,
    yes: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    let config = AppConfig::load()?;
    let client = ModrinthClient::new(config.user_agent())?;

    let saved_dir = dir.clone();
    let local_mods = gather_mods(l10n, dir, names, from_file, recursive, &config, &client).await?;

    if local_mods.is_empty() {
        l10n.println("No mods found to update.", "未找到任何 mod 可更新。");
        return Ok(());
    }

    let mod_infos = modrinth_updater_core::analyze_mods(
        &client, &local_mods, &[loader.clone()], &[game_version.clone()],
    ).await?;

    let updates: Vec<&ModInfo> = mod_infos.iter().filter(|m| m.update_version.is_some()).collect();

    if updates.is_empty() {
        l10n.println(
            &format!("All {} mod(s) are up-to-date!", mod_infos.len()),
            &format!("全部 {} 个 mod 均已是最新版本！", mod_infos.len()),
        );
        return Ok(());
    }

    l10n.println(
        &format!("{} update(s) available out of {} mod(s):\n", updates.len(), mod_infos.len()),
        &format!("在 {} 个 mod 中发现 {} 个可用更新：\n", mod_infos.len(), updates.len()),
    );

    for (i, m) in updates.iter().enumerate() {
        let current = m.current_version.as_ref().and_then(|v| v.version_number.as_deref()).unwrap_or("?");
        let latest = m.update_version.as_ref().and_then(|v| v.version_number.as_deref()).unwrap_or("?");
        println!("  {}. {}  {} -> {}", i + 1, m.local.filename, current, latest);
    }

    if dry_run {
        l10n.println("\nDry run — no files will be modified.", "\n试运行 — 不会修改任何文件。");
        return Ok(());
    }

    if !yes {
        l10n.println("\nApply these updates? [y/N] ", "\n是否应用这些更新？[y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            l10n.println("Cancelled.", "已取消。");
            return Ok(());
        }
    }

    let pb = ProgressBar::new(updates.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let loaders = vec![loader.clone()];
    let versions = vec![game_version.clone()];
    let output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
    let report = run_updates(&client, &local_mods, &loaders, &versions, false, &output_dir).await?;

    pb.finish_with_message(l10n.t("Update complete", "更新完成"));

    println!();
    let summary_title = l10n.t("Update Summary", "更新摘要");
    println!("═══════════════════════════════════");
    println!("  {}", summary_title);
    println!("═══════════════════════════════════");
    println!("  {}    {}", l10n.t("Total scanned", "已扫描"), report.total_scanned);
    println!("  {}    {}", l10n.t("Updates found", "发现更新"), report.total_updates_available);
    println!("  {}     {}", l10n.t("Successfully", "更新成功"), report.total_updated);
    println!("  {}           {}", l10n.t("Failed", "更新失败"), report.total_failed);
    println!("  {}  {}", l10n.t("Already current", "已是最新"), report.total_up_to_date);
    println!("═══════════════════════════════════");

    if report.total_failed > 0 {
        l10n.println("\nFailed updates:", "\n更新失败：");
        for r in &report.results {
            if !r.success {
                println!("  ✗ {} — {}",
                    r.filename,
                    r.error.as_deref().unwrap_or("Unknown error / 未知错误"));
            }
        }
    }

    save_report(&report)?;

    let out_path = output.unwrap_or_else(default_report_path);
    let md = report.to_markdown();
    std::fs::write(&out_path, &md).context("Failed to write report")?;
    l10n.println(&format!("\nReport saved to {:?}", out_path), &format!("\n报告已保存到 {:?}", out_path));

    let mut config = AppConfig::load()?;
    config.last_loader = Some(loader);
    config.last_game_version = Some(game_version);
    if let Some(d) = saved_dir {
        config.last_mods_dir = Some(d);
    }
    config.save()?;

    Ok(())
}

fn cmd_report(l10n: &L10n, output: Option<PathBuf>) -> Result<()> {
    match load_latest_report()? {
        Some(report) => {
            let md = report.to_markdown();
            let out_path = output.unwrap_or_else(default_report_path);
            std::fs::write(&out_path, &md).context("Failed to write report")?;
            l10n.println(&format!("Report saved to {:?}", out_path), &format!("报告已保存到 {:?}", out_path));
        }
        None => {
            l10n.println(
                "No previous report found. Run 'check' or 'update' first.",
                "未找到之前的报告。请先运行 'check' 或 'update'。",
            );
        }
    }
    Ok(())
}

fn cmd_config(
    l10n: &L10n,
    set_dir: Option<PathBuf>,
    set_loader: Option<String>,
    set_game_version: Option<String>,
) -> Result<()> {
    let mut config = AppConfig::load()?;

    let mut changed = false;
    if let Some(dir) = set_dir { config.last_mods_dir = Some(dir); changed = true; }
    if let Some(loader) = set_loader { config.last_loader = Some(loader); changed = true; }
    if let Some(version) = set_game_version { config.last_game_version = Some(version); changed = true; }

    if changed {
        config.save()?;
        l10n.println("Configuration updated.", "配置已更新。");
    }

    println!();
    l10n.println("Current configuration:", "当前配置：");
    println!("  {}  {}",
        l10n.t("Mods directory:", "Mods 目录："),
        config.last_mods_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(not set)".to_string()));
    println!("  {}          {}",
        l10n.t("Loader:", "加载器："),
        config.last_loader.as_deref().unwrap_or("(not set)"));
    println!("  {}    {}",
        l10n.t("Game version:", "游戏版本："),
        config.last_game_version.as_deref().unwrap_or("(not set)"));

    Ok(())
}

// ── Helper: Gather mods from various sources ─────────────────────────

async fn gather_mods(
    l10n: &L10n,
    dir: Option<PathBuf>,
    names: Vec<String>,
    from_file: Option<PathBuf>,
    recursive: bool,
    config: &AppConfig,
    client: &ModrinthClient,
) -> Result<Vec<LocalMod>> {
    let mut local_mods = Vec::new();

    let actual_dir = dir.or_else(|| config.last_mods_dir.clone());
    if let Some(ref d) = actual_dir {
        info!("Scanning directory: {:?}", d);
        local_mods = scan_directory(d, recursive)?;
    }

    if !names.is_empty() {
        l10n.println(
            &format!("Searching for {} mod(s) by name...", names.len()),
            &format!("正在按名称搜索 {} 个 mod...", names.len()),
        );
        for name in &names {
            match client.search_project(name).await {
                Ok(Some(hit)) => {
                    l10n.println(&format!("  Found: {} ({})", hit.title, hit.slug),
                        &format!("  找到: {} ({})", hit.title, hit.slug));
                }
                Ok(None) => {
                    l10n.println(&format!("  Not found: {}", name), &format!("  未找到: {}", name));
                }
                Err(e) => {
                    l10n.eprintln(&format!("  Error searching '{}': {}", name, e),
                        &format!("  搜索 '{}' 出错: {}", name, e));
                }
            }
        }
        if actual_dir.is_none() {
            l10n.println(
                "\nNote: Searching by name only shows results. To check for updates,\nuse --dir to scan your mods folder with actual .jar files.\n",
                "\n提示: 按名称搜索仅展示结果。如需检查更新，\n请使用 --dir 扫描包含 .jar 文件的 mods 文件夹。\n",
            );
        }
    }

    if let Some(file) = from_file {
        let file_names = parse_mod_names_from_file(&file)?;
        l10n.println(
            &format!("Read {} mod names from {:?}", file_names.len(), file),
            &format!("从 {:?} 读取了 {} 个 mod 名称", file, file_names.len()),
        );
        for name in &file_names {
            match client.search_project(name).await {
                Ok(Some(hit)) => {
                    l10n.println(&format!("  Found: {} ({})", hit.title, hit.slug),
                        &format!("  找到: {} ({})", hit.title, hit.slug));
                }
                Ok(None) => {
                    l10n.println(&format!("  Not found: {}", name), &format!("  未找到: {}", name));
                }
                Err(e) => {
                    l10n.eprintln(&format!("  Error searching '{}': {}", name, e),
                        &format!("  搜索 '{}' 出错: {}", name, e));
                }
            }
        }
        if actual_dir.is_none() {
            l10n.println(
                "\nNote: File-based search only shows results. To check for updates,\nuse --dir to scan your mods folder with actual .jar files.\n",
                "\n提示: 基于文件的搜索仅展示结果。如需检查更新，\n请使用 --dir 扫描包含 .jar 文件的 mods 文件夹。\n",
            );
        }
    }

    Ok(local_mods)
}

// ── Utilities ────────────────────────────────────────────────────────

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    } else {
        s.to_string()
    }
}
