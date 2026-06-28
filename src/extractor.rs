// extractor.rs — Use Tree-sitter to parse C# and pull out translatable units
// (classes, structs, enums, interfaces, and top-level methods) one node at a time.
//
// The orchestrator feeds these nodes to the LLM individually, rather than dumping
// a whole file/repo into the model. Tree-sitter only finds the *boundaries* — it
// does no translation. All meaning is handled downstream by the LLM.
//
// Ported from the former `tree-sitter-c-sharp-rust/extractor.py`; it now calls the
// tree-sitter C# grammar directly through the Rust bindings instead of Python.

use tree_sitter::{Language, Node, Parser};

/// The node kinds Tree-sitter uses for C# declarations we care about.
const TYPE_DECLS: &[&str] = &[
    "class_declaration",
    "struct_declaration",
    "enum_declaration",
    "interface_declaration",
    "record_declaration",
];

/// One translatable unit of C# source.
#[derive(Debug, Clone)]
pub struct Unit {
    /// e.g. "class_declaration"
    pub kind: String,
    /// the identifier, e.g. "Calculator"
    pub name: String,
    /// the exact C# text of this node
    pub source: String,
    /// byte offset (for ordering / debugging)
    pub start: usize,
}

pub struct CSharpExtractor {
    parser: Parser,
}

impl CSharpExtractor {
    pub fn new() -> Self {
        let language: Language = tree_sitter_c_sharp::LANGUAGE.into();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("loading the bundled C# grammar should not fail");
        Self { parser }
    }

    pub fn parse(&mut self, code: &str) -> Vec<Unit> {
        let tree = self
            .parser
            .parse(code, None)
            .expect("tree-sitter returned no tree for the C# source");
        let src = code.as_bytes();
        let mut units: Vec<Unit> = Vec::new();
        walk(tree.root_node(), src, &mut units);
        // Stable order: by position in the source file.
        units.sort_by_key(|u| u.start);
        units
    }
}

/// Depth-first walk. When we hit a type declaration, capture the whole node (it
/// carries its own methods/fields with it) and do NOT descend into it — we
/// translate a class as one unit. Top-level methods that aren't inside a class
/// are captured separately.
fn walk(node: Node, src: &[u8], out: &mut Vec<Unit>) {
    let kind = node.kind();

    if TYPE_DECLS.contains(&kind) {
        out.push(make_unit(node, src));
        return; // don't recurse; the class body travels with the class
    }

    if kind == "method_declaration" {
        // Only reached for methods not wrapped in a captured type.
        out.push(make_unit(node, src));
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, out);
    }
}

fn make_unit(node: Node, src: &[u8]) -> Unit {
    let mut name = String::from("<anonymous>");
    // The identifier child holds the declared name.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            name = String::from_utf8_lossy(&src[child.start_byte()..child.end_byte()]).into_owned();
            break;
        }
    }
    let source = String::from_utf8_lossy(&src[node.start_byte()..node.end_byte()]).into_owned();
    Unit {
        kind: node.kind().to_string(),
        name,
        source,
        start: node.start_byte(),
    }
}
