use std::fmt::Write;
use std::path::Path;

use crate::models::node::{human_readable_size, Node, NodeType};
use crate::models::scan_result::ScanResult;

pub fn export_html(result: &ScanResult, output_path: &Path) -> anyhow::Result<()> {
    let mut html = String::new();

    write!(html, r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>DiskLens Report</title>
<style>
    body {{ font-family: -apple-system, system-ui, sans-serif; margin: 20px; background: #1a1a2e; color: #e0e0e0; }}
    h1 {{ color: #00d4ff; }}
    h2 {{ color: #5dade2; margin-top: 30px; }}
    .summary {{ background: #16213e; padding: 15px; border-radius: 8px; margin-bottom: 20px; }}
    .summary p {{ margin: 6px 0; }}
    .summary strong {{ color: #00d4ff; }}
    .node {{ display: flex; align-items: center; padding: 4px 0; }}
    .name {{ min-width: 300px; }}
    .size {{ min-width: 100px; text-align: right; color: #aaa; margin-right: 10px; }}
    .pct {{ min-width: 50px; text-align: right; color: #888; margin-right: 10px; }}
    .bar {{ width: 200px; height: 16px; background: #0f3460; border-radius: 3px; overflow: hidden; }}
    .bar-fill {{ height: 100%; border-radius: 3px; background: linear-gradient(90deg, #00d4ff, #0f3460); }}
    .dir {{ color: #5dade2; }}
    .file {{ color: #aaa; }}
    .error {{ color: #e74c3c; }}
    .error-list {{ background: #2c1a1a; padding: 15px; border-radius: 8px; border-left: 3px solid #e74c3c; }}
    details {{ margin-left: 20px; }}
    summary {{ cursor: pointer; padding: 4px; }}
    summary:hover {{ background: #16213e; border-radius: 4px; }}
</style>
</head>
<body>
"#)?;

    // Summary section
    write!(html, "<h1>DiskLens Report</h1>\n")?;
    write!(html, "<div class=\"summary\">\n")?;
    write!(html, "<p><strong>Path:</strong> {}</p>\n", escape_html(&result.scan_path.display().to_string()))?;
    write!(html, "<p><strong>Total Size:</strong> {}</p>\n", human_readable_size(result.total_size))?;
    write!(html, "<p><strong>Files:</strong> {}</p>\n", result.total_files)?;
    write!(html, "<p><strong>Directories:</strong> {}</p>\n", result.total_dirs)?;
    write!(html, "<p><strong>Scan Duration:</strong> {:.2}s</p>\n", result.scan_duration.as_secs_f64())?;
    write!(html, "</div>\n")?;

    // Directory tree
    write!(html, "<h2>Directory Tree</h2>\n")?;
    write_node_html(&mut html, &result.root, result.total_size, 0, 4)?;

    // Error list
    if !result.errors.is_empty() {
        write!(html, "<h2>Errors ({} total)</h2>\n", result.errors.len())?;
        write!(html, "<div class=\"error-list\">\n<ul>\n")?;
        for err in &result.errors {
            write!(
                html,
                "<li class=\"error\"><strong>{:?}</strong>: {}</li>\n",
                err.error_type,
                escape_html(&err.path.display().to_string()),
            )?;
        }
        write!(html, "</ul>\n</div>\n")?;
    }

    write!(html, "</body>\n</html>")?;

    std::fs::write(output_path, html)?;
    Ok(())
}

fn write_node_html(
    html: &mut String,
    node: &Node,
    total_size: u64,
    depth: usize,
    max_depth: usize,
) -> std::fmt::Result {
    if depth > max_depth {
        return Ok(());
    }

    let pct = node.percentage(total_size);
    let bar_width = (pct * 2.0).min(200.0);
    let name_class = match node.node_type {
        NodeType::Directory => "dir",
        _ => "file",
    };
    let icon = match node.node_type {
        NodeType::Directory => "📁",
        NodeType::File => "📄",
        NodeType::Symlink => "🔗",
        NodeType::Other => "❓",
    };

    let has_children = node.node_type == NodeType::Directory && !node.children.is_empty() && depth < max_depth;

    if has_children {
        write!(html, "<details{}>\n", if depth == 0 { " open" } else { "" })?;
        write!(html, "<summary>")?;
        write!(html, "<span class=\"node\">")?;
        write!(html, "<span class=\"name {name_class}\">{icon} {}</span>", escape_html(&node.name))?;
        write!(html, "<span class=\"size\">{}</span>", human_readable_size(node.size))?;
        write!(html, "<span class=\"pct\">{pct:.1}%</span>")?;
        write!(html, "<span class=\"bar\"><span class=\"bar-fill\" style=\"width:{bar_width:.0}px\"></span></span>")?;
        write!(html, "</span>")?;
        write!(html, "</summary>\n")?;

        let mut children: Vec<&Node> = node.children.iter().collect();
        children.sort_by(|a, b| b.size.cmp(&a.size));

        for child in children {
            write_node_html(html, child, total_size, depth + 1, max_depth)?;
        }

        write!(html, "</details>\n")?;
    } else {
        write!(html, "<div style=\"margin-left:20px\">")?;
        write!(html, "<span class=\"node\">")?;
        write!(html, "<span class=\"name {name_class}\">{icon} {}</span>", escape_html(&node.name))?;
        write!(html, "<span class=\"size\">{}</span>", human_readable_size(node.size))?;
        write!(html, "<span class=\"pct\">{pct:.1}%</span>")?;
        write!(html, "<span class=\"bar\"><span class=\"bar-fill\" style=\"width:{bar_width:.0}px\"></span></span>")?;
        write!(html, "</span>")?;
        write!(html, "</div>\n")?;
    }

    Ok(())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
