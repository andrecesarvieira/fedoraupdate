use adw::gtk;
use adw::prelude::*;
use fedoraupdate::{
    APP_ID, Schedule, UpdateCache, config_path, current_unix_time, load_cache, load_schedule,
    needs_offline_update, run_check, save_cache, save_schedule,
};
use gtk::glib;
use std::cell::Cell;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

#[derive(Clone)]
struct Ui {
    window: adw::ApplicationWindow,
    list: gtk::ListBox,
    count: gtk::Label,
    checked: gtk::Label,
    total: gtk::Label,
    install_progress: gtk::ProgressBar,
    status: gtk::Label,
    refresh: gtk::Button,
    install_online: gtk::Button,
    install_offline: gtk::Button,
    busy: Rc<Cell<bool>>,
}

fn build_ui(app: &adw::Application) {
    if !config_path().exists() {
        let _ = save_schedule(Schedule::default());
    }

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("FedoraUpdate"))));

    let refresh = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Verificar atualizações")
        .build();
    header.pack_end(&refresh);

    let count = summary_value("Carregando…");
    let checked = summary_detail("Última verificação: —");
    let status_text = gtk::Box::new(gtk::Orientation::Vertical, 7);
    status_text.set_hexpand(true);
    status_text.append(&count);
    status_text.append(&checked);

    let summary_icon = gtk::Image::from_icon_name("software-update-available-symbolic");
    summary_icon.set_pixel_size(32);
    summary_icon.set_valign(gtk::Align::Center);
    summary_icon.add_css_class("accent");

    let total = summary_detail("Tamanho: não informado");
    total.set_halign(gtk::Align::End);

    let summary_content = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    summary_content.set_hexpand(true);
    summary_content.set_margin_top(22);
    summary_content.set_margin_bottom(22);
    summary_content.set_margin_start(24);
    summary_content.set_margin_end(24);
    summary_content.append(&summary_icon);
    summary_content.append(&status_text);
    summary_content.append(&total);

    let summary = gtk::Box::new(gtk::Orientation::Vertical, 0);
    summary.set_hexpand(true);
    summary.append(&summary_content);

    let install_progress = gtk::ProgressBar::builder()
        .visible(false)
        .pulse_step(0.08)
        .build();
    install_progress.set_margin_bottom(16);
    install_progress.set_margin_start(24);
    install_progress.set_margin_end(24);
    summary.append(&install_progress);
    summary.add_css_class("card");

    let status = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    status.add_css_class("dim-label");

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .min_content_height(64)
        .propagate_natural_height(false)
        .child(&list)
        .build();

    let updates_group = adw::PreferencesGroup::builder()
        .title("Atualizações disponíveis")
        .vexpand(true)
        .build();
    updates_group.add(&scrolled);

    let install_online = gtk::Button::builder()
        .label("Instalar todas")
        .hexpand(true)
        .height_request(44)
        .build();
    install_online.add_css_class("suggested-action");
    install_online.add_css_class("pill");

    let install_offline = gtk::Button::builder()
        .label("Instalar ao reiniciar")
        .hexpand(true)
        .height_request(44)
        .build();
    install_offline.add_css_class("pill");

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions.append(&install_offline);
    actions.append(&install_online);

    let schedule = load_schedule();
    let schedule_label = gtk::Label::builder()
        .label(format!(
            "Verificação às {:02}:{:02}",
            schedule.hour, schedule.minute
        ))
        .css_classes(["dim-label"])
        .build();
    let edit_schedule = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .tooltip_text("Editar agendamento")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();

    let schedule_row = adw::ActionRow::builder()
        .title("Agendamento diário")
        .subtitle("Se o computador estiver desligado, a verificação ocorrerá no próximo login.")
        .activatable(true)
        .build();
    schedule_row.add_prefix(&gtk::Image::from_icon_name("alarm-symbolic"));
    schedule_row.add_suffix(&schedule_label);
    schedule_row.add_suffix(&edit_schedule);

    let schedule_group = adw::PreferencesGroup::new();
    schedule_group.add(&schedule_row);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
    content.set_vexpand(true);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.append(&summary);
    content.append(&status);
    content.append(&updates_group);

    let clamp = adw::Clamp::builder()
        .maximum_size(900)
        .tightening_threshold(720)
        .vexpand(true)
        .child(&content)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    let indicator_banner = adw::Banner::builder()
        .title("Ative o ícone do FedoraUpdate na barra superior.")
        .button_label("Ativar")
        .revealed(!appindicator_enabled() || !tray_running())
        .build();
    indicator_banner.connect_button_clicked(|banner| {
        let extension_ready = appindicator_enabled()
            || Command::new("/usr/bin/gnome-extensions")
                .args(["enable", "appindicatorsupport@rgcjonas.gmail.com"])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
        let tray_ready = start_tray();
        if extension_ready && tray_ready {
            banner.set_revealed(false);
        } else {
            banner.set_title(
                "Não foi possível ativar o ícone. Verifique se a extensão AppIndicator está instalada.",
            );
        }
    });
    toolbar.add_top_bar(&indicator_banner);
    let footer_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    footer_content.set_margin_top(12);
    footer_content.set_margin_bottom(12);
    footer_content.set_margin_start(24);
    footer_content.set_margin_end(24);
    footer_content.append(&actions);
    footer_content.append(&schedule_group);
    let footer = adw::Clamp::builder()
        .maximum_size(900)
        .tightening_threshold(720)
        .child(&footer_content)
        .build();
    toolbar.add_bottom_bar(&footer);
    toolbar.set_content(Some(&clamp));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("FedoraUpdate")
        .default_width(820)
        .default_height(800)
        .content(&toolbar)
        .build();

    let ui = Ui {
        window: window.clone(),
        list,
        count,
        checked,
        total,
        install_progress,
        status,
        refresh: refresh.clone(),
        install_online: install_online.clone(),
        install_offline: install_offline.clone(),
        busy: Rc::new(Cell::new(false)),
    };

    let schedule_window = window.clone();
    let schedule_value = schedule_label.clone();
    let open_schedule = move || show_schedule_dialog(&schedule_window, &schedule_value);
    edit_schedule.connect_clicked({
        let open_schedule = open_schedule.clone();
        move |_| open_schedule()
    });
    schedule_row.connect_activated(move |_| open_schedule());

    let online_ui = ui.clone();
    install_online.connect_clicked(move |_| install_updates(&online_ui, "online"));
    let offline_ui = ui.clone();
    install_offline.connect_clicked(move |_| install_updates(&offline_ui, "offline"));
    let refresh_ui = ui.clone();
    refresh.connect_clicked(move |_| check_async(&refresh_ui));

    let initial_cache_timestamp = if let Ok(cache) = load_cache() {
        let timestamp = cache.checked_at_unix;
        render_updates(&ui, &cache);
        timestamp
    } else {
        check_async(&ui);
        0
    };
    monitor_background_checks(&ui, initial_cache_timestamp);
    window.present();
}

