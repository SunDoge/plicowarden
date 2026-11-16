//! An unlocked-account search index. This connection never opens a disk file.
use bitwarden_vault::{CipherRepromptType, CipherView};
use rusqlite::{Connection, params, params_from_iter};

pub(crate) struct SearchIndex(Connection);
impl SearchIndex {
    pub(crate) fn build(views: &[CipherView]) -> Result<Self, String> {
        let mut index = Self::empty()?;
        let transaction = index
            .0
            .transaction()
            .map_err(|_| "Could not start search indexing")?;
        {
            let mut insert = transaction
                .prepare("INSERT INTO vault_search(id,content) VALUES(?1,?2)")
                .map_err(|_| "Could not prepare search index")?;
            for view in views {
                let Some(id) = view.id else { continue };
                let mut targets = vec![view.name.clone()];
                if let Some(login) = &view.login {
                    if let Some(username) = &login.username {
                        targets.push(username.clone());
                    }
                    if let Some(uris) = &login.uris {
                        targets.extend(uris.iter().filter_map(|uri| uri.uri.clone()));
                    }
                }
                if view.reprompt == CipherRepromptType::None {
                    if let Some(notes) = &view.notes {
                        targets.push(notes.clone());
                    }
                    if let Some(fields) = &view.fields {
                        targets.extend(
                            fields
                                .iter()
                                .filter(|f| f.r#type == bitwarden_vault::FieldType::Text)
                                .filter_map(|f| f.value.clone()),
                        );
                    }
                }
                insert
                    .execute(params![id.to_string(), targets.join("\n").to_lowercase()])
                    .map_err(|_| "Could not index vault item")?;
            }
        }
        transaction
            .commit()
            .map_err(|_| "Could not commit search index")?;
        Ok(index)
    }
    fn empty() -> Result<Self, String> {
        let connection =
            Connection::open_in_memory().map_err(|_| "Could not open in-memory search index")?;
        connection.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA journal_mode=MEMORY; CREATE VIRTUAL TABLE vault_search USING fts5(id UNINDEXED,content,tokenize='trigram');").map_err(|_|"Could not initialize full-text search")?;
        Ok(Self(connection))
    }
    pub(crate) fn query(&self, query: &str) -> Result<Vec<String>, String> {
        let parts: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
        let long: Vec<_> = parts
            .iter()
            .filter(|p| p.chars().count() >= 3)
            .map(|p| format!("\"{}\"", p.replace('"', "\"\"")))
            .collect();
        let mut sql = "SELECT id FROM vault_search WHERE 1=1".to_owned();
        let mut values = Vec::new();
        if !long.is_empty() {
            sql.push_str(" AND vault_search MATCH ?");
            values.push(long.join(" AND "));
        }
        for part in parts.iter().filter(|p| p.chars().count() < 3) {
            sql.push_str(" AND content LIKE ? ESCAPE '\\'");
            values.push(format!(
                "%{}%",
                part.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            ));
        }
        let mut stmt = self
            .0
            .prepare(&sql)
            .map_err(|_| "Could not prepare local search")?;
        let rows = stmt
            .query_map(params_from_iter(values), |row| row.get::<_, String>(0))
            .map_err(|_| "Local search failed")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Could not read local search results".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_text_search_matches_multiple_targets_unicode_substrings_and_literal_punctuation() {
        let index = SearchIndex::empty().unwrap();
        index
            .0
            .execute(
                "INSERT INTO vault_search(id,content) VALUES(?1,?2)",
                params![
                    "one",
                    "github\nalice@example.invalid\n项目 文档\n100%_complete"
                ],
            )
            .unwrap();
        index
            .0
            .execute(
                "INSERT INTO vault_search(id,content) VALUES(?1,?2)",
                params!["two", "mail\nbob@example.invalid\nother notes"],
            )
            .unwrap();
        for query in [
            "GITHUB alice",
            "github 项目",
            "hub 文档",
            "%_",
            "alice@example.invalid",
        ] {
            assert_eq!(index.query(query).unwrap(), vec!["one"], "{query}");
        }
        assert!(index.query("github bob").unwrap().is_empty());
        assert!(index.query("\" OR 1=1 --").unwrap().is_empty());
        assert_eq!(
            index
                .0
                .query_row("PRAGMA database_list", [], |row| row.get::<_, String>(2))
                .unwrap(),
            ""
        );
    }
}
