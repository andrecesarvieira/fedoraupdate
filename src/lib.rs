use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

pub const APP_ID: &str = "org.fedoraupdate.FedoraUpdate";
pub const APP_NAME: &str = "FedoraUpdate";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageUpdate {
    pub name: String,
    pub arch: String,
    pub evr: String,
    pub repository: String,
    #[serde(default)]
    pub download_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCache {
    pub checked_at_unix: u64,
    pub updates: Vec<PackageUpdate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schedule {
    pub hour: u8,
    pub minute: u8,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            hour: 10,
            minute: 0,
        }
    }
}

impl Schedule {
    pub fn validate(self) -> Result<Self> {
        if self.hour > 23 || self.minute > 59 {
            bail!("Horário inválido: {:02}:{:02}", self.hour, self.minute);
        }
        Ok(self)
    }
}

fn xdg_path(variable: &str, fallback: &str) -> PathBuf {
    if let Some(value) = env::var_os(variable).filter(|value| !value.is_empty()) {
        return PathBuf::from(value);
    }
    let home = env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(fallback)
}

pub fn cache_path() -> PathBuf {
    xdg_path("XDG_CACHE_HOME", ".cache").join("fedoraupdate/updates.json")
}

pub fn config_path() -> PathBuf {
    xdg_path("XDG_CONFIG_HOME", ".config").join("fedoraupdate/config.json")
}

pub fn timer_dropin_path() -> PathBuf {
    xdg_path("XDG_CONFIG_HOME", ".config")
        .join("systemd/user/fedoraupdate-check.timer.d/schedule.conf")
}

pub fn parse_dnf_json(input: &str) -> Result<Vec<PackageUpdate>> {
    let root: Value = serde_json::from_str(input).context("resposta JSON inválida do DNF5")?;
    let sections = root
        .as_object()
        .context("a resposta do DNF5 não é um objeto JSON")?;
    let mut updates = Vec::new();

    for entries in sections.values() {
        let Some(entries) = entries.as_array() else {
            continue;
        };
        for entry in entries {
            let Some(object) = entry.as_object() else {
                continue;
            };
            let string = |key: &str| {
                object
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let update = PackageUpdate {
                name: string("name"),
                arch: string("arch"),
                evr: string("evr"),
                repository: string("repository"),
                download_size: object
                    .get("download_size")
                    .or_else(|| object.get("downloadsize"))
                    .or_else(|| object.get("size"))
                    .and_then(Value::as_u64),
            };
            if !update.name.is_empty() {
                updates.push(update);
            }
        }
    }

    updates.sort_by(|a, b| a.name.cmp(&b.name).then(a.arch.cmp(&b.arch)));
    updates.dedup();
    Ok(updates)
}

pub fn run_check() -> Result<Vec<PackageUpdate>> {
    let output = Command::new("/usr/bin/dnf5")
        .args(["--refresh", "check-upgrade", "--json"])
        .env("LC_ALL", "C.UTF-8")
        .output()
        .context("não foi possível executar o DNF5")?;

    let code = output.status.code();
    if code != Some(0) && code != Some(100) {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "DNF5 encerrou com código {}: {}",
            code.unwrap_or(-1),
            stderr
        );
    }
    parse_dnf_json(&String::from_utf8_lossy(&output.stdout))
}

pub fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn load_cache() -> Result<UpdateCache> {
    let contents =
        fs::read_to_string(cache_path()).context("ainda não há uma verificação salva")?;
    serde_json::from_str(&contents).context("cache de atualizações inválido")
}

pub fn save_cache(updates: Vec<PackageUpdate>) -> Result<bool> {
    let path = cache_path();
    let previous = load_cache().ok();
    let changed = previous
        .as_ref()
        .map(|old| old.updates != updates)
        .unwrap_or(true);
    let cache = UpdateCache {
        checked_at_unix: current_unix_time(),
        updates,
    };
    write_json_atomic(&path, &cache)?;
    Ok(changed)
}

pub fn load_schedule() -> Schedule {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Schedule>(&text).ok())
        .and_then(|schedule| schedule.validate().ok())
        .unwrap_or_default()
}

pub fn save_schedule(schedule: Schedule) -> Result<()> {
    let schedule = schedule.validate()?;
    write_json_atomic(&config_path(), &schedule)?;

    let dropin = timer_dropin_path();
    if let Some(parent) = dropin.parent() {
        fs::create_dir_all(parent).context("não foi possível criar a configuração do timer")?;
    }
    let contents = format!(
        "[Timer]\nOnCalendar=\nOnCalendar=*-*-* {:02}:{:02}:00\nPersistent=true\nRandomizedDelaySec=0\n",
        schedule.hour, schedule.minute
    );
    write_atomic(&dropin, contents.as_bytes())?;

    systemctl_user(["daemon-reload"])?;
    systemctl_user(["enable", "--now", "fedoraupdate-check.timer"])?;
    systemctl_user(["restart", "fedoraupdate-check.timer"])?;
    Ok(())
}

fn systemctl_user<const N: usize>(args: [&str; N]) -> Result<ExitStatus> {
    let status = Command::new("/usr/bin/systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("não foi possível configurar o agendamento")?;
    if !status.success() {
        bail!("systemd recusou a configuração do agendamento");
    }
    Ok(status)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let data = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &data)
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().context("caminho sem diretório pai")?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, data)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn needs_offline_update(updates: &[PackageUpdate]) -> bool {
    const CRITICAL: &[&str] = &["kernel", "systemd", "glibc", "dbus", "rpm", "dnf5"];
    updates.iter().any(|update| {
        CRITICAL
            .iter()
            .any(|prefix| update.name == *prefix || update.name.starts_with(&format!("{prefix}-")))
    })
}

pub fn notify(summary: &str, body: &str, urgency: &str) {
    let _ = Command::new("/usr/bin/notify-send")
        .args([
            "--app-name=FedoraUpdate",
            "--icon=org.fedoraupdate.FedoraUpdate-symbolic",
            "--urgency",
            urgency,
            summary,
            body,
        ])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_sorts_dnf5_output() {
        let input = r#"{
          "Available upgrades": [
            {"name":"zlib","arch":"x86_64","evr":"1.3-2.fc44","repository":"updates"},
            {"name":"bash","arch":"x86_64","evr":"5.2-1.fc44","repository":"updates"}
          ],
          "Obsoleting packages": []
        }"#;
        let packages = parse_dnf_json(input).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bash");
        assert_eq!(packages[1].repository, "updates");
    }

    #[test]
    fn recognizes_critical_packages() {
        let package = PackageUpdate {
            name: "kernel-core".into(),
            arch: "x86_64".into(),
            evr: "1".into(),
            repository: "updates".into(),
            download_size: None,
        };
        assert!(needs_offline_update(&[package]));
    }

    #[test]
    fn validates_schedule() {
        assert!(
            Schedule {
                hour: 23,
                minute: 59
            }
            .validate()
            .is_ok()
        );
        assert!(
            Schedule {
                hour: 24,
                minute: 0
            }
            .validate()
            .is_err()
        );
    }
}