struct BackgroundSnapshot {
    active: bool,
    failed: bool,
    cache: Option<UpdateCache>,
}

fn monitor_background_checks(ui: &Ui, initial_cache_timestamp: u64) {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let state = Command::new("/usr/bin/systemctl")
                .args(["--user", "is-active", "fedoraupdate-check.service"])
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .unwrap_or_default();
            let snapshot = BackgroundSnapshot {
                active: matches!(state.as_str(), "active" | "activating"),
                failed: state == "failed",
                cache: load_cache().ok(),
            };
            if sender.send(snapshot).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(700));
        }
    });

    let ui = ui.clone();
    let last_cache_timestamp = Rc::new(Cell::new(initial_cache_timestamp));
    let was_active = Rc::new(Cell::new(false));
    glib::timeout_add_local(Duration::from_millis(200), move || {
        let mut latest = None;
        while let Ok(snapshot) = receiver.try_recv() {
            latest = Some(snapshot);
        }
        let Some(snapshot) = latest else {
            return glib::ControlFlow::Continue;
        };
        if ui.busy.get() {
            return glib::ControlFlow::Continue;
        }

        if snapshot.active {
            if !was_active.replace(true) {
                ui.count.set_label("Verificando atualizações…");
                ui.checked
                    .set_label("Solicitação recebida pelo ícone da bandeja");
                ui.total.set_label("Consultando repositórios");
                set_status(&ui, None);
                ui.refresh.set_sensitive(false);
                ui.install_online.set_sensitive(false);
                ui.install_offline.set_sensitive(false);
            }
            return glib::ControlFlow::Continue;
        }

        let check_just_finished = was_active.replace(false);
        if let Some(cache) = snapshot.cache {
            if check_just_finished || cache.checked_at_unix != last_cache_timestamp.get() {
                last_cache_timestamp.set(cache.checked_at_unix);
                render_updates(&ui, &cache);
                if snapshot.failed {
                    set_status(
                        &ui,
                        Some("A verificação iniciada pela bandeja não pôde ser concluída."),
                    );
                }
            }
        } else if check_just_finished && snapshot.failed {
            ui.count.set_label("Não foi possível verificar");
            ui.checked.set_label("Tente novamente em alguns instantes");
            set_status(
                &ui,
                Some("A verificação iniciada pela bandeja não pôde ser concluída."),
            );
        }
        ui.refresh.set_sensitive(true);
        glib::ControlFlow::Continue
    });
}

