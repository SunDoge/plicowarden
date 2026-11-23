use crate::{
    model::{Item, Warden},
    ui::*,
};
use plico::prelude::*;

impl Warden {
    #[component]
    #[rustfmt::skip]
    pub fn bulk_toolbar(&mut self, ui: &mut UiContext, visible: &[Item]) {
        let disabled = self.busy || !self.authenticated || self.bulk_selected.is_empty();
        let all_favorite = !self.bulk_selected.is_empty()
            && visible.iter().filter(|item| self.bulk_selected.contains(&item.id)).all(|item| item.favorite);
        surface(ui).padding_x(14).padding_y(8) + |ui| {
            row(ui).width_percent(100.).gap(8).align(Align::Center).wrap(true) + |ui| {
                text!(ui, "{} selected", self.bulk_selected.len()).font_size(13).weight(650).grow(1.);
                if button(ui).text("All").aria_label("Select all visible").ghost()
                    .disabled(self.busy || visible.len() > 500).clicked()
                {
                    self.bulk_selected.extend(visible.iter().map(|item| item.id.clone()));
                }
                if tool(ui, "minus", "Clear selection", self.busy) {
                    self.bulk_selected.clear();
                }
                let actions: &[(&str, &str, &str)] = if self.filter == "Trash" {
                    &[
                        ("restore", "Restore selected", "restore"),
                        ("purge", "Delete selected permanently", "trash"),
                    ]
                } else {
                    &[
                        (
                            if all_favorite { "unfavorite" } else { "favorite" },
                            if all_favorite { "Unfavorite selected" } else { "Favorite selected" },
                            "star",
                        ),
                        (
                            if self.filter == "Archive" { "unarchive" } else { "archive" },
                            if self.filter == "Archive" { "Unarchive selected" } else { "Archive selected" },
                            "archive",
                        ),
                        ("trash", "Trash selected", "trash"),
                    ]
                };
                for &(action, label, name) in actions {
                    ui.keyed(action, |ui| {
                        if tool(ui, name, label, disabled) {
                            if action == "trash" || action == "purge" {
                                self.bulk_confirm = Some(action.into());
                                self.bulk_modal.open();
                            } else {
                                self.apply_bulk_action(action);
                            }
                        }
                    });
                }
                if self.filter != "Trash"
                    && tool(ui, "folder", "Move selected to folder", disabled)
                {
                    self.open_move_dialog(self.bulk_selected.iter().cloned().collect());
                }
                if tool(ui, "close", "Done selecting", self.busy) {
                    self.bulk_mode = false;
                    self.bulk_selected.clear();
                }
            };
            if !self.authenticated {
                text(ui, "Connect your account to update selected items.").font_size(12).color(MUTED)
                    .width_percent(100.);
            }
        };
    }
    #[component]
    #[rustfmt::skip]
    pub fn bulk_confirmation(&mut self, ui: &mut UiContext) {
        let mut state = std::mem::take(&mut self.bulk_modal);
        let mut confirm = false;
        let action = self.bulk_confirm.clone().unwrap_or_default();
        modal(ui, &mut state).title("Confirm bulk action").width(440.)
            .children(|ui| {
            text!(
                ui,
                "{} {} items?",
                if action == "purge" {
                    "Permanently delete"
                } else {
                    "Move to trash"
                },
                self.bulk_selected.len()
            ).font_size(22).weight(650).width_percent(100.);
            text(
                ui,
                if action == "purge" {
                    "This cannot be undone."
                } else {
                    "You can restore these items from Trash."
                },
            ).color(MUTED).width_percent(100.);
            row(ui).gap(10).justify(Justify::End).width_percent(100.) + |ui| {
                if button(ui).text("Cancel bulk action").clicked() {
                    ui.close_modal();
                }
                if button(ui).text("Confirm bulk action").primary()
                    .disabled(self.busy || self.bulk_selected.is_empty()).clicked()
                {
                    confirm = true;
                    ui.close_modal();
                }
            };
        });
        self.bulk_modal = state;
        if confirm {
            self.apply_bulk_action(&action);
        }
        if !self.bulk_modal.is_open() {
            self.bulk_confirm = None;
        }
    }
}
