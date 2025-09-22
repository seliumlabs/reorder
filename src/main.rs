use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{Attribute, File, Item};

type Cat = usize;

fn main() -> Result<()> {
    let inputs: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if inputs.is_empty() {
        bail!("usage: selium_order <files>...");
    }

    let files = collect_input_files(inputs)?;

    for path in files {
        reorder_file(&path).with_context(|| format!("reorder {}", path.display()))?;
    }

    Ok(())
}

fn collect_input_files(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        collect_path(&path, &mut files, &mut seen)?;
    }

    if files.is_empty() {
        bail!("no Rust files found");
    }

    Ok(files)
}

fn collect_path(path: &Path, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("inspect metadata for {}", path.display()))?;

    if metadata.is_dir() {
        collect_directory(path, files, seen)?;
    } else if metadata.is_file() {
        push_file(path.to_path_buf(), files, seen);
    }

    Ok(())
}

fn collect_directory(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<()> {
    let mut queue = std::collections::VecDeque::from([dir.to_path_buf()]);

    while let Some(current) = queue.pop_front() {
        let mut entries = Vec::new();
        let read_dir = fs::read_dir(&current)
            .with_context(|| format!("read directory {}", current.display()))?;

        for entry in read_dir {
            let entry = entry.with_context(|| format!("read entry in {}", current.display()))?;
            entries.push(entry);
        }

        entries.sort_by(|a, b| a.path().cmp(&b.path()));

        for entry in entries {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("determine type for {}", path.display()))?;

            if file_type.is_dir() {
                queue.push_back(path);
            } else if file_type.is_file() {
                if is_rust_file(&path) {
                    push_file(path, files, seen);
                }
            } else if file_type.is_symlink() {
                let metadata = fs::metadata(&path)
                    .with_context(|| format!("inspect symlink target {}", path.display()))?;
                if metadata.is_dir() {
                    continue;
                } else if metadata.is_file() && is_rust_file(&path) {
                    push_file(path, files, seen);
                }
            }
        }
    }

    Ok(())
}

fn push_file(path: PathBuf, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if seen.insert(path.clone()) {
        files.push(path);
    }
}

fn is_rust_file(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("rs"),
        None => false,
    }
}

fn reorder_file(path: &Path) -> Result<()> {
    let src = fs::read_to_string(path).with_context(|| format!("read file {}", path.display()))?;
    let mut file: File =
        syn::parse_file(&src).with_context(|| format!("parse {}", path.display()))?;
    let line_starts = line_start_offsets(&src);

    let shebang = file.shebang.take();
    let crate_attrs = std::mem::take(&mut file.attrs);

    let mut buckets: Vec<Vec<String>> = vec![Vec::new(); 8];
    for item in file.items.into_iter() {
        let cat = category(&item);
        let snippet = item_snippet(&item, &src, &line_starts);
        buckets[cat].push(snippet);
    }

    let mut out = String::new();
    if let Some(sb) = shebang {
        out.push_str(&sb);
        out.push('\n');
    }
    if !crate_attrs.is_empty() {
        let header = header_to_string(&crate_attrs, &src, &line_starts);
        out.push_str(header.trim_end());
        out.push_str("\n\n");
    }

    let mut wrote_any = !out.is_empty();

    for (idx, bucket) in buckets.into_iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }

        if wrote_any && idx != 0 {
            while !out.ends_with("\n\n") {
                out.push('\n');
            }
        }
        wrote_any = true;

        let extra_blank = blank_lines_after(idx);

        for item in bucket {
            out.push_str(item.trim_end_matches('\n'));
            out.push('\n');
            for _ in 0..extra_blank {
                out.push('\n');
            }
        }
    }

    while out.ends_with("\n\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }

    if out != src {
        fs::write(path, out)?;
    }

    Ok(())
}

fn header_to_string(attrs: &[Attribute], src: &str, line_starts: &[usize]) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let mut start = usize::MAX;
    let mut end = 0usize;

    for attr in attrs {
        let range = span_range(attr.span(), line_starts, src.len());
        start = start.min(range.start);
        end = end.max(range.end);
    }

    src[start..end].to_string()
}

fn category(item: &Item) -> Cat {
    if is_test_module(item) {
        return 7;
    }

    match item {
        Item::Use(_) | Item::ExternCrate(_) => 0,
        Item::Type(_) => 1,
        Item::Const(_) | Item::Static(_) => 2,
        Item::Trait(_) | Item::TraitAlias(_) => 3,
        Item::Struct(_) | Item::Enum(_) | Item::Union(_) | Item::Mod(_) => 4,
        Item::Impl(_) => 5,
        Item::Fn(_) | Item::ForeignMod(_) | Item::Macro(_) | Item::Verbatim(_) => 6,
        _ => 6,
    }
}

fn blank_lines_after(category: usize) -> usize {
    match category {
        0 | 2 => 0,
        _ => 1,
    }
}

fn is_test_module(item: &Item) -> bool {
    match item {
        Item::Mod(module) => has_cfg_test(&module.attrs) || module.ident == "tests",
        _ => false,
    }
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        match attr.parse_args::<syn::Expr>() {
            Ok(expr) => contains_test(&expr),
            Err(_) => false,
        }
    })
}

fn contains_test(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(path) => path.path.is_ident("test"),
        syn::Expr::Tuple(tuple) => tuple.elems.iter().any(contains_test),
        syn::Expr::Binary(bin) => contains_test(&bin.left) || contains_test(&bin.right),
        syn::Expr::Group(group) => contains_test(&group.expr),
        syn::Expr::Call(call) => {
            if let syn::Expr::Path(path) = &*call.func {
                if path.path.is_ident("any") || path.path.is_ident("all") {
                    return call.args.iter().any(contains_test);
                }
            }
            false
        }
        _ => false,
    }
}

fn line_start_offsets(src: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(src.len() / 32 + 2);
    starts.push(0);
    for (idx, ch) in src.char_indices() {
        if ch == '\n' {
            let next = idx + ch.len_utf8();
            starts.push(next);
        }
    }
    if *starts.last().unwrap_or(&0) != src.len() {
        starts.push(src.len());
    }
    starts
}

fn span_range(
    span: proc_macro2::Span,
    line_starts: &[usize],
    src_len: usize,
) -> std::ops::Range<usize> {
    let start = span.start();
    let end = span.end();

    let start_line_index = start.line.saturating_sub(1);
    let end_line_index = end.line.saturating_sub(1);

    let start_line_base = line_starts
        .get(start_line_index)
        .copied()
        .unwrap_or(src_len);
    let end_line_base = line_starts.get(end_line_index).copied().unwrap_or(src_len);

    let mut start_idx = start_line_base.saturating_add(start.column);
    let mut end_idx = end_line_base.saturating_add(end.column);

    if start_idx > src_len {
        start_idx = src_len;
    }
    if end_idx > src_len {
        end_idx = src_len;
    }

    if start_idx > end_idx {
        start_idx = end_idx;
    }

    start_idx..end_idx
}

fn item_snippet(item: &Item, src: &str, line_starts: &[usize]) -> String {
    let mut range = span_range(item.span(), line_starts, src.len());

    for attr in item_attributes(item) {
        let attr_range = span_range(attr.span(), line_starts, src.len());
        if attr_range.start < range.start {
            range.start = attr_range.start;
        }
    }

    range.start = range.start.min(range.end);

    src[range].to_string()
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        Item::Verbatim(_) => &[],
        _ => &[],
    }
}
