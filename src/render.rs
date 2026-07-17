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
        Some(format!(
            "\n… {} more (--limit 0 for all)",
            total - shown
        ))
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
