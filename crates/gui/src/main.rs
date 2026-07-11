#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use modsupdater_core::{
    analyze_mods, install_update, scan_directory, AppConfig, ModInfo, ModrinthClient, UpdateReport,
};
use std::path::PathBuf;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

// ── Async bridge ────────────────────────────────────────────────────

enum Command {
    FetchTags,
    ScanAndCheck { dir: PathBuf, loader: String, game_version: String },
    DownloadSelected { indices: Vec<usize>, output_dir: PathBuf },
}

enum AppEvent {
    TagsLoaded { loaders: Vec<String>, game_versions: Vec<String> },
    ScanStarted,
    ScanComplete { count: usize },
    CheckStarted,
    CheckComplete { mod_infos: Vec<ModInfo> },
    DownloadProgress { index: usize, filename: String, pct: f32 },
    DownloadComplete { success: bool, error: Option<String> },
    AllDownloadsComplete { report: UpdateReport },
    Error(String),
}

// ── App state ──────────────────────────────────────────────────────

enum AppState {
    Idle,
    Scanning,
    Checking,
    Results { mod_infos: Vec<ModInfo>, selected: Vec<bool> },
    Downloading { mod_infos: Vec<ModInfo>, progress: Vec<(usize, String, f32)>, completed: usize, failed: usize, errors: Vec<String> },
    Done { report: UpdateReport },
}

// ── Main App ───────────────────────────────────────────────────────

pub struct ModUpdaterApp {
    cmd_tx: UnboundedSender<Command>,
    event_rx: UnboundedReceiver<AppEvent>,
    mods_dir: String,
    loader: String,
    game_version: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
    state: AppState,
    status: String,
}

impl ModUpdaterApp {
    fn new(cmd_tx: UnboundedSender<Command>, event_rx: UnboundedReceiver<AppEvent>) -> Self {
        Self {
            cmd_tx, event_rx,
            mods_dir: String::new(),
            loader: "fabric".into(),
            game_version: "1.21.1".into(),
            loaders: vec!["fabric".into()],
            game_versions: vec!["1.21.1".into()],
            state: AppState::Idle,
            status: "Ready. Select a mods directory and click 'Check Updates'.".into(),
        }
    }

    fn process_events(&mut self) {
        while let Ok(e) = self.event_rx.try_recv() {
            match e {
                AppEvent::TagsLoaded { loaders, game_versions } => {
                    if !loaders.contains(&self.loader) { self.loader.clone_from(&loaders[0]); }
                    if !game_versions.contains(&self.game_version) { self.game_version.clone_from(&game_versions[0]); }
                    self.loaders = loaders;
                    self.game_versions = game_versions;
                }
                AppEvent::ScanStarted => { self.state = AppState::Scanning; self.status = "Scanning...".into(); }
                AppEvent::ScanComplete { count } => { self.status = format!("Found {} mod(s), checking API...", count); }
                AppEvent::CheckStarted => { self.state = AppState::Checking; }
                AppEvent::CheckComplete { mod_infos } => {
                    let n = mod_infos.len();
                    let u = mod_infos.iter().filter(|m| m.update_version.is_some()).count();
                    self.status = format!("{} mod(s) scanned, {} update(s) available.", n, u);
                    let sel = vec![true; n];
                    self.state = AppState::Results { mod_infos, selected: sel };
                }
                AppEvent::DownloadProgress { index, filename, pct } => {
                    if let AppState::Downloading { ref mut progress, .. } = self.state {
                        if let Some(e) = progress.iter_mut().find(|(i,..)| *i == index) { e.1 = filename; e.2 = pct; }
                        else { progress.push((index, filename, pct)); }
                    }
                }
                AppEvent::DownloadComplete { success, error } => {
                    if let AppState::Downloading { ref mut completed, ref mut failed, ref mut errors, .. } = self.state {
                        if success { *completed += 1; } else { *failed += 1; }
                        if let Some(e) = error { errors.push(e); }
                    }
                }
                AppEvent::AllDownloadsComplete { report } => {
                    self.status = format!("Done: {} updated, {} failed. Files in ./mods_update/", report.total_updated, report.total_failed);
                    self.state = AppState::Done { report };
                }
                AppEvent::Error(msg) => { self.status = format!("Error: {}", msg); }
            }
        }
    }
}

