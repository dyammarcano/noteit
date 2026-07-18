use crate::store::contexts::Context;
use crate::store::notes::{Note, Status};

const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Compact base36 id -- what `list` prints and `done`/`open` accept.
/// Raw rowids are an implementation detail users should not have to type.
///
/// Assumes positive SQLite rowids (>= 1); non-positive input renders `"0"` and does not round-trip.
pub fn short_id(id: i64) -> String {
    if id <= 0 {
        return "0".to_string();
    }
    let mut n = id as u64;
    let mut buf = Vec::new();
    while n > 0 {
        buf.push(ALPHABET[(n % 36) as usize]);
        n /= 36;
    }
    buf.reverse();
    String::from_utf8(buf).expect("ascii")
}

pub fn parse_short_id(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    let mut acc: i64 = 0;
    for c in s.chars() {
        let d = c.to_digit(36)? as i64;
        acc = acc.checked_mul(36)?.checked_add(d)?;
    }
    Some(acc)
}

fn status_mark(s: Status) -> &'static str {
    match s {
        Status::Open => " ",
        Status::Done => "x",
    }
}

fn truncation_notice(shown: usize, total: usize, limit: Option<usize>) -> Option<String> {
    let _limit = limit?;
    if total > shown {
        Some(format!("\n… {} more (--limit 0 for all)", total - shown))
    } else {
        None
    }
}

pub fn render_list(notes: &[Note], limit: Option<usize>, total: usize) -> String {
    if notes.is_empty() {
        return "no notes here yet".to_string();
    }
    let mut out = String::new();
    for n in notes {
        out.push_str(&format!(
            "[{}] {}  {}\n",
            status_mark(n.status),
            short_id(n.id),
            n.body
        ));
    }
    if let Some(notice) = truncation_notice(notes.len(), total, limit) {
        out.push_str(&notice);
    }
    out.trim_end().to_string()
}

/// Renders notes grouped by context (project).
///
/// REQUIRES rows to be pre-sorted by context id; if unsorted, emits repeated project headers
/// with no error. The caller is responsible for sorting.
pub fn render_grouped(rows: &[(Context, Note)], total: usize, limit: Option<usize>) -> String {
    if rows.is_empty() {
        return "no notes yet".to_string();
    }
    let mut out = String::new();
    let mut current: Option<i64> = None;
    for (ctx, n) in rows {
        if current != Some(ctx.id) {
            if current.is_some() {
                out.push('\n');
            }
            out.push_str(&format!("{}\n", ctx.display_name));
            current = Some(ctx.id);
        }
        out.push_str(&format!(
            "  [{}] {}  {}\n",
            status_mark(n.status),
            short_id(n.id),
            n.body
        ));
    }
    if let Some(notice) = truncation_notice(rows.len(), total, limit) {
        out.push_str(&notice);
    }
    out.trim_end().to_string()
}

pub fn render_flat(rows: &[(Context, Note)], total: usize, limit: Option<usize>) -> String {
    if rows.is_empty() {
        return "no notes yet".to_string();
    }
    let mut out = String::new();
    for (ctx, n) in rows {
        out.push_str(&format!(
            "[{}] {}  {:<12}  {}\n",
            status_mark(n.status),
            short_id(n.id),
            ctx.display_name,
            n.body
        ));
    }
    if let Some(notice) = truncation_notice(rows.len(), total, limit) {
        out.push_str(&notice);
    }
    out.trim_end().to_string()
}

/// Escape a string as a JSON string body (without the surrounding quotes).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Serialize every note (with its context) as a stable, hand-rolled JSON
/// document for `noteit export`. Std-only — no serde dependency.
pub fn export_json(rows: &[(Context, Note)]) -> String {
    let mut out = String::from("{\n  \"noteit_export\": \"v1\",\n  \"notes\": [");
    for (i, (ctx, n)) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    {");
        out.push_str(&format!("\"id\": {}, ", n.id));
        out.push_str(&format!("\"context_key\": \"{}\", ", json_escape(&ctx.key)));
        out.push_str(&format!(
            "\"project\": \"{}\", ",
            json_escape(&ctx.display_name)
        ));
        out.push_str(&format!("\"subpath\": \"{}\", ", json_escape(&n.subpath)));
        out.push_str(&format!("\"body\": \"{}\", ", json_escape(&n.body)));
        out.push_str(&format!("\"status\": \"{}\", ", n.status.as_str()));
        out.push_str(&format!("\"created_at\": {}, ", n.created_at));
        out.push_str(&format!("\"updated_at\": {}", n.updated_at));
        out.push('}');
    }
    if rows.is_empty() {
        out.push_str("]\n}\n");
    } else {
        out.push_str("\n  ]\n}\n");
    }
    out
}