fn summary_value(text: &str) -> gtk::Label {
    let label = gtk::Label::builder().label(text).xalign(0.0).build();
    label.add_css_class("title-3");
    label
}

fn summary_detail(text: &str) -> gtk::Label {
    let label = gtk::Label::builder().label(text).xalign(0.0).build();
    label.add_css_class("dim-label");
    label
}

fn appindicator_enabled() -> bool {
    Command::new("/usr/bin/gnome-extensions")
        .args(["list", "--enabled"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == "appindicatorsupport@rgcjonas.gmail.com")
        })
        .unwrap_or(false)
}

fn tray_running() -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .chars()
            .all(|character| character.is_ascii_digit())
            && std::fs::read_link(entry.path().join("exe"))
                .ok()
                .and_then(|path| path.file_name().map(|name| name.to_owned()))
                .map(|name| name == "fedoraupdate-tray")
                .unwrap_or(false)
    })
}

fn start_tray() -> bool {
    tray_running() || Command::new("/usr/bin/fedoraupdate-tray").spawn().is_ok()
}

fn clear_list(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn render_updates(ui: &Ui, cache: &UpdateCache) {
    clear_list(&ui.list);
    for update in &cache.updates {
        let row = adw::ActionRow::builder()
            .title(&update.name)
            .subtitle(format!("{}  →  {}", update.arch, update.evr))
            .build();
        let repository = gtk::Label::builder()
            .label(package_kind(&update.name, &update.repository))
            .valign(gtk::Align::Center)
            .css_classes(["caption", "dim-label"])
            .build();
        row.add_suffix(&repository);
        if let Some(size) = update.download_size {
            let size = gtk::Label::builder()
                .label(format_size(size))
                .valign(gtk::Align::Center)
                .width_chars(9)
                .xalign(1.0)
                .css_classes(["dim-label"])
                .build();
            row.add_suffix(&size);
        }
        ui.list.append(&row);
    }

    let count = cache.updates.len();
    let critical = needs_offline_update(&cache.updates);
    ui.count.set_label(&match count {
        0 => "Sistema atualizado".to_owned(),
        1 => "1 atualização disponível".to_owned(),
        n => format!("{n} atualizações disponíveis"),
    });
    ui.checked
        .set_label(&relative_checked_time(cache.checked_at_unix));

    let known_total: u64 = cache.updates.iter().filter_map(|u| u.download_size).sum();
    let known_count = cache
        .updates
        .iter()
        .filter(|u| u.download_size.is_some())
        .count();
    let total_text = if count > 0 && known_count == count {
        format!("Download total: {}", format_size(known_total))
    } else if count > 0 {
        "Todos os pacotes serão instalados".to_owned()
    } else {
        "Nenhum download necessário".to_owned()
    };
    ui.total.set_label(&total_text);

    if count == 0 {
        let empty = adw::ActionRow::builder()
            .title("Nenhuma atualização pendente")
            .subtitle("O Fedora está com os pacotes mais recentes.")
            .build();
        empty.add_prefix(&gtk::Image::from_icon_name("object-select-symbolic"));
        ui.list.append(&empty);
    }

    set_status(
        ui,
        if critical {
            Some(
                "Há componentes do sistema nesta atualização. Instalar ao reiniciar é recomendado.",
            )
        } else {
            None
        },
    );
    ui.install_online.set_sensitive(count > 0);
    ui.install_offline.set_sensitive(count > 0);
}

fn package_kind(name: &str, repository: &str) -> String {
    let kind = if name.starts_with("kernel") || matches!(name, "systemd" | "glibc" | "dbus") {
        "Sistema"
    } else if name.starts_with("gnome-") || name == "mutter" {
        "Ambiente GNOME"
    } else if name.contains("mesa") || name.contains("driver") {
        "Driver"
    } else if name.contains("openssl") || name.contains("crypto") {
        "Segurança"
    } else if name.ends_with("-libs") || name.starts_with("lib") {
        "Biblioteca"
    } else {
        repository
    };
    kind.to_owned()
}

fn format_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes as f64 >= MIB {
        format!("{:.1} MB", bytes as f64 / MIB).replace('.', ",")
    } else if bytes as f64 >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn relative_checked_time(timestamp: u64) -> String {
    if timestamp == 0 {
        return "Última verificação: agora".to_owned();
    }
    let elapsed = current_unix_time().saturating_sub(timestamp);
    match elapsed {
        0..=59 => "Última verificação: agora".to_owned(),
        60..=3599 => format!("Última verificação: há {} min", elapsed / 60),
        3600..=86399 => format!("Última verificação: há {} h", elapsed / 3600),
        _ => format!("Última verificação: há {} dias", elapsed / 86400),
    }
}

fn set_status(ui: &Ui, message: Option<&str>) {
    if let Some(message) = message {
        ui.status.set_label(message);
        ui.status.set_visible(true);
    } else {
        ui.status.set_visible(false);
    }
}

fn check_async(ui: &Ui) {
    ui.busy.set(true);
    ui.refresh.set_sensitive(false);
    ui.count.set_label("Verificando atualizações…");
    ui.checked
        .set_label("Consultando os repositórios habilitados");
    set_status(ui, None);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = run_check()
            .and_then(|updates| {
                save_cache(updates)?;
                load_cache()
            })
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(cache)) => {
                render_updates(&ui, &cache);
                ui.busy.set(false);
                ui.refresh.set_sensitive(true);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                ui.count.set_label("Não foi possível verificar");
                ui.checked.set_label("Tente novamente em alguns instantes");
                set_status(&ui, Some(&format!("Falha na verificação: {error}")));
                ui.busy.set(false);
                ui.refresh.set_sensitive(true);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                set_status(&ui, Some("A verificação foi interrompida."));
                ui.busy.set(false);
                ui.refresh.set_sensitive(true);
                glib::ControlFlow::Break
            }
        }
    });
}