impl eframe::App for ModUpdaterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();
        if matches!(self.state, AppState::Scanning | AppState::Checking | AppState::Downloading { .. }) {
            ctx.request_repaint();
        }

        // ── Destructure ─────────────────────────────────────────────
        let Self { cmd_tx, mods_dir, loader, game_version, loaders: _, game_versions: _, state, status, .. } = self;

        // ── Top panel ───────────────────────────────────────────────
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("ModsUpdater");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.text_edit_singleline(mods_dir);
                if ui.button("Browse...").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        *mods_dir = p.display().to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Loader:");
                ui.text_edit_singleline(loader);
                ui.separator();
                ui.label("Game Version:");
                ui.text_edit_singleline(game_version);
            });
            let busy = matches!(state, AppState::Scanning | AppState::Checking);
            let can = !mods_dir.is_empty() && !busy;
            ui.horizontal(|ui| {
                if ui.add_enabled(can, egui::Button::new("🔍 Check Updates")).clicked() {
                    let _ = cmd_tx.send(Command::ScanAndCheck {
                        dir: PathBuf::from(mods_dir.clone()),
                        loader: loader.clone(),
                        game_version: game_version.clone(),
                    });
                    *state = AppState::Scanning;
                }
                if busy { ui.add(egui::Spinner::new()); }
            });
            ui.separator();
        });

        // ── Central panel ───────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&*status);
            ui.separator();
            match state {
                AppState::Idle => {
                    ui.centered_and_justified(|ui| { ui.label("Select a directory and click 'Check Updates'."); });
                }
                AppState::Scanning | AppState::Checking => {
                    ui.centered_and_justified(|ui| { ui.add(egui::Spinner::new()); });
                }
                AppState::Results { mod_infos, selected } => {
                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                        egui::Grid::new("results").striped(true).show(ui, |ui| {
                            ui.label(""); ui.label(egui::RichText::new("Mod Name").strong());
                            ui.label(egui::RichText::new("Current").strong());
                            ui.label(egui::RichText::new("Latest").strong());
                            ui.label(egui::RichText::new("Type").strong());
                            ui.end_row();
                            for (i, m) in mod_infos.iter().enumerate() {
                                let has = m.update_version.is_some();
                                let c = if has { egui::Color32::GREEN } else { egui::Color32::GRAY };
                                ui.checkbox(&mut selected[i], "");
                                ui.colored_label(c, &m.local.filename);
                                ui.label(m.current_version.as_ref().and_then(|v| v.version_number.as_deref()).unwrap_or("?"));
                                if let Some(ref u) = m.update_version {
                                    ui.colored_label(c, u.version_number.as_deref().unwrap_or("?"));
                                } else { ui.colored_label(egui::Color32::GRAY, "—"); }
                                ui.label(m.update_version.as_ref().map(|v| format!("{:?}", v.version_type)).unwrap_or_default());
                                ui.end_row();
                            }
                        });
                    });
                }
                AppState::Downloading { mod_infos, progress, completed, failed, errors, .. } => {
                    ui.label(format!("{} / {} completed, {} failed", completed, mod_infos.len(), failed));
                    ui.separator();
                    for (_idx, name, pct) in progress {
                        let n = name.clone();
                        let p = *pct;
                        ui.horizontal(|ui| {
                            ui.label(&n);
                            ui.add(egui::ProgressBar::new(p).desired_width(200.0).text(format!("{:.0}%", p * 100.0)));
                        });
                    }
                    if !errors.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new("Errors:").color(egui::Color32::RED));
                        for e in errors { ui.label(format!("  ✗ {}", e)); }
                    }
                }
                AppState::Done { report } => {
                    ui.label(egui::RichText::new("✅ Update Complete!").strong());
                    egui::Grid::new("summary").striped(true).show(ui, |ui| {
                        ui.label("Total scanned:"); ui.label(report.total_scanned.to_string()); ui.end_row();
                        ui.label("Updates available:"); ui.label(report.total_updates_available.to_string()); ui.end_row();
                        ui.label("Succeeded:");
                        ui.label(egui::RichText::new(report.total_updated.to_string()).color(egui::Color32::GREEN)); ui.end_row();
                        ui.label("Failed:");
                        ui.label(egui::RichText::new(report.total_failed.to_string()).color(egui::Color32::RED)); ui.end_row();
                        ui.label("Up-to-date:"); ui.label(report.total_up_to_date.to_string()); ui.end_row();
                    });
                    ui.separator();
                    ui.label("Files saved to ./mods_update/");
                }
            }
        });

        // ── Bottom panel ────────────────────────────────────────────
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.separator();
            match state {
                AppState::Results { mod_infos, selected } => {
                    let indices: Vec<usize> = mod_infos.iter().enumerate()
                        .filter(|(i, m)| selected[*i] && m.update_version.is_some())
                        .map(|(i, _)| i).collect();
                    let n = indices.len();
                    let has = mod_infos.iter().any(|m| m.update_version.is_some());

                    let mut do_download = false;
                    let mut do_select_all = false;
                    let mut do_deselect_all = false;

                    ui.horizontal(|ui| {
                        if ui.add_enabled(n > 0, egui::Button::new(format!("⬇ Update Selected ({})", n))).clicked() { do_download = true; }
                        if ui.add_enabled(has, egui::Button::new("Select All")).clicked() { do_select_all = true; }
                        if ui.button("Deselect All").clicked() { do_deselect_all = true; }
                    });

                    // Mutate selection first (while state is still Results)
                    if do_select_all { for s in selected.iter_mut() { *s = true; } }
                    if do_deselect_all { for s in selected.iter_mut() { *s = false; } }

                    // Transition state last (after releasing selected borrow)
                    if do_download {
                        let _ = cmd_tx.send(Command::DownloadSelected { indices, output_dir: PathBuf::from("./mods_update") });
                        let mi = mod_infos.clone();
                        *state = AppState::Downloading {
                            mod_infos: mi,
                            progress: vec![], completed: 0, failed: 0, errors: vec![],
                        };
                    }
                }
                AppState::Done { .. } => {
                    if ui.button("🔍 Check Again").clicked() { *state = AppState::Idle; }
                }
                _ => {}
            }
        });
    }
}

