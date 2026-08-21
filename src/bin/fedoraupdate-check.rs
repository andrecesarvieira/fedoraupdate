use anyhow::Result;
use fedoraupdate::{notify, run_check, save_cache};

fn main() -> Result<()> {
    match run_check() {
        Ok(updates) => {
            let count = updates.len();
            let changed = save_cache(updates)?;
            if changed && count > 0 {
                let body = if count == 1 {
                    "Há 1 atualização disponível.".to_owned()
                } else {
                    format!("Há {count} atualizações disponíveis.")
                };
                notify("Atualizações disponíveis", &body, "normal");
            }
            Ok(())
        }
        Err(error) => {
            notify(
                "Falha ao verificar atualizações",
                &error.to_string(),
                "critical",
            );
            Err(error)
        }
    }
}
