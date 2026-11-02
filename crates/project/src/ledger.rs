//! Saved source labels and live inheritance. No filesystem access is needed to classify a source.
use crate::{BinEntry, BinId, ItemId, MediaRef, Project};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceClass {
    Mine,
    BorrowedFootage,
    BorrowedMusic,
    #[default]
    Unknown,
}

/// An explicit label. Unknown is a real override; absence means inherit.
/// Citation labels belong to the assignment: an item override does not inherit its bin or
/// folder rule's citation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLabel {
    pub class: SourceClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceRuleKind {
    Folder,
    /// D10 library-index roots always supply borrowed footage.
    LibraryIndex,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRule {
    pub folder: String,
    pub kind: SourceRuleKind,
    pub label: SourceLabel,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLedger {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub items: BTreeMap<ItemId, SourceLabel>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bins: BTreeMap<BinId, SourceLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<SourceRule>,
}

impl SourceLedger {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.bins.is_empty() && self.rules.is_empty()
    }
}

/// Portable lexical folder keys: `/` and `\` are separators; no disk access, symlink following
/// or case folding. Reject parent traversal rather than letting it escape a matching root.
/// Preserve absolute/relative paths, drive prefixes and share roots; collapse repeated separators.
pub fn folder_key(path: &str) -> Option<String> {
    if path.is_empty() || path.len() > 8192 || path.chars().any(char::is_control) {
        return None;
    }
    let path = path.replace('\\', "/");
    if path.split('/').any(|c| c == "..") {
        return None;
    }
    let root = if path.starts_with("//") {
        "//"
    } else if path.starts_with('/') {
        "/"
    } else {
        ""
    };
    let tail = path.split('/').filter(|c| !c.is_empty() && *c != ".").collect::<Vec<_>>().join("/");
    (!tail.is_empty() || root == "/").then(|| format!("{root}{tail}"))
}

impl Project {
    /// Item override > nearest bin > longest folder rule > unknown. Rules are evaluated live,
    /// including for newly imported items; inherited labels never become explicit overrides.
    /// Bin traversal is iterative, and media resolution already bounds subclip chains.
    pub fn source_label(&self, item: ItemId) -> Option<SourceLabel> {
        self.item(item)?;
        if let Some(label) = self.source_ledger.items.get(&item) {
            return Some(label.clone());
        }
        let mut bins = vec![(&self.root, None)];
        while let Some((bin, inherited)) = bins.pop() {
            let label = self.source_ledger.bins.get(&bin.id).or(inherited);
            for entry in &bin.children {
                match entry {
                    BinEntry::Item(id) if *id == item => {
                        if let Some(label) = label {
                            return Some(label.clone());
                        }
                    }
                    BinEntry::Bin(child) => bins.push((child, label)),
                    _ => {}
                }
            }
        }
        let path = self.resolve_media(item).and_then(|(_, m, _)| match &m.media {
            MediaRef::File { path } => folder_key(path),
            MediaRef::Generator(_) => None,
        });
        let rule = path.as_ref().and_then(|path| {
            self.source_ledger
                .rules
                .iter()
                .filter_map(|rule| {
                    let key = folder_key(&rule.folder)?;
                    // A root matches descendants only, never a sibling with the same prefix.
                    let rest = path.strip_prefix(key.as_str())?;
                    (!rest.is_empty() && (key.ends_with('/') || rest.starts_with('/'))).then_some((key.len(), rule))
                })
                .max_by_key(|(len, _)| *len)
                .map(|(_, rule)| rule)
        });
        Some(match rule {
            Some(rule) => SourceLabel {
                class: if rule.kind == SourceRuleKind::LibraryIndex { SourceClass::BorrowedFootage } else { rule.label.class },
                citation: rule.label.citation.clone(),
            },
            None => SourceLabel::default(),
        })
    }
}

#[cfg(test)]
mod tests;
