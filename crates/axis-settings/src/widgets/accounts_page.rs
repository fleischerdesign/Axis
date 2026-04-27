use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use axis_domain::models::cloud::{CloudStatus, AccountStatus};
use crate::presentation::accounts::{AccountsView, AccountsPresenter};
use axis_presentation::View;

pub struct AccountsPage {
    root: adw::ToolbarView,
    accounts_list: gtk4::ListBox,
    _presenter: Rc<AccountsPresenter>,
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
        toolbar_view.set_content(Some(&preferences_page));

        let list_group = adw::PreferencesGroup::builder()
            .title("Connected Accounts")
            .description("Manage your cloud services and accounts")
            .build();
        preferences_page.add(&list_group);

        let accounts_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        list_group.add(&accounts_list);

        let add_group = adw::PreferencesGroup::builder()
            .title("Add Account")
            .build();
        
        let google_row = adw::ActionRow::builder()
            .title("Google")
            .subtitle("Calendar, Tasks and more")
            .activatable(true)
            .build();
        
        let google_icon = gtk4::Image::from_icon_name("google-symbolic");
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
            _presenter: presenter,
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
                .sensitive(false)
                .build();
            self.accounts_list.append(&empty_row);
            return;
        }

        for account in &status.accounts {
            let row = adw::ActionRow::builder()
                .title(&account.display_name)
                .subtitle(&account.provider_name)
                .build();
            
            let status_icon = match account.status {
                AccountStatus::Online => "emblem-ok-symbolic",
                AccountStatus::NeedsAuthentication(_) => "dialog-warning-symbolic",
                _ => "dialog-error-symbolic",
            };
            
            row.add_suffix(&gtk4::Image::from_icon_name(status_icon));
            self.accounts_list.append(&row);
        }
    }
}

impl AccountsView for AccountsPage {
    fn on_auth_error(&self, error: String) {
        log::error!("Auth error: {}", error);
    }
}