// ── Background worker ───────────────────────────────────────────────

fn start_worker(event_tx: UnboundedSender<AppEvent>) -> UnboundedSender<Command> {
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Command>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = AppConfig::load().unwrap_or_default();
            let client = match ModrinthClient::new(config.user_agent()) {
                Ok(c) => c,
                Err(e) => { let _ = event_tx.send(AppEvent::Error(e.to_string())); return; }
            };
            let mut stored: Vec<ModInfo> = Vec::new();

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    Command::FetchTags => {
                        let loaders: Vec<String> = client.get_loaders().await
                            .map(|l| l.into_iter().map(|x| x.name).collect())
                            .unwrap_or_else(|_| vec!["fabric".into()]);
                        let mut gv: Vec<String> = client.get_game_versions().await
                            .map(|v| { let mut vs: Vec<_> = v.into_iter().filter(|g| g.major).map(|g| g.version).collect(); vs.sort_by(|a,b| b.cmp(a)); vs })
                            .unwrap_or_else(|_| vec!["1.21.1".into()]);
                        if !gv.contains(&"1.21.1".into()) { gv.push("1.21.1".into()); }
                        let _ = event_tx.send(AppEvent::TagsLoaded { loaders, game_versions: gv });
                    }
                    Command::ScanAndCheck { dir, loader, game_version } => {
                        let _ = event_tx.send(AppEvent::ScanStarted);
                        let local = match scan_directory(&dir, false) {
                            Ok(m) => m,
                            Err(e) => { let _ = event_tx.send(AppEvent::Error(e.to_string())); continue; }
                        };
                        let _ = event_tx.send(AppEvent::ScanComplete { count: local.len() });
                        if local.is_empty() { stored.clear(); let _ = event_tx.send(AppEvent::CheckComplete { mod_infos: vec![] }); continue; }
                        let _ = event_tx.send(AppEvent::CheckStarted);
                        match analyze_mods(&client, &local, &[loader], &[game_version]).await {
                            Ok(infos) => { stored = infos.clone(); let _ = event_tx.send(AppEvent::CheckComplete { mod_infos: infos }); }
                            Err(e) => { let _ = event_tx.send(AppEvent::Error(e.to_string())); }
                        }
                    }
                    Command::DownloadSelected { indices, output_dir } => {
                        let mut ok = 0usize; let mut fail = 0usize;
                        for &idx in &indices {
                            let info = match stored.get(idx) {
                                Some(i) => i,
                                None => { fail += 1; continue; }
                            };
                            if info.update_version.is_none() { fail += 1; continue; }
                            let _ = event_tx.send(AppEvent::DownloadProgress { index: idx, filename: info.local.filename.clone(), pct: 0.5 });
                            match install_update(&client, info, &output_dir).await {
                                Ok(r) => {
                                    if r.success {
                                        ok += 1;
                                        let _ = event_tx.send(AppEvent::DownloadProgress { index: idx, filename: info.local.filename.clone(), pct: 1.0 });
                                        let _ = event_tx.send(AppEvent::DownloadComplete { success: true, error: None });
                                    } else {
                                        fail += 1;
                                        let _ = event_tx.send(AppEvent::DownloadComplete { success: false, error: r.error });
                                    }
                                }
                                Err(e) => { fail += 1; let _ = event_tx.send(AppEvent::DownloadComplete { success: false, error: Some(e.to_string()) }); }
                            }
                        }
                        let report = UpdateReport {
                            total_scanned: stored.len(), total_updates_available: indices.len(),
                            total_updated: ok, total_failed: fail, total_up_to_date: stored.len().saturating_sub(indices.len()),
                            results: vec![], loader: String::new(), game_version: String::new(),
                            timestamp: chrono::Utc::now(),
                        };
                        let _ = event_tx.send(AppEvent::AllDownloadsComplete { report });
                    }
                }
            }
        });
    });
    cmd_tx
}

