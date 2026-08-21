use anyhow::{Context, Result, bail};
use std::env;
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        bail!("este helper deve ser iniciado pelo Polkit");
    }

    let mut args = env::args_os().skip(1);
    let mode = args.next().and_then(|value| value.into_string().ok());
    if args.next().is_some() {
        bail!("argumentos adicionais não são permitidos");
    }

    let dnf_args: &[&str] = match mode.as_deref() {
        Some("online") => &["--refresh", "upgrade", "-y"],
        Some("offline") => &["--refresh", "upgrade", "--offline", "-y"],
        _ => bail!("modo permitido: online ou offline"),
    };

    let status = Command::new("/usr/bin/dnf5")
        .args(dnf_args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("LC_ALL", "C.UTF-8")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("não foi possível iniciar o DNF5")?;

    if !status.success() {
        bail!(
            "a atualização falhou com código {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