fn install_updates(ui: &Ui, mode: &'static str) {
    ui.busy.set(true);
    ui.install_online.set_sensitive(false);
    ui.install_offline.set_sensitive(false);
    ui.count.set_label("Preparando instalação…");
    ui.checked
        .set_label("Aguardando autorização do administrador");
    ui.total.set_label("Confirme a solicitação do sistema");
    ui.install_progress.set_visible(true);
    ui.install_progress.set_fraction(0.0);
    set_status(ui, None);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut child = Command::new("/usr/bin/pkexec")
                .args(["/usr/libexec/fedoraupdate-helper", mode])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|error| error.to_string())?;

            if let Some(stdout) = child.stdout.take() {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if line == "FEDORAUPDATE_STATUS:AUTHORIZED" {
                        let _ = sender.send(InstallEvent::Authorized);
                    } else if let Some(stage) = install_stage(&line) {
                        let _ = sender.send(InstallEvent::Stage(stage));
                    }
                }
            }

            child
                .wait()
                .map(|status| (status.code(), status.success()))
                .map_err(|error| error.to_string())
        })();
        let _ = sender.send(InstallEvent::Finished(result));
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(150), move || {
        ui.install_progress.pulse();
        let mut finished = None;
        let mut disconnected = false;
        loop {
            let event = match receiver.try_recv() {
                Ok(event) => event,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            };
            match event {
                InstallEvent::Authorized => {
                    if mode == "offline" {
                        ui.count.set_label("Preparando atualização…");
                    } else {
                        ui.count.set_label("Instalando atualizações…");
                    }
                    ui.checked
                        .set_label("Autorização concluída. Iniciando o DNF…");
                    ui.total.set_label("Operação em andamento");
                }
                InstallEvent::Stage(stage) => ui.checked.set_label(&stage),
                InstallEvent::Finished(result) => finished = Some(result),
            }
        }

        match finished {
            Some(Ok((_, true))) => {
                ui.install_progress.set_visible(false);
                let message = if mode == "offline" {
                    "As atualizações foram preparadas. Reinicie o computador para concluir."
                } else {
                    "As atualizações foram instaladas com sucesso."
                };
                show_dialog(&ui.window, "Atualização concluída", message);
                ui.busy.set(false);
                check_async(&ui);
                glib::ControlFlow::Break
            }
            Some(Ok((Some(126), false))) => {
                ui.install_progress.set_visible(false);
                ui.count.set_label("Instalação cancelada");
                ui.checked.set_label("Nenhuma alteração foi feita");
                ui.total.set_label("");
                set_status(&ui, Some("Instalação cancelada pelo usuário."));
                ui.busy.set(false);
                ui.install_online.set_sensitive(true);
                ui.install_offline.set_sensitive(true);
                glib::ControlFlow::Break
            }
            Some(Ok((_, false)) | Err(_)) => {
                ui.install_progress.set_visible(false);
                ui.count.set_label("Falha na instalação");
                ui.checked
                    .set_label("O DNF não conseguiu concluir a operação");
                ui.total.set_label("");
                set_status(&ui, Some("Não foi possível instalar as atualizações."));
                ui.busy.set(false);
                ui.install_online.set_sensitive(true);
                ui.install_offline.set_sensitive(true);
                glib::ControlFlow::Break
            }
            None if disconnected => {
                ui.install_progress.set_visible(false);
                set_status(&ui, Some("A instalação foi interrompida."));
                ui.busy.set(false);
                glib::ControlFlow::Break
            }
            None => glib::ControlFlow::Continue,
        }
    });
}