// ── App icon ────────────────────────────────────────────────────────

fn app_icon() -> egui::IconData {
    let w = 32u32;
    let h = 32u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let dx = x as i32 - cx;
            let dy = y as i32 - cy;
            // Rounded square background
            let dist = (dx * dx + dy * dy) as f32;
            if dist < (cx * cx) as f32 {
                // Dark bg
                rgba[idx] = 26;
                rgba[idx + 1] = 26;
                rgba[idx + 2] = 46;
            } else {
                rgba[idx + 3] = 0;
            }
            // Green upward arrow (triangle)
            let top = -8;
            let bottom = 8;
            let mid = dx;
            let in_tri = dy < bottom && dy > top
                && dy > mid.abs() * 2 - 8
                && dy < (bottom - mid.abs() * 2 + 2);
            if in_tri {
                rgba[idx] = 74;
                rgba[idx + 1] = 222;
                rgba[idx + 2] = 128;
            }
            rgba[idx + 3] = 255;
        }
    }
    egui::IconData { rgba, width: w, height: h }
}

// ── main ────────────────────────────────────────────────────────────

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();
    let (event_tx, event_rx) = unbounded_channel();
    let cmd_tx = start_worker(event_tx);
    let _ = cmd_tx.send(Command::FetchTags);
    let icon = app_icon();
    eframe::run_native(
        "ModsUpdater",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([920.0, 620.0])
                .with_icon(icon),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(ModUpdaterApp::new(cmd_tx, event_rx)))),
    )
}
