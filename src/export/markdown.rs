use std::fmt::Write;
use std::path::Path;

use crate::models::node::{human_readable_size, Node, NodeType};
use crate::models::scan_result::ScanResult;

pub fn export_markdown(result: &ScanResult, output_path: &Path) -> anyhow::Result<()> {
    let mut md = String::new();

    writeln!(md, "# DiskLens Report")?;
    writeln!(md)?;
    writeln!(md, "- **Path:** {}", result.scan_path.display())?;
    writeln!(md, "- **Total Size:** {}", human_readable_size(result.total_size))?;
    writeln!(md, "- **Files:** {}", result.total_files)?;
    writeln!(md, "- **Directories:** {}", result.total_dirs)?;
    writeln!(md, "- **Scan Duration:** {:.2}s", result.scan_duration.as_secs_f64())?;
    writeln!(md)?;

    writeln!(md, "## Directory Tree")?;
    writeln!(md)?;
    writeln!(md, "| Name | Size | % |")?;
    writeln!(md, "|------|------|---|")?;

    write_node_markdown(&mut md, &result.root, result.total_size, 0, 3)?;

    if !result.errors.is_empty() {
        writeln!(md)?;
        writeln!(md, "## Errors ({} total)", result.errors.len())?;
        writeln!(md)?;
        for err in &result.errors {
            writeln!(md, "- **{:?}**: {}", err.error_type, err.path.display())?;
        }
    }

    std::fs::write(output_path, md)?;
    Ok(())
}

fn write_node_markdown(
    md: &mut String,
    node: &Node,
    total_size: u64,
    depth: usize,
    max_depth: usize,
) -> std::fmt::Result {
    if depth > max_depth {
        return Ok(());
    }

    let indent = "\u{00a0}\u{00a0}".repeat(depth);
    let icon = match node.node_type {
        NodeType::Directory => "📁 ",
        NodeType::File => "📄 ",
        NodeType::Symlink => "🔗 ",
        NodeType::Other => "❓ ",
    };
    let pct = node.percentage(total_size);

    writeln!(
        md,
        "| {}{}{} | {} | {:.1}% |",
        indent,
        icon,
        node.name,
        human_readable_size(node.size),
        pct,
    )?;

    if node.node_type == NodeType::Directory && depth < max_depth {
        let mut children: Vec<&Node> = node.children.iter().collect();
        children.sort_by(|a, b| b.size.cmp(&a.size));

        for child in children {
            write_node_markdown(md, child, total_size, depth + 1, max_depth)?;
        }
    }

    Ok(())
}
