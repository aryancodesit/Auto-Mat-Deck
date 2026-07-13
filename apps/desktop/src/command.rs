use crate::model::*;

/// All domain mutations expressible in the editor.
///
/// Device mutations are not migrated yet — they remain on `AppState` methods.
/// This enum covers only profile/page/button operations that the editor needs.
///
/// Identity is always supplied by the caller — the reducer never calls `new()`.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // Profile
    CreateProfile {
        profile_id: ProfileId,
        initial_page_id: PageId,
        name: String,
    },
    DeleteProfile {
        profile_id: ProfileId,
    },
    RenameProfile {
        profile_id: ProfileId,
        new_name: String,
    },
    // Page
    AddPage {
        profile_id: ProfileId,
        page_id: PageId,
        name: String,
    },
    DeletePage {
        profile_id: ProfileId,
        page_id: PageId,
    },
    RenamePage {
        profile_id: ProfileId,
        page_id: PageId,
        new_name: String,
    },
    // Button
    AddButton {
        profile_id: ProfileId,
        page_id: PageId,
        button: Button,
    },
    RemoveButton {
        profile_id: ProfileId,
        page_id: PageId,
        button_id: ButtonId,
    },
    UpdateButton {
        profile_id: ProfileId,
        page_id: PageId,
        button: Button,
    },
    MoveButton {
        profile_id: ProfileId,
        from_page: PageId,
        button_id: ButtonId,
        to_page: PageId,
    },
}

/// Reason a command could not be applied.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    ProfileNotFound,
    PageNotFound,
    ButtonNotFound,
    TargetPageNotFound,
    CannotDeleteLastProfile,
    CannotDeleteLastPage,
    DuplicateButtonId,
}

