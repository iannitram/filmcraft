//! Source ownership commands. All validation precedes the undoable edit; queries only read.
use crate::{Result, Session, commands::bad};
use filmcraft_project::{BinEntry, BinId, ItemId, SourceClass, SourceLabel, SourceRule, SourceRuleKind, source_folder_key};
use serde_json::{Value, json};

fn object(p: &Value, id: &str) -> Result<()> {
    if p.is_object() { Ok(()) } else { Err(bad(id, "parameters must be an object")) }
}

fn class(p: &Value, id: &str) -> Result<Option<SourceClass>> {
    let value = p.get("class").ok_or_else(|| bad(id, "need class"))?;
    if value.is_null() {
        return Ok(None);
    }
    match value.as_str() {
        Some("mine") => Ok(Some(SourceClass::Mine)),
        Some("borrowedFootage") => Ok(Some(SourceClass::BorrowedFootage)),
        Some("borrowedMusic") => Ok(Some(SourceClass::BorrowedMusic)),
        Some("unknown") => Ok(Some(SourceClass::Unknown)),
        _ => Err(bad(id, "class must be mine, borrowedFootage, borrowedMusic, unknown or null")),
    }
}

fn citation(p: &Value, previous: Option<&SourceLabel>, id: &str) -> Result<Option<String>> {
    match p.get("citation") {
        None => Ok(previous.and_then(|l| l.citation.clone())),
        Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|s| s.len() <= 1024 && !s.chars().any(char::is_control))
            .map(|s| (!s.is_empty()).then(|| s.to_string()))
            .ok_or_else(|| bad(id, "citation must be text of at most 1024 bytes without control characters, or null")),
    }
}

fn bin_exists(s: &Session, id: BinId) -> bool {
    let mut bins = vec![&s.project.root];
    while let Some(bin) = bins.pop() {
        if bin.id == id {
            return true;
        }
        bins.extend(bin.children.iter().filter_map(|e| if let BinEntry::Bin(b) = e { Some(b) } else { None }));
    }
    false
}

fn source_exists(s: &Session, item: ItemId) -> bool {
    s.project.item(item).is_some_and(|i| i.as_sequence().is_none())
}

pub(crate) fn set_class(s: &mut Session, p: &Value) -> Result<Value> {
    let id = "ledger.setClass";
    object(p, id)?;
    let (item, bin, previous) = match (p.get("item"), p.get("bin")) {
        (Some(v), None) => {
            let item = ItemId(v.as_u64().ok_or_else(|| bad(id, "item must be an unsigned id"))?);
            if !source_exists(s, item) {
                return Err(bad(id, "no such source item"));
            }
            (Some(item), None, s.project.source_ledger.items.get(&item))
        }
        (None, Some(v)) => {
            let bin = BinId(v.as_u64().ok_or_else(|| bad(id, "bin must be an unsigned id"))?);
            if !bin_exists(s, bin) {
                return Err(bad(id, "no such bin"));
            }
            (None, Some(bin), s.project.source_ledger.bins.get(&bin))
        }
        _ => return Err(bad(id, "name exactly one item or bin")),
    };
    let class = class(p, id)?;
    let citation = citation(p, previous, id)?;
    if class.is_none() && p.get("citation").is_some_and(|v| !v.is_null()) {
        return Err(bad(id, "clearing an override also clears its citation"));
    }
    let label = class.map(|class| SourceLabel { class, citation });
    s.edit("Set Source Class", |pr, _| {
        if let Some(item) = item {
            match &label {
                Some(label) => {
                    pr.source_ledger.items.insert(item, label.clone());
                }
                None => {
                    pr.source_ledger.items.remove(&item);
                }
            }
        }
        if let Some(bin) = bin {
            match &label {
                Some(label) => {
                    pr.source_ledger.bins.insert(bin, label.clone());
                }
                None => {
                    pr.source_ledger.bins.remove(&bin);
                }
            }
        }
        Ok(())
    })?;
    Ok(json!({"item": item, "bin": bin, "label": label}))
}

pub(crate) fn set_rule(s: &mut Session, p: &Value) -> Result<Value> {
    let id = "ledger.setRule";
    object(p, id)?;
    let folder = p
        .get("folder")
        .and_then(Value::as_str)
        .and_then(source_folder_key)
        .ok_or_else(|| bad(id, "need a folder of at most 8192 bytes without parent traversal or control characters"))?;
    let kind = match p.get("kind").map(Value::as_str) {
        None | Some(Some("folder")) => SourceRuleKind::Folder,
        Some(Some("libraryIndex")) => SourceRuleKind::LibraryIndex,
        _ => return Err(bad(id, "kind must be folder or libraryIndex")),
    };
    let class = if kind == SourceRuleKind::LibraryIndex && p.get("class").is_none() { Some(SourceClass::BorrowedFootage) } else { class(p, id)? };
    if kind == SourceRuleKind::LibraryIndex && class.is_some_and(|c| c != SourceClass::BorrowedFootage) {
        return Err(bad(id, "a libraryIndex rule must use borrowedFootage"));
    }
    let previous = s.project.source_ledger.rules.iter().find(|r| source_folder_key(&r.folder).as_ref() == Some(&folder));
    let citation = citation(p, previous.map(|r| &r.label), id)?;
    if class.is_none() && p.get("citation").is_some_and(|v| !v.is_null()) {
        return Err(bad(id, "removing a rule also clears its citation"));
    }
    if class.is_some() && previous.is_none() && s.project.source_ledger.rules.len() >= 256 {
        return Err(bad(id, "at most 256 source rules are supported"));
    }
    let rule = class.map(|class| SourceRule { folder: folder.clone(), kind, label: SourceLabel { class, citation } });
    s.edit("Set Source Rule", |pr, _| {
        pr.source_ledger.rules.retain(|r| source_folder_key(&r.folder).as_ref() != Some(&folder));
        if let Some(rule) = &rule {
            pr.source_ledger.rules.push(rule.clone());
        }
        Ok(())
    })?;
    Ok(json!({"rule": rule}))
}

pub(crate) fn inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let id = "ledger.inspectSources";
    object(p, id)?;
    let item = p.get("item").map(|v| v.as_u64().map(ItemId).ok_or_else(|| bad(id, "item must be an unsigned id"))).transpose()?;
    if item.is_some_and(|item| !source_exists(s, item)) {
        return Err(bad(id, "no such source item"));
    }
    if item.is_none() && s.project.items.len() > 100_000 {
        return Err(bad(id, "too many items to inspect at once; name an item"));
    }
    let sources: Vec<Value> = s
        .project
        .items
        .values()
        .filter(|i| i.as_sequence().is_none() && item.is_none_or(|id| i.id == id))
        .map(|i| {
            let label = s.project.source_label(i.id).unwrap_or_default();
            json!({"item":i.id, "name":i.name, "class":label.class, "citation":label.citation, "override":s.project.source_ledger.items.get(&i.id)})
        })
        .collect();
    Ok(json!({"sources":sources, "bins":s.project.source_ledger.bins, "rules":s.project.source_ledger.rules}))
}

#[cfg(test)]
mod tests;
