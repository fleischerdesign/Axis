use crate::presentation::accounts::{AccountsPresenter, AccountsView};
use axis_domain::models::cloud::{AccountStatus, CloudStatus};
use axis_presentation::View;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::rc::Rc;

pub struct AccountsPage {
    root: adw::ToolbarView,
    accounts_list: gtk4::ListBox,
    presenter: Rc<AccountsPresenter>,
}

impl AccountsPage {
    pub fn new(presenter: Rc<AccountsPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("Accounts")
            .icon_name("avatar-default-symbolic")
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&preferences_page)
            .build();

        toolbar_view.set_content(Some(&clamp));

        // 1. Connected Accounts Group
        let list_group = adw::PreferencesGroup::builder()
            .title("Connected Accounts")
            .description("Manage your cloud services and sync accounts")
            .build();
        preferences_page.add(&list_group);

        let accounts_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        list_group.add(&accounts_list);

        // 2. Add Account Group
        let add_group = adw::PreferencesGroup::builder()
            .title("Add Account")
            .description("Link external providers to sync data")
            .build();

        let google_row = adw::ActionRow::builder()
            .title("Google")
            .subtitle("Calendar, Tasks and user profile")
            .activatable(true)
            .build();

        let google_icon = gtk4::Image::from_icon_name("avatar-default-symbolic");
        google_row.add_prefix(&google_icon);
        google_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));

        let p_c = presenter.clone();
        google_row.connect_activated(move |_| {
            p_c.add_google_account();
        });

        add_group.add(&google_row);
        preferences_page.add(&add_group);

        Rc::new(Self {
            root: toolbar_view,
            accounts_list,
            presenter,
        })
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<CloudStatus> for AccountsPage {
    fn render(&self, status: &CloudStatus) {
        while let Some(child) = self.accounts_list.first_child() {
            self.accounts_list.remove(&child);
        }

        if status.accounts.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title("No accounts connected")
                .subtitle("Add an account below to sync calendars and tasks")
                .sensitive(false)
                .build();
            empty_row.add_prefix(&gtk4::Image::from_icon_name("avatar-default-symbolic"));
            self.accounts_list.append(&empty_row);
            return;
        }

        for account in &status.accounts {
            let provider_icon = "avatar-default-symbolic";

            let status_subtitle = match &account.status {
                AccountStatus::Online => format!("{} · Connected", account.provider_name),
                AccountStatus::Offline => format!("{} · Offline", account.provider_name),
                AccountStatus::NeedsAuthentication(_) => {
                    format!("{} · Re-authentication needed", account.provider_name)
                }
                AccountStatus::Error(err) => format!("{} · Error: {}", account.provider_name, err),
            };

            let row = adw::ActionRow::builder()
                .title(&account.display_name)
                .subtitle(status_subtitle)
                .build();

            row.add_prefix(&gtk4::Image::from_icon_name(provider_icon));

            let status_icon = match account.status {
                AccountStatus::Online => "object-select-symbolic",
                AccountStatus::NeedsAuthentication(_) => "dialog-warning-symbolic",
                _ => "dialog-error-symbolic",
            };
            row.add_suffix(&gtk4::Image::from_icon_name(status_icon));

            if matches!(account.status, AccountStatus::NeedsAuthentication(_)) {
                let reauth_btn = gtk4::Button::builder()
                    .label("Re-authenticate")
                    .valign(gtk4::Align::Center)
                    .css_classes(vec!["suggested-action".to_string()])
                    .build();

                let p = self.presenter.clone();
                reauth_btn.connect_clicked(move |_| {
                    p.add_google_account();
                });
                row.add_suffix(&reauth_btn);
            }

            self.accounts_list.append(&row);
        }
    }
}

impl AccountsView for AccountsPage {
    fn on_auth_error(&self, error: String) {
        log::error!("[accounts] Auth error: {}", error);
    }
}