/// Reducer: returns a new `Document` or a `CommandError`.
///
/// Identity is always supplied through the `Command` — the reducer never
/// generates IDs. Identical inputs always produce identical outputs.
///
/// Invariants enforced:
/// - At least one Profile always exists.
/// - A Profile always has at least one Page.
/// - IDs are validated before mutation.
/// - Deletion commands validate entity existence before invariant checks.
/// - Failed commands leave the input unchanged.
/// - MoveButton does not silently duplicate a button.
pub fn apply(doc: &Document, cmd: &Command) -> Result<Document, CommandError> {
    let mut out = doc.clone();
    match cmd {
        Command::CreateProfile {
            profile_id,
            initial_page_id,
            name,
        } => {
            out.profiles.push(Profile {
                id: profile_id.clone(),
                name: name.clone(),
                pages: vec![Page {
                    id: initial_page_id.clone(),
                    name: "Page 1".into(),
                    buttons: Vec::new(),
                }],
            });
            Ok(out)
        }
        Command::DeleteProfile { profile_id } => {
            let pos = out.profiles.iter().position(|p| p.id == *profile_id);
            match pos {
                None => Err(CommandError::ProfileNotFound),
                Some(_) if out.profiles.len() <= 1 => Err(CommandError::CannotDeleteLastProfile),
                Some(_) => {
                    out.profiles.retain(|p| p.id != *profile_id);
                    Ok(out)
                }
            }
        }
        Command::RenameProfile {
            profile_id,
            new_name,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    p.name = new_name.clone();
                    Ok(out)
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::AddPage {
            profile_id,
            page_id,
            name,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    p.pages.push(Page {
                        id: page_id.clone(),
                        name: name.clone(),
                        buttons: Vec::new(),
                    });
                    Ok(out)
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::DeletePage {
            profile_id,
            page_id,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                None => Err(CommandError::ProfileNotFound),
                Some(p) => {
                    let exists = p.pages.iter().any(|pg| pg.id == *page_id);
                    if !exists {
                        return Err(CommandError::PageNotFound);
                    }
                    if p.pages.len() <= 1 {
                        return Err(CommandError::CannotDeleteLastPage);
                    }
                    p.pages.retain(|pg| pg.id != *page_id);
                    Ok(out)
                }
            }
        }
        Command::RenamePage {
            profile_id,
            page_id,
            new_name,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    let pg = p.pages.iter_mut().find(|pg| pg.id == *page_id);
                    match pg {
                        Some(pg) => {
                            pg.name = new_name.clone();
                            Ok(out)
                        }
                        None => Err(CommandError::PageNotFound),
                    }
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::AddButton {
            profile_id,
            page_id,
            button,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    let pg = p.pages.iter_mut().find(|pg| pg.id == *page_id);
                    match pg {
                        Some(pg) => {
                            if pg.buttons.iter().any(|b| b.id == button.id) {
                                return Err(CommandError::DuplicateButtonId);
                            }
                            pg.buttons.push(button.clone());
                            Ok(out)
                        }
                        None => Err(CommandError::PageNotFound),
                    }
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::RemoveButton {
            profile_id,
            page_id,
            button_id,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    let pg = p.pages.iter_mut().find(|pg| pg.id == *page_id);
                    match pg {
                        Some(pg) => {
                            let before = pg.buttons.len();
                            pg.buttons.retain(|b| b.id != *button_id);
                            if pg.buttons.len() == before {
                                return Err(CommandError::ButtonNotFound);
                            }
                            Ok(out)
                        }
                        None => Err(CommandError::PageNotFound),
                    }
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::UpdateButton {
            profile_id,
            page_id,
            button,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    let pg = p.pages.iter_mut().find(|pg| pg.id == *page_id);
                    match pg {
                        Some(pg) => {
                            let existing = pg.buttons.iter_mut().find(|b| b.id == button.id);
                            match existing {
                                Some(b) => {
                                    // Preserve ButtonId, replace everything else
                                    b.label = button.label.clone();
                                    b.action = button.action.clone();
                                    Ok(out)
                                }
                                None => Err(CommandError::ButtonNotFound),
                            }
                        }
                        None => Err(CommandError::PageNotFound),
                    }
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
        Command::MoveButton {
            profile_id,
            from_page,
            button_id,
            to_page,
        } => {
            let p = out.profiles.iter_mut().find(|p| p.id == *profile_id);
            match p {
                Some(p) => {
                    // Find both pages
                    let from_idx = p.pages.iter().position(|pg| pg.id == *from_page);
                    let to_idx = p.pages.iter().position(|pg| pg.id == *to_page);
                    match (from_idx, to_idx) {
                        (Some(fi), Some(ti)) => {
                            let button = p.pages[fi]
                                .buttons
                                .iter()
                                .find(|b| b.id == *button_id)
                                .cloned();
                            match button {
                                Some(b) => {
                                    if p.pages[ti].buttons.iter().any(|x| x.id == b.id) {
                                        return Err(CommandError::DuplicateButtonId);
                                    }
                                    p.pages[fi].buttons.retain(|x| x.id != b.id);
                                    p.pages[ti].buttons.push(b);
                                    Ok(out)
                                }
                                None => Err(CommandError::ButtonNotFound),
                            }
                        }
                        (None, _) => Err(CommandError::PageNotFound),
                        (_, None) => Err(CommandError::TargetPageNotFound),
                    }
                }
                None => Err(CommandError::ProfileNotFound),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Document {
        let mut doc = Document::empty();
        // override the default single-profile doc with known IDs
        let pid = ProfileId::from_string("profile-a");
        let pgid = PageId::from_string("page-a");
        let bid = ButtonId::from_string("btn-a");
        doc.profiles = vec![Profile {
            id: pid,
            name: "Test Profile".into(),
            pages: vec![Page {
                id: pgid,
                name: "Page 1".into(),
                buttons: vec![Button {
                    id: bid,
                    label: "Click Me".into(),
                    action: ActionReference {
                        action_name: "launch".into(),
                        payload: serde_json::json!({"app": "notepad.exe"}),
                    },
                }],
            }],
        }];
        doc
    }

    // ── Profile commands ──

    #[test]
    fn create_profile() {
        let doc = fixture();
        let result = apply(
            &doc,
            &Command::CreateProfile {
                profile_id: ProfileId::from_string("new-profile"),
                initial_page_id: PageId::from_string("initial-page"),
                name: "New".into(),
            },
        );
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.profiles.len(), 2);
        assert_eq!(out.profiles[1].id.as_str(), "new-profile");
        assert_eq!(out.profiles[1].name, "New");
        assert_eq!(out.profiles[1].pages.len(), 1);
        assert_eq!(out.profiles[1].pages[0].id.as_str(), "initial-page");
    }

    #[test]
    fn delete_profile() {
        // Start with 2 profiles so deletion of one is valid
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let doc = apply(
            &doc,
            &Command::CreateProfile {
                profile_id: ProfileId::from_string("second"),
                initial_page_id: PageId::from_string("second-page"),
                name: "Second".into(),
            },
        )
        .unwrap();
        let result = apply(&doc, &Command::DeleteProfile { profile_id: pid });
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles.len(), 1);
    }

    #[test]
    fn reject_delete_last_profile() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let result = apply(&doc, &Command::DeleteProfile { profile_id: pid });
        assert_eq!(result, Err(CommandError::CannotDeleteLastProfile));
    }

    #[test]
    fn delete_missing_profile_from_single_profile_document_returns_not_found() {
        let doc = fixture();
        let result = apply(
            &doc,
            &Command::DeleteProfile {
                profile_id: ProfileId::from_string("nope"),
            },
        );
        assert_eq!(result, Err(CommandError::ProfileNotFound));
    }

    #[test]
    fn rename_profile() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let result = apply(
            &doc,
            &Command::RenameProfile {
                profile_id: pid,
                new_name: "Renamed".into(),
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].name, "Renamed");
    }

    // ── Page commands ──

    #[test]
    fn add_page() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let result = apply(
            &doc,
            &Command::AddPage {
                profile_id: pid,
                page_id: PageId::from_string("page-b"),
                name: "Page 2".into(),
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].pages.len(), 2);
    }

    #[test]
    fn reject_delete_last_page() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let result = apply(
            &doc,
            &Command::DeletePage {
                profile_id: pid,
                page_id: pgid,
            },
        );
        assert_eq!(result, Err(CommandError::CannotDeleteLastPage));
    }

    #[test]
    fn delete_page() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        // Add a second page first so we can delete one
        let doc = apply(
            &doc,
            &Command::AddPage {
                profile_id: pid.clone(),
                page_id: PageId::from_string("page-b"),
                name: "Page 2".into(),
            },
        )
        .unwrap();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let result = apply(
            &doc,
            &Command::DeletePage {
                profile_id: pid,
                page_id: pgid,
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].pages.len(), 1);
    }

    #[test]
    fn delete_missing_page_from_single_page_profile_returns_not_found() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let result = apply(
            &doc,
            &Command::DeletePage {
                profile_id: pid,
                page_id: PageId::from_string("nope"),
            },
        );
        assert_eq!(result, Err(CommandError::PageNotFound));
    }

    #[test]
    fn rename_page() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let result = apply(
            &doc,
            &Command::RenamePage {
                profile_id: pid,
                page_id: pgid,
                new_name: "Renamed".into(),
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].pages[0].name, "Renamed");
    }

    // ── Button commands ──

    #[test]
    fn add_button() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let btn = Button {
            id: ButtonId::from_string("btn-b"),
            label: "New Button".into(),
            action: ActionReference {
                action_name: "lock".into(),
                payload: serde_json::json!({}),
            },
        };
        let result = apply(
            &doc,
            &Command::AddButton {
                profile_id: pid,
                page_id: pgid,
                button: btn,
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].pages[0].buttons.len(), 2);
    }

    #[test]
    fn remove_button() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let result = apply(
            &doc,
            &Command::RemoveButton {
                profile_id: pid,
                page_id: pgid,
                button_id: bid,
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profiles[0].pages[0].buttons.len(), 0);
    }

    #[test]
    fn update_button_preserves_id() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let updated = Button {
            id: bid.clone(),
            label: "Updated".into(),
            action: ActionReference {
                action_name: "open_url".into(),
                payload: serde_json::json!({"url": "https://example.com"}),
            },
        };
        let result = apply(
            &doc,
            &Command::UpdateButton {
                profile_id: pid,
                page_id: pgid,
                button: updated,
            },
        );
        assert!(result.is_ok());
        let out = result.unwrap();
        let b = &out.profiles[0].pages[0].buttons[0];
        assert_eq!(b.id, bid);
        assert_eq!(b.label, "Updated");
        assert_eq!(b.action.action_name, "open_url");
    }

    #[test]
    fn move_button_between_pages() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pg_a = doc.profiles[0].pages[0].id.clone();
        // Add a second page
        let doc = apply(
            &doc,
            &Command::AddPage {
                profile_id: pid.clone(),
                page_id: PageId::from_string("page-b"),
                name: "Page B".into(),
            },
        )
        .unwrap();
        let pg_b = doc.profiles[0].pages[1].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        let result = apply(
            &doc,
            &Command::MoveButton {
                profile_id: pid,
                from_page: pg_a,
                button_id: bid.clone(),
                to_page: pg_b,
            },
        );
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.profiles[0].pages[0].buttons.len(), 0);
        assert_eq!(out.profiles[0].pages[1].buttons.len(), 1);
        assert_eq!(out.profiles[0].pages[1].buttons[0].id, bid);
    }

    // ── Error cases ──

    #[test]
    fn reject_missing_profile() {
        let doc = fixture();
        let missing = ProfileId::from_string("no-such-profile");
        let result = apply(
            &doc,
            &Command::RenameProfile {
                profile_id: missing,
                new_name: "x".into(),
            },
        );
        assert_eq!(result, Err(CommandError::ProfileNotFound));
    }

    #[test]
    fn reject_missing_page() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let missing = PageId::from_string("no-such-page");
        let result = apply(
            &doc,
            &Command::RenamePage {
                profile_id: pid,
                page_id: missing,
                new_name: "x".into(),
            },
        );
        assert_eq!(result, Err(CommandError::PageNotFound));
    }

    #[test]
    fn reject_missing_button() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pgid = doc.profiles[0].pages[0].id.clone();
        let missing = ButtonId::from_string("no-such-button");
        let result = apply(
            &doc,
            &Command::RemoveButton {
                profile_id: pid,
                page_id: pgid,
                button_id: missing,
            },
        );
        assert_eq!(result, Err(CommandError::ButtonNotFound));
    }

    #[test]
    fn failed_command_does_not_mutate_input() {
        let doc = fixture();
        let original = doc.clone();
        let missing = ProfileId::from_string("nowhere");
        let _ = apply(
            &doc,
            &Command::DeleteProfile {
                profile_id: missing,
            },
        );
        assert_eq!(doc, original);
    }

    // ── Determinism ──

    #[test]
    fn create_profile_deterministic() {
        let doc = fixture();
        let cmd = Command::CreateProfile {
            profile_id: ProfileId::from_string("fixed-id"),
            initial_page_id: PageId::from_string("fixed-page"),
            name: "Same".into(),
        };
        assert_eq!(apply(&doc, &cmd), apply(&doc, &cmd));
        let out = apply(&doc, &cmd).unwrap();
        assert_eq!(out.profiles[1].id.as_str(), "fixed-id");
        assert_eq!(out.profiles[1].pages[0].id.as_str(), "fixed-page");
    }

    #[test]
    fn add_page_deterministic() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let cmd = Command::AddPage {
            profile_id: pid,
            page_id: PageId::from_string("fixed-page"),
            name: "Same".into(),
        };
        assert_eq!(apply(&doc, &cmd), apply(&doc, &cmd));
    }

    // ── Identity ownership ──

    #[test]
    fn create_profile_preserves_supplied_ids() {
        let doc = fixture();
        let result = apply(
            &doc,
            &Command::CreateProfile {
                profile_id: ProfileId::from_string("pro-one"),
                initial_page_id: PageId::from_string("page-one"),
                name: "One".into(),
            },
        )
        .unwrap();
        let result = apply(
            &result,
            &Command::CreateProfile {
                profile_id: ProfileId::from_string("pro-two"),
                initial_page_id: PageId::from_string("page-two"),
                name: "Two".into(),
            },
        )
        .unwrap();
        assert_eq!(result.profiles[1].id.as_str(), "pro-one");
        assert_eq!(result.profiles[1].pages[0].id.as_str(), "page-one");
        assert_eq!(result.profiles[2].id.as_str(), "pro-two");
        assert_eq!(result.profiles[2].pages[0].id.as_str(), "page-two");
    }

    #[test]
    fn move_button_does_not_duplicate() {
        let doc = fixture();
        let pid = doc.profiles[0].id.clone();
        let pg_a = doc.profiles[0].pages[0].id.clone();
        let bid = doc.profiles[0].pages[0].buttons[0].id.clone();
        // Move to same page — should fail
        let result = apply(
            &doc,
            &Command::MoveButton {
                profile_id: pid,
                from_page: pg_a.clone(),
                button_id: bid,
                to_page: pg_a,
            },
        );
        assert_eq!(result, Err(CommandError::DuplicateButtonId));
    }
}
