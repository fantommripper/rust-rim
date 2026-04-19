use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub const RIMWORLD_APP_ID: &str = "294100";

// ─── События ─────────────────────────────────────────────────────────────────

pub enum DownloadEvent {
    Log(String),
    ItemStarted(u64),
    ItemDone(u64),
    ItemFailed(u64),
    Finished { failed: Vec<u64> },
}

pub enum InstallEvent {
    Log(String),
    Done,
    Error(String),
}

// ─── Пути ────────────────────────────────────────────────────────────────────

pub fn steamcmd_dir(base: &Path) -> PathBuf {
    base.join("steamcmd")
}

pub fn steamcmd_executable(base: &Path) -> PathBuf {
    let dir = steamcmd_dir(base);
    if cfg!(target_os = "windows") {
        dir.join("steamcmd.exe")
    } else {
        dir.join("steamcmd.sh")
    }
}

/// Папка, куда SteamCMD скачивает моды Workshop:
/// `{base}/steam/steamapps/workshop/content/294100/`
pub fn steam_content_path(base: &Path) -> PathBuf {
    base.join("steam")
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join(RIMWORLD_APP_ID)
}

pub fn is_installed(base: &Path) -> bool {
    if is_nixos() {
        find_system_steamcmd().is_some()
    } else {
        steamcmd_executable(base).exists()
    }
}

/// Возвращает `true` на NixOS (по наличию `/etc/NIXOS`).
pub fn is_nixos() -> bool {
    std::path::Path::new("/etc/NIXOS").exists()
}

/// Ищет системный бинарник `steamcmd` (nixpkgs) в PATH.
fn find_system_steamcmd() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("steamcmd");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ─── Установка ───────────────────────────────────────────────────────────────

pub fn install_async(base: PathBuf, tx: mpsc::Sender<InstallEvent>) {
    std::thread::spawn(move || {
        if let Err(e) = run_install(&base, &tx) {
            let _ = tx.send(InstallEvent::Error(e.to_string()));
        }
    });
}

fn run_install(base: &Path, tx: &mpsc::Sender<InstallEvent>) -> anyhow::Result<()> {
    let install_dir = steamcmd_dir(base);
    std::fs::create_dir_all(&install_dir)?;

    let url = if cfg!(target_os = "windows") {
        "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip"
    } else if cfg!(target_os = "macos") {
        "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_osx.tar.gz"
    } else {
        "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz"
    };

    let _ = tx.send(InstallEvent::Log(format!("Загрузка: {url}")));

    let bytes = download_bytes(url)?;

    let _ = tx.send(InstallEvent::Log(format!(
        "Распаковка ({} МБ)...",
        bytes.len() / 1_048_576
    )));

    if cfg!(target_os = "windows") {
        extract_zip(&bytes, &install_dir)?;
    } else {
        extract_tar_gz(&bytes, &install_dir)?;
        set_executable(&steamcmd_executable(base));
    }

    if steamcmd_executable(base).exists() {
        let _ = tx.send(InstallEvent::Log("SteamCMD успешно установлен.".into()));
        let _ = tx.send(InstallEvent::Done);
    } else {
        return Err(anyhow::anyhow!(
            "Исполняемый файл не найден после распаковки"
        ));
    }
    Ok(())
}

fn download_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP ошибка: {e}"))?;
    let mut buf = Vec::new();
    response.into_reader().read_to_end(&mut buf)?;
    Ok(buf)
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) -> anyhow::Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    let gz = GzDecoder::new(std::io::Cursor::new(bytes));
    let mut archive = Archive::new(gz);
    archive.unpack(dest)?;
    Ok(())
}

fn extract_zip(bytes: &[u8], dest: &Path) -> anyhow::Result<()> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    archive.extract(dest)?;
    Ok(())
}

#[allow(unused_variables)]
fn set_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
        }
    }
}

// ─── Скачивание модов ─────────────────────────────────────────────────────────

pub fn download_mods_async(
    base: PathBuf,
    ids: Vec<u64>,
    validate: bool,
    tx: mpsc::Sender<DownloadEvent>,
) {
    std::thread::spawn(move || {
        if let Err(e) = run_download(&base, &ids, validate, &tx) {
            let _ = tx.send(DownloadEvent::Log(format!("Критическая ошибка: {e}")));
            let _ = tx.send(DownloadEvent::Finished { failed: ids });
        }
    });
}