enum InstallEvent {
    Authorized,
    Stage(String),
    Finished(Result<(Option<i32>, bool), String>),
}

fn install_stage(line: &str) -> Option<String> {
    let line = line.to_ascii_lowercase();
    let message = if line.contains("updating and loading repositories")
        || line.contains("refreshing metadata")
    {
        "Atualizando os repositórios…"
    } else if line.contains("repositories loaded") || line.contains("metadata cache created") {
        "Repositórios atualizados. Resolvendo dependências…"
    } else if line.contains("downloading packages") || line.contains("downloading files") {
        "Baixando os pacotes…"
    } else if line.contains("running transaction")
        || line.contains("upgrading:")
        || line.contains("installing:")
    {
        "Aplicando as atualizações…"
    } else if line.contains("complete!") || line.contains("transaction finished") {
        "Finalizando a instalação…"
    } else {
        return None;
    };
    Some(message.to_owned())
}

#[cfg(test)]
mod install_tests {
    use super::install_stage;

    #[test]
    fn translates_dnf_progress_into_user_facing_stages() {
        assert_eq!(
            install_stage("Updating and loading repositories:"),
            Some("Atualizando os repositórios…".to_owned())
        );
        assert_eq!(
            install_stage("Running transaction"),
            Some("Aplicando as atualizações…".to_owned())
        );
        assert_eq!(install_stage("unrelated output"), None);
    }
}

fn show_schedule_dialog(parent: &adw::ApplicationWindow, schedule_label: &gtk::Label) {
    let schedule = load_schedule();
    let hour = gtk::SpinButton::with_range(0.0, 23.0, 1.0);
    hour.set_value(schedule.hour.into());
    hour.set_width_chars(2);
    let minute = gtk::SpinButton::with_range(0.0, 59.0, 1.0);
    minute.set_value(schedule.minute.into());
    minute.set_width_chars(2);

    let time = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    time.set_halign(gtk::Align::Center);
    time.set_margin_top(12);
    time.set_margin_bottom(6);
    time.append(&hour);
    time.append(&gtk::Label::new(Some(":")));
    time.append(&minute);

    let dialog = adw::AlertDialog::builder()
        .heading("Horário da verificação")
        .body("O FedoraUpdate verificará atualizações uma vez por dia.")
        .extra_child(&time)
        .build();
    dialog.add_response("cancel", "Cancelar");
    dialog.add_response("save", "Salvar");
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");

    let response_parent = parent.clone();
    let schedule_label = schedule_label.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "save" {
            return;
        }
        let schedule = Schedule {
            hour: hour.value_as_int() as u8,
            minute: minute.value_as_int() as u8,
        };
        match save_schedule(schedule) {
            Ok(()) => {
                schedule_label.set_label(&format!(
                    "Verificação às {:02}:{:02}",
                    schedule.hour, schedule.minute
                ));
                show_dialog(
                    &response_parent,
                    "Agendamento salvo",
                    &format!(
                        "A próxima verificação diária ocorrerá às {:02}:{:02}.",
                        schedule.hour, schedule.minute
                    ),
                );
            }
            Err(error) => show_dialog(
                &response_parent,
                "Não foi possível salvar",
                &error.to_string(),
            ),
        }
    });
    dialog.present(Some(parent));
}

fn show_dialog(parent: &adw::ApplicationWindow, heading: &str, body: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    dialog.add_response("close", "Fechar");
    dialog.set_default_response(Some("close"));
    dialog.present(Some(parent));
}
