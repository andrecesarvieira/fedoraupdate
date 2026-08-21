Name:           fedoraupdate
Version:        0.2.7
Release:        1%{?dist}
Summary:        Verificador nativo de atualizações para Fedora 44

License:        GPL-3.0-or-later
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  pkgconfig(gtk+-3.0)
BuildRequires:  pkgconfig(appindicator3-0.1)
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib
Requires:       dnf5
Requires:       polkit
Requires:       libnotify
Requires:       libappindicator-gtk3
Requires:       gnome-shell-extension-appindicator

%description
FedoraUpdate verifica atualizações diariamente com DNF5, mostra os pacotes
disponíveis e instala todos apenas após confirmação e autorização gráfica via
Polkit. Inclui um AppIndicator e opções de atualização on-line ou off-line.

%prep
%autosetup

%build
# GTK 3 e GTK 4 não podem ser carregados no mesmo processo. Cada binário é
# compilado com somente o toolkit que utiliza.
/usr/bin/cargo build --release --locked --no-default-features \
  --features gui --bin fedoraupdate
/usr/bin/cargo build --release --locked --no-default-features \
  --features tray --bin fedoraupdate-tray
/usr/bin/cargo build --release --locked --no-default-features \
  --bin fedoraupdate-check --bin fedoraupdate-helper

%install
install -Dpm0755 target/release/fedoraupdate %{buildroot}%{_bindir}/fedoraupdate
install -Dpm0755 target/release/fedoraupdate-check %{buildroot}%{_bindir}/fedoraupdate-check
install -Dpm0755 target/release/fedoraupdate-tray %{buildroot}%{_bindir}/fedoraupdate-tray
install -Dpm0755 target/release/fedoraupdate-helper %{buildroot}%{_libexecdir}/fedoraupdate-helper

install -Dpm0644 data/org.fedoraupdate.FedoraUpdate.desktop \
  %{buildroot}%{_datadir}/applications/org.fedoraupdate.FedoraUpdate.desktop
install -Dpm0644 data/fedoraupdate-tray.desktop \
  %{buildroot}%{_sysconfdir}/xdg/autostart/fedoraupdate-tray.desktop
install -Dpm0644 data/org.fedoraupdate.FedoraUpdate.metainfo.xml \
  %{buildroot}%{_metainfodir}/org.fedoraupdate.FedoraUpdate.metainfo.xml
install -Dpm0644 data/org.fedoraupdate.FedoraUpdate.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.fedoraupdate.FedoraUpdate.svg
install -Dpm0644 data/org.fedoraupdate.FedoraUpdate-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/org.fedoraupdate.FedoraUpdate-symbolic.svg
install -Dpm0644 data/org.fedoraupdate.FedoraUpdate-updates-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/org.fedoraupdate.FedoraUpdate-updates-symbolic.svg
install -Dpm0644 data/org.fedoraupdate.FedoraUpdate-updates.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.fedoraupdate.FedoraUpdate-updates.svg
install -Dpm0644 data/fedoraupdate-check.service \
  %{buildroot}%{_userunitdir}/fedoraupdate-check.service
install -Dpm0644 data/fedoraupdate-check.timer \
  %{buildroot}%{_userunitdir}/fedoraupdate-check.timer
install -Dpm0644 data/org.fedoraupdate.policy \
  %{buildroot}%{_datadir}/polkit-1/actions/org.fedoraupdate.policy
install -Dpm0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%check
/usr/bin/cargo test --release --no-default-features --locked
desktop-file-validate data/org.fedoraupdate.FedoraUpdate.desktop
desktop-file-validate data/fedoraupdate-tray.desktop
appstream-util validate-relax --nonet data/org.fedoraupdate.FedoraUpdate.metainfo.xml

%files
%dir %attr(0755,root,root) %{_licensedir}/%{name}
%license %attr(0644,root,root) %{_licensedir}/%{name}/LICENSE
%dir %attr(0755,root,root) %{_docdir}/%{name}
%doc %attr(0644,root,root) %{_docdir}/%{name}/README.md
%attr(0755,root,root) %{_bindir}/fedoraupdate
%attr(0755,root,root) %{_bindir}/fedoraupdate-check
%attr(0755,root,root) %{_bindir}/fedoraupdate-tray
%attr(0755,root,root) %{_libexecdir}/fedoraupdate-helper
%attr(0644,root,root) %{_datadir}/applications/org.fedoraupdate.FedoraUpdate.desktop
%attr(0644,root,root) %{_sysconfdir}/xdg/autostart/fedoraupdate-tray.desktop
%attr(0644,root,root) %{_metainfodir}/org.fedoraupdate.FedoraUpdate.metainfo.xml
%attr(0644,root,root) %{_datadir}/icons/hicolor/scalable/apps/org.fedoraupdate.FedoraUpdate.svg
%attr(0644,root,root) %{_datadir}/icons/hicolor/symbolic/apps/org.fedoraupdate.FedoraUpdate-symbolic.svg
%attr(0644,root,root) %{_datadir}/icons/hicolor/symbolic/apps/org.fedoraupdate.FedoraUpdate-updates-symbolic.svg
%attr(0644,root,root) %{_datadir}/icons/hicolor/scalable/apps/org.fedoraupdate.FedoraUpdate-updates.svg
%attr(0644,root,root) %{_userunitdir}/fedoraupdate-check.service
%attr(0644,root,root) %{_userunitdir}/fedoraupdate-check.timer
%attr(0644,root,root) %{_datadir}/polkit-1/actions/org.fedoraupdate.policy

%changelog
* Thu Aug 20 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.7-1
- Mostra atividade contínua enquanto o DNF executa a atualização
- Atualiza a interface assim que o Polkit conclui a autorização
- Exibe as principais etapas da instalação na janela

* Thu Aug 20 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.6-1
- Expande verticalmente a lista até as ações fixas do rodapé
- Mantém a rolagem restrita aos pacotes disponíveis

* Thu Aug 20 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.5-1
- Usa um ícone híbrido no estado de atenção
- Preserva o ícone simbólico normal e destaca pendências com um ponto laranja

* Thu Aug 20 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.4-1
- Usa o estado nativo de atenção do AppIndicator quando há atualizações
- Mantém o ícone simbólico e delega o destaque visual ao GNOME

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.3-1
- Sincroniza a janela com verificações iniciadas pelo AppIndicator
- Exibe o estado de verificação imediatamente no menu da bandeja

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.2-1
- Corrige o ícone do estado sem atualizações
- Fixa as ações de instalação no rodapé
- Ajusta a altura da lista à quantidade de pacotes
- Ativa a extensão e inicia o AppIndicator pela janela

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.1-1
- Fixa o agendamento no rodapé da janela

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.2.0-1
- Redesenha a janela com resumo, lista compacta e ações mais claras
- Exibe tamanho de download quando informado pelo DNF5
- Move a edição do agendamento para um diálogo nativo

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.1.1-2
- Corrige permissões dos diretórios de documentação e licença

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.1.1-1
- Isola GTK 4 e GTK 3 em binários compilados com features separadas

* Wed Aug 19 2026 FedoraUpdate contributors <noreply@example.invalid> - 0.1.0-1
- Primeira versão local para Fedora 44
