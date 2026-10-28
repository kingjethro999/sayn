pub mod ast;
pub mod discovery;
pub mod parser;
pub mod validator;

pub use ast::*;
pub use discovery::discover_sayn_files;
pub use parser::parse_sayn_source;
pub use validator::validate_document;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_sample_sayn() {
        let content = r#"
Create a responsive developer portfolio website.

My name is Jordan.
I build full-stack web applications.

Pages:
- Home page with an introduction.
- Projects page with cards.

@route /portfolio
@uses ./assets/profile.png

```typescript
console.log("literal code block preserved");
```
"#;
        let doc = parse_sayn_source(Path::new("app/page.sayn"), content);
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].title, "Pages");
        assert_eq!(doc.sections[0].blocks.len(), 5);
        assert_eq!(doc.all_directives().len(), 2);
    }

    #[test]
    fn test_validator_contradiction_detection() {
        let content = r#"
Build a Next.js application.
Never use JavaScript anywhere.
"#;
        let doc = parse_sayn_source(Path::new("app/test.sayn"), content);
        let diags = validate_document(&doc, Path::new("."));
        assert!(diags.iter().any(|d| d.code == "E101"));
    }
}
