use forgo_lib_yaml::{Doc, Node};

fn main() {
    let s = "---\n- !str foo\n";
    
    println!("Input: {:?}", s);
    
    match Doc::from_str(s) {
        Ok(doc) => {
            println!("✓ Parsed successfully!");
            
            // Check the AST
            match &doc.root().node {
                Node::Seq(items) => {
                    println!("Sequence with {} items", items.len());
                    for (i, item) in items.iter().enumerate() {
                        println!("Item {}: tag={:?}, node={:?}", i, item.meta.tag, item.node);
                    }
                }
                _ => println!("Root is not a sequence: {:?}", doc.root().node),
            }
            
            match doc.to_string() {
                Ok(out) => {
                    println!("\nOutput:");
                    println!("{}", out);
                }
                Err(e) => println!("Emit error: {:?}", e),
            }
        }
        Err(e) => println!("✗ Parse error: {:?}", e),
    }
}
