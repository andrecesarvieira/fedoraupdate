use fedoraupdate::{APP_NAME, load_cache};
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

fn spawn(program: &str, args: &[&str]) {
    let _ = Command::new(program).args(args).spawn();
}

fn check_service_active() -> bool {
    Command::new("/usr/bin/systemctl")
        .args(["--user", "is-active", "fedoraupdate-check.service"])
        .output()
        .map(|output| {
            matches!(
                String::from_utf8_lossy(&output.stdout).trim(),
                "active" | "activating"
            )
        })
        .unwrap_or(false)
}

fn main() {
    gtk::init().expect("não foi possível iniciar GTK 3");

    let mut indicator = AppIndicator::new("fedoraupdate", "org.fedoraupdate.FedoraUpdate-symbolic");
    indicator.set_status(AppIndicatorStatus::Active);
    indicator.set_title(APP_NAME);
    indicator.set_icon_full("org.fedoraupdate.FedoraUpdate-symbolic", "FedoraUpdate");
    indicator.set_attention_icon_full(
        "org.fedoraupdate.FedoraUpdate-updates",
        "Atualizações disponíveis",
    );

    let mut menu = gtk::Menu::new();
    let status_item = gtk::MenuItem::with_label("Verificação pendente");
    status_item.set_sensitive(false);
    menu.append(&status_item);
    menu.append(&gtk::SeparatorMenuItem::new());

    let open = gtk::MenuItem::with_label("Abrir FedoraUpdate");
    open.connect_activate(|_| spawn("/usr/bin/fedoraupdate", &[]));
    menu.append(&open);

    let check = gtk::MenuItem::with_label("Verificar agora");
    let checking_status = status_item.clone();
    let checking_action = check.clone();
    check.connect_activate(move |_| {
        checking_status.set_label("Verificando atualizações…");
        checking_action.set_sensitive(false);
        spawn(
            "/usr/bin/systemctl",
            &["--user", "start", "fedoraupdate-check.service"],
        )
    });
    menu.append(&check);

    menu.append(&gtk::SeparatorMenuItem::new());
    let quit = gtk::MenuItem::with_label("Sair");
    quit.connect_activate(|_| gtk::main_quit());
    menu.append(&quit);
    menu.show_all();
    indicator.set_menu(&mut menu);

    let indicator = Rc::new(RefCell::new(indicator));
    let status_clone = status_item.clone();
    let indicator_clone = indicator.clone();
    let check_clone = check.clone();
    gtk::glib::timeout_add_local(Duration::from_secs(1), move || {
        if check_service_active() {
            status_clone.set_label("Verificando atualizações…");
            check_clone.set_sensitive(false);
            let mut indicator = indicator_clone.borrow_mut();
            indicator.set_status(AppIndicatorStatus::Active);
            indicator.set_icon_full(
                "org.fedoraupdate.FedoraUpdate-symbolic",
                "Verificando atualizações",
            );
            return gtk::glib::ControlFlow::Continue;
        }
        check_clone.set_sensitive(true);
        if let Ok(cache) = load_cache() {
            let count = cache.updates.len();
            let label = match count {
                0 => "Sistema atualizado".to_owned(),
                1 => "1 atualização disponível".to_owned(),
                _ => format!("{count} atualizações disponíveis"),
            };
            status_clone.set_label(&label);
            let mut indicator = indicator_clone.borrow_mut();
            if count == 0 {
                indicator.set_status(AppIndicatorStatus::Active);
                indicator.set_icon_full("org.fedoraupdate.FedoraUpdate-symbolic", &label);
            } else {
                indicator.set_attention_icon_full("org.fedoraupdate.FedoraUpdate-updates", &label);
                indicator.set_status(AppIndicatorStatus::Attention);
            }
        }
        gtk::glib::ControlFlow::Continue
    });

    gtk::main();
}