fn run_download(
    base: &Path,
    ids: &[u64],
    validate: bool,
    tx: &mpsc::Sender<DownloadEvent>,
) -> anyhow::Result<()> {
    // На NixOS используем системный steamcmd из nixpkgs (уже пропатчен).
    let (exe, is_system) = if is_nixos() {
        match find_system_steamcmd() {
            Some(p) => (p, true),
            None => return Err(anyhow::anyhow!(
                "NixOS: steamcmd не найден в PATH.\n\
                 Установите через nixpkgs: nix-shell -p steamcmd"
            )),
        }
    } else {
        (steamcmd_executable(base), false)
    };
    let steam_path = base.join("steam");
    std::fs::create_dir_all(&steam_path)?;

    let _ = tx.send(DownloadEvent::Log(format!(
        "SteamCMD: {} {}",
        exe.display(),
        if is_system { "(системный)" } else { "" }
    )));
    let _ = tx.send(DownloadEvent::Log(format!(
        "Скачиваем {} мод(ов)...",
        ids.len()
    )));

    // ── Запуск SteamCMD (аргументы вместо скрипта — совместимо с NixOS FHS) ──
    let steam_path_str = steam_path.to_string_lossy().replace('\\', "/");
    let (mut cmd, wrapped) = if is_system {
        (std::process::Command::new(&exe), false)
    } else {
        steamcmd_command(&exe)
    };
    if wrapped {
        let _ = tx.send(DownloadEvent::Log("(обёрнут в steam-run для FHS-совместимости)".into()));
    }
    cmd.arg("+force_install_dir").arg(&steam_path_str);
    cmd.arg("+login").arg("anonymous");
    for &id in ids {
        if validate {
            cmd.arg("+workshop_download_item")
                .arg(RIMWORLD_APP_ID)
                .arg(id.to_string())
                .arg("validate");
        } else {
            cmd.arg("+workshop_download_item")
                .arg(RIMWORLD_APP_ID)
                .arg(id.to_string());
        }
    }
    cmd.arg("+quit");

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Не удалось запустить SteamCMD: {e}"))?;

    // Отдельный поток для чтения stderr (предотвращает deadlock)
    let tx_err = tx.clone();
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines().flatten() {
                for part in split_ansi_lines(&line) {
                    if !part.is_empty() {
                        let _ = tx_err.send(DownloadEvent::Log(part));
                    }
                }
            }
        });
    }

    // ── Чтение stdout и разбор прогресса ────────────────────────────────────
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("stdout не настроен"))?;
    let reader = std::io::BufReader::new(stdout);

    let mut failed: Vec<u64> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // SteamCMD иногда объединяет несколько событий в одну строку через ANSI-коды.
        // Разбиваем по ANSI-границам и \r, проверяем все паттерны независимо.
        for part in split_ansi_lines(&line) {
            if part.is_empty() { continue; }

            let _ = tx.send(DownloadEvent::Log(part.clone()));

            // Не используем else-if: одна часть строки может содержать и Success, и Downloading
            if let Some(id) = parse_downloading_id(&part) {
                let _ = tx.send(DownloadEvent::ItemStarted(id));
            }
            if let Some(id) = parse_success_id(&part) {
                let _ = tx.send(DownloadEvent::ItemDone(id));
            }
            if let Some(id) = parse_error_id(&part) {
                if !failed.contains(&id) {
                    failed.push(id);
                }
                let _ = tx.send(DownloadEvent::ItemFailed(id));
            }
        }
    }

    let status = child.wait()?;

    // Ненулевой код выхода — считаем все моды неудачными (напр. steamcmd не запустился).
    if !status.success() && failed.is_empty() {
        let _ = tx.send(DownloadEvent::Log(format!(
            "✕ SteamCMD завершился с ошибкой (код {})",
            status.code().unwrap_or(-1)
        )));
        let all_ids: Vec<u64> = ids.to_vec();
        let _ = tx.send(DownloadEvent::Finished { failed: all_ids });
        return Ok(());
    }

    let _ = tx.send(DownloadEvent::Finished { failed });
    Ok(())
}

// ─── NixOS / steam-run ────────────────────────────────────────────────────────

/// На NixOS бинарники ELF требуют FHS-окружения.
/// `steam-run` (пакет `steam` в nixpkgs) создаёт его.
/// Ищет `steam-run` в PATH без запуска подпроцессов.
#[cfg(target_os = "linux")]
fn find_steam_run() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("steam-run");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn find_steam_run() -> Option<PathBuf> { None }

/// Создаёт `Command` для запуска `exe` (steamcmd).
/// На Linux при наличии `steam-run` оборачивает в него (нужно для NixOS).
fn steamcmd_command(exe: &Path) -> (std::process::Command, bool) {
    if let Some(steam_run) = find_steam_run() {
        let mut cmd = std::process::Command::new(steam_run);
        cmd.arg(exe);
        (cmd, true)
    } else {
        (std::process::Command::new(exe), false)
    }
}

// ─── Очистка ANSI и разбиение строк ──────────────────────────────────────────

/// Убирает ANSI escape-последовательности вида ESC [ ... <letter>.
fn strip_ansi_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() { break; }
                }
            }
            // bare ESC без '[' — просто пропускаем
        } else {
            out.push(c);
        }
    }
    out
}

/// Зачищает ANSI, делит по '\r', возвращает непустые обрезанные части.
fn split_ansi_lines(raw: &str) -> Vec<String> {
    let clean = strip_ansi_codes(raw);
    clean.split('\r')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ─── Вспомогательные парсеры вывода SteamCMD ─────────────────────────────────

fn parse_id_after(line: &str, prefix: &str) -> Option<u64> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
}

fn parse_downloading_id(line: &str) -> Option<u64> {
    parse_id_after(line, "Downloading item ")
}

fn parse_success_id(line: &str) -> Option<u64> {
    parse_id_after(line, "Success. Downloaded item ")
}

fn parse_error_id(line: &str) -> Option<u64> {
    parse_id_after(line, "ERROR! Download item ")
}
